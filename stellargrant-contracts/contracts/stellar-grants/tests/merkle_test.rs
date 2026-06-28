use soroban_sdk::{testutils::Address as _, Bytes, Env, Vec};
use stellar_grants::{merkle, MerkleCommitment, MerkleProof, StellarGrantsContract, Storage};

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

/// Known-vector test: 4-leaf Merkle tree with hardcoded leaf values and expected root.
#[test]
fn test_four_leaf_known_vector_root_and_proof() {
    let env = Env::default();
    let contract_id = env.register(StellarGrantsContract, ());

    let l0 = leaf_a(&env);
    let l1 = leaf_b(&env);
    let l2 = leaf_c(&env);
    let l3 = leaf_d(&env);

    let h0 = merkle::hash_leaf(&env, &l0);
    let h1 = merkle::hash_leaf(&env, &l1);
    let h2 = merkle::hash_leaf(&env, &l2);
    let h3 = merkle::hash_leaf(&env, &l3);

    let h01 = merkle::hash_pair(&env, &h0, &h1);
    let h23 = merkle::hash_pair(&env, &h2, &h3);
    let expected_root = merkle::hash_pair(&env, &h01, &h23);

    let owner = soroban_sdk::Address::generate(&env);
    let grant_id = 42u64;
    let milestone_idx = 1u32;

    env.as_contract(&contract_id, || {
        Storage::set_merkle_commitment(
            &env,
            grant_id,
            milestone_idx,
            &MerkleCommitment {
                grant_id,
                milestone_idx,
                root: expected_root.clone(),
                leaf_count: 4,
                committed_by: owner,
                committed_at: 0,
            },
        );

        let mut siblings = Vec::new(&env);
        siblings.push_back(h1);
        siblings.push_back(h23);

        let proof = MerkleProof {
            leaf: l0,
            leaf_index: 0,
            siblings,
        };

        assert!(merkle::verify_proof(&env, grant_id, milestone_idx, &proof));
    });
}

#[test]
fn test_tampered_leaf_rejected() {
    let env = Env::default();
    let contract_id = env.register(StellarGrantsContract, ());

    let l0 = leaf_a(&env);
    let l1 = leaf_b(&env);
    let l2 = leaf_c(&env);
    let l3 = leaf_d(&env);

    let h0 = merkle::hash_leaf(&env, &l0);
    let h1 = merkle::hash_leaf(&env, &l1);
    let h2 = merkle::hash_leaf(&env, &l2);
    let h3 = merkle::hash_leaf(&env, &l3);
    let root = merkle::hash_pair(
        &env,
        &merkle::hash_pair(&env, &h0, &h1),
        &merkle::hash_pair(&env, &h2, &h3),
    );

    let owner = soroban_sdk::Address::generate(&env);
    let grant_id = 7u64;

    env.as_contract(&contract_id, || {
        Storage::set_merkle_commitment(
            &env,
            grant_id,
            0,
            &MerkleCommitment {
                grant_id,
                milestone_idx: 0,
                root,
                leaf_count: 4,
                committed_by: owner,
                committed_at: 0,
            },
        );

        let mut siblings = Vec::new(&env);
        siblings.push_back(h1);
        siblings.push_back(merkle::hash_pair(&env, &h2, &h3));

        let tampered = MerkleProof {
            leaf: Bytes::from_array(&env, b"leaf-a-evil"),
            leaf_index: 0,
            siblings,
        };

        assert!(!merkle::verify_proof(&env, grant_id, 0, &tampered));
    });
}
