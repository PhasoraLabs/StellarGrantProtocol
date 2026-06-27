use soroban_sdk::{Address, Bytes, Env};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{GrantStatus, MerkleCommitment, MerkleProof};

const ROOT_LEN: u32 = 32;

/// Commit a Merkle root for a milestone's evidence set.
/// Contributor must be the grant's assigned contributor (grant owner).
pub fn commit_evidence_root(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    milestone_idx: u32,
    root: Bytes,
    leaf_count: u32,
) -> Result<(), ContractError> {
    contributor.require_auth();

    if root.len() != ROOT_LEN || leaf_count == 0 {
        return Err(ContractError::InvalidInput);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }
    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    if Storage::get_merkle_commitment(env, grant_id, milestone_idx).is_some() {
        return Err(ContractError::InvalidState);
    }

    let commitment = MerkleCommitment {
        grant_id,
        milestone_idx,
        root,
        leaf_count,
        committed_by: contributor.clone(),
        committed_at: env.ledger().timestamp(),
    };
    Storage::set_merkle_commitment(env, grant_id, milestone_idx, &commitment);
    Ok(())
}

/// Verify that `proof.leaf` is included in the committed root.
pub fn verify_proof(env: &Env, grant_id: u64, milestone_idx: u32, proof: &MerkleProof) -> bool {
    let Some(commitment) = Storage::get_merkle_commitment(env, grant_id, milestone_idx) else {
        return false;
    };

    let mut hash = hash_leaf(env, &proof.leaf);
    let mut idx = proof.leaf_index;

    for i in 0..proof.siblings.len() {
        let sibling = proof.siblings.get(i).unwrap_or_else(|| Bytes::new(env));
        if idx & 1 == 0 {
            hash = hash_pair(env, &hash, &sibling);
        } else {
            hash = hash_pair(env, &sibling, &hash);
        }
        idx >>= 1;
    }

    hash == commitment.root
}

/// Hash two child nodes together: SHA-256(left || right).
pub fn hash_pair(env: &Env, left: &Bytes, right: &Bytes) -> Bytes {
    let mut data = Bytes::new(env);
    data.append(left);
    data.append(right);
    env.crypto().sha256(&data).into()
}

/// Hash a leaf value: SHA-256(leaf_data).
pub fn hash_leaf(env: &Env, data: &Bytes) -> Bytes {
    env.crypto().sha256(data).into()
}

/// Return the committed root for a milestone, if any.
pub fn get_commitment(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<MerkleCommitment> {
    Storage::get_merkle_commitment(env, grant_id, milestone_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Bytes, Env, Vec};

    fn leaf_a(env: &Env) -> Bytes {
        Bytes::from_array(env, b"leaf-a")
    }

    fn leaf_b(env: &Env) -> Bytes {
        Bytes::from_array(env, b"leaf-b")
    }

    fn leaf_c(env: &Env) -> Bytes {
        Bytes::from_array(env, b"leaf-c")
    }

    fn leaf_d(env: &Env) -> Bytes {
        Bytes::from_array(env, b"leaf-d")
    }

    fn build_four_leaf_root(env: &Env) -> (Bytes, Vec<Bytes>) {
        let l0 = leaf_a(env);
        let l1 = leaf_b(env);
        let l2 = leaf_c(env);
        let l3 = leaf_d(env);

        let h0 = hash_leaf(env, &l0);
        let h1 = hash_leaf(env, &l1);
        let h2 = hash_leaf(env, &l2);
        let h3 = hash_leaf(env, &l3);

        let h01 = hash_pair(env, &h0, &h1);
        let h23 = hash_pair(env, &h2, &h3);
        let root = hash_pair(env, &h01, &h23);

        let mut leaves = Vec::new(env);
        leaves.push_back(l0);
        leaves.push_back(l1);
        leaves.push_back(l2);
        leaves.push_back(l3);
        (root, leaves)
    }

    #[test]
    fn test_hash_pair_is_order_sensitive() {
        let env = Env::default();
        let a = Bytes::from_array(&env, b"a");
        let b = Bytes::from_array(&env, b"b");
        assert_ne!(hash_pair(&env, &a, &b), hash_pair(&env, &b, &a));
    }

    #[test]
    fn test_tampered_leaf_fails_verification() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let grant_id = 1u64;
        let milestone_idx = 0u32;

        let (root, leaves) = build_four_leaf_root(&env);

        env.as_contract(&contract_id, || {
            let commitment = MerkleCommitment {
                grant_id,
                milestone_idx,
                root: root.clone(),
                leaf_count: 4,
                committed_by: owner.clone(),
                committed_at: 0,
            };
            Storage::set_merkle_commitment(&env, grant_id, milestone_idx, &commitment);

            let h0 = hash_leaf(&env, &leaves.get(0).unwrap());
            let h1 = hash_leaf(&env, &leaves.get(1).unwrap());
            let h2 = hash_leaf(&env, &leaves.get(2).unwrap());
            let h3 = hash_leaf(&env, &leaves.get(3).unwrap());
            let h23 = hash_pair(&env, &h2, &h3);

            let mut siblings = Vec::new(&env);
            siblings.push_back(h1);
            siblings.push_back(h23);

            let valid = MerkleProof {
                leaf: leaves.get(0).unwrap(),
                leaf_index: 0,
                siblings: siblings.clone(),
            };
            assert!(verify_proof(&env, grant_id, milestone_idx, &valid));

            let tampered = MerkleProof {
                leaf: Bytes::from_array(&env, b"leaf-a-tampered"),
                leaf_index: 0,
                siblings,
            };
            assert!(!verify_proof(&env, grant_id, milestone_idx, &tampered));

            let _ = h0;
            let _ = h3;
        });
    }
}
