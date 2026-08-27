/// Milestone NFT certificate module (issue #570).
/// Mints a unique, soulbound (non-transferable by default) NFT for each approved milestone,
/// permanently linking the contributor's address to their on-chain work history.
use soroban_sdk::{xdr::ToXdr, Address, Bytes, Env, Vec};

use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{MilestoneNft, NftMetadata};

/// Mint a milestone NFT for a contributor. Called by governance when a milestone is approved.
/// Returns the new global `token_id`.
pub fn mint(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    owner: &Address,
    metadata: NftMetadata,
) -> Result<u32, ContractError> {
    let token_id = Storage::next_nft_id(env);
    let minted_at = env.ledger().timestamp();
    let minted_at_ledger = env.ledger().sequence();

    let proof = compute_proof_hash(env, grant_id, milestone_idx, owner, minted_at);

    let nft = MilestoneNft {
        token_id,
        grant_id,
        milestone_idx,
        owner: owner.clone(),
        minted_at,
        minted_at_ledger,
        metadata,
        is_transferable: false,
        proof_hash: proof,
    };

    Storage::set_milestone_nft(env, &nft);
    Storage::set_nft_token_index(env, token_id, grant_id, milestone_idx);

    let mut owned = Storage::get_nfts_by_owner(env, owner);
    owned.push_back(token_id);
    Storage::set_nfts_by_owner(env, owner, &owned);

    Events::emit_nft_minted(env, token_id, grant_id, milestone_idx, owner.clone());
    Ok(token_id)
}

/// Return the NFT for a specific (grant_id, milestone_idx), if minted.
pub fn get_nft(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<MilestoneNft> {
    Storage::get_milestone_nft(env, grant_id, milestone_idx)
}

/// Return all NFT token IDs owned by a contributor.
pub fn get_by_owner(env: &Env, owner: &Address) -> Vec<u32> {
    Storage::get_nfts_by_owner(env, owner)
}

/// Return an NFT by its global token ID.
pub fn get_by_token_id(env: &Env, token_id: u32) -> Option<MilestoneNft> {
    Storage::get_nft_by_token_id(env, token_id)
}

/// Verify the proof hash of an NFT. Returns false if the NFT is not found or the hash
/// does not match a freshly computed value from its stored fields.
pub fn verify_nft(env: &Env, token_id: u32) -> bool {
    let nft = match Storage::get_nft_by_token_id(env, token_id) {
        Some(n) => n,
        None => return false,
    };
    let expected = compute_proof_hash(
        env,
        nft.grant_id,
        nft.milestone_idx,
        &nft.owner,
        nft.minted_at,
    );
    expected == nft.proof_hash
}

/// Admin can mark an NFT as transferable (unlocks transfer for that specific token).
pub fn set_transferable(
    env: &Env,
    admin: &Address,
    token_id: u32,
    transferable: bool,
) -> Result<(), ContractError> {
    admin.require_auth();
    let global_admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if *admin != global_admin {
        return Err(ContractError::Unauthorized);
    }

    let mut nft = Storage::get_nft_by_token_id(env, token_id).ok_or(ContractError::NftNotFound)?;
    nft.is_transferable = transferable;
    Storage::set_milestone_nft(env, &nft);
    Ok(())
}

/// Transfer an NFT to a new owner. Only permitted when `is_transferable == true`.
pub fn transfer(
    env: &Env,
    from: &Address,
    to: &Address,
    token_id: u32,
) -> Result<(), ContractError> {
    from.require_auth();

    let mut nft = Storage::get_nft_by_token_id(env, token_id).ok_or(ContractError::NftNotFound)?;
    if nft.owner != *from {
        return Err(ContractError::NotNftOwner);
    }
    if !nft.is_transferable {
        return Err(ContractError::NftNotTransferable);
    }

    // Remove token_id from sender's list
    let mut from_owned = Storage::get_nfts_by_owner(env, from);
    let mut new_from: Vec<u32> = Vec::new(env);
    for i in 0..from_owned.len() {
        let id = from_owned.get(i).unwrap();
        if id != token_id {
            new_from.push_back(id);
        }
    }
    Storage::set_nfts_by_owner(env, from, &new_from);

    // Add token_id to recipient's list
    let mut to_owned = Storage::get_nfts_by_owner(env, to);
    to_owned.push_back(token_id);
    Storage::set_nfts_by_owner(env, to, &to_owned);

    nft.owner = to.clone();
    Storage::set_milestone_nft(env, &nft);

    Events::emit_nft_transferred(env, token_id, from.clone(), to.clone());
    Ok(())
}

/// Compute `SHA-256(grant_id_be || milestone_idx_be || minted_at_be || owner_xdr)`.
/// Binds the grant ID, milestone index, timestamp, and owner address to permanently link
/// the contributor's address to their on-chain milestone proof.
fn compute_proof_hash(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    owner: &Address,
    minted_at: u64,
) -> Bytes {
    let mut input = Bytes::new(env);
    input.extend_from_array(&grant_id.to_be_bytes());
    input.extend_from_array(&milestone_idx.to_be_bytes());
    input.extend_from_array(&minted_at.to_be_bytes());
    input.append(&owner.to_xdr(env));
    env.crypto().sha256(&input).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String, Vec};

    fn sample_metadata(env: &Env) -> NftMetadata {
        NftMetadata {
            name: String::from_str(env, "Milestone #1"),
            description: String::from_str(env, "First milestone completed"),
            grant_title: String::from_str(env, "My Grant"),
            image_uri: String::from_str(env, "ipfs://Qm..."),
            attributes: Vec::new(env),
        }
    }

    #[test]
    fn mint_returns_token_id_and_nft_stored() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);

        let token_id = mint(&env, 1, 0, &owner, sample_metadata(&env)).unwrap();
        assert_eq!(token_id, 1);

        let nft = get_nft(&env, 1, 0).unwrap();
        assert_eq!(nft.owner, owner);
        assert_eq!(nft.grant_id, 1);
        assert_eq!(nft.milestone_idx, 0);
        assert!(!nft.is_transferable);
    }

    #[test]
    fn get_by_owner_returns_correct_list() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);

        mint(&env, 1, 0, &owner, sample_metadata(&env)).unwrap();
        mint(&env, 1, 1, &owner, sample_metadata(&env)).unwrap();

        let owned = get_by_owner(&env, &owner);
        assert_eq!(owned.len(), 2);
    }

    #[test]
    fn transfer_soulbound_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_id = mint(&env, 1, 0, &owner, sample_metadata(&env)).unwrap();
        let result = transfer(&env, &owner, &recipient, token_id);
        assert_eq!(result, Err(ContractError::NftNotTransferable));
    }

    #[test]
    fn transfer_after_unlock_succeeds() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let owner = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_id = mint(&env, 1, 0, &owner, sample_metadata(&env)).unwrap();
        set_transferable(&env, &admin, token_id, true).unwrap();

        transfer(&env, &owner, &recipient, token_id).unwrap();

        let nft = get_nft(&env, 1, 0).unwrap();
        assert_eq!(nft.owner, recipient);
        assert_eq!(get_by_owner(&env, &owner).len(), 0);
        assert_eq!(get_by_owner(&env, &recipient).len(), 1);
    }

    #[test]
    fn verify_nft_detects_tampering() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let owner = Address::generate(&env);
        let token_id = mint(&env, 1, 0, &owner, sample_metadata(&env)).unwrap();
        assert!(verify_nft(&env, token_id));

        // Tamper: overwrite proof_hash with zeroes
        let mut nft = get_nft(&env, 1, 0).unwrap();
        nft.proof_hash = Bytes::new(&env);
        Storage::set_milestone_nft(&env, &nft);

        assert!(!verify_nft(&env, token_id));
    }

    #[test]
    fn proof_hash_depends_on_owner() {
        let env = Env::default();
        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);

        let hash1 = compute_proof_hash(&env, 1, 0, &owner1, 100);
        let hash2 = compute_proof_hash(&env, 1, 0, &owner2, 100);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn verify_nft_detects_owner_tampering() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let token_id = mint(&env, 1, 0, &owner, sample_metadata(&env)).unwrap();
        assert!(verify_nft(&env, token_id));

        // Tamper: change stored owner address on the NFT
        let mut nft = get_nft(&env, 1, 0).unwrap();
        nft.owner = attacker;
        Storage::set_milestone_nft(&env, &nft);

        assert!(!verify_nft(&env, token_id));
    }
}
