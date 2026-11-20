use crate::storage::keys::DataKey;
use crate::types::{BridgeRelayer, ChainId, ContractError, CrossChainProof};
use soroban_sdk::{Address, Env, String, Vec};

pub fn register_relayer(
    env: &Env,
    admin: &Address,
    relayer: Address,
    authorized_chains: Vec<ChainId>,
) -> Result<(), ContractError> {
    admin.require_auth();
    if crate::storage::helpers::Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let record = BridgeRelayer {
        address: relayer.clone(),
        is_active: true,
        registered_at: env.ledger().timestamp(),
        authorized_chains,
    };

    env.storage()
        .persistent()
        .set(&DataKey::BridgeRelayer(relayer), &record);
    Ok(())
}

pub fn deactivate_relayer(
    env: &Env,
    admin: &Address,
    relayer: Address,
) -> Result<(), ContractError> {
    admin.require_auth();
    if crate::storage::helpers::Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let mut record = get_relayer(env, &relayer).ok_or(ContractError::InvalidState)?;
    record.is_active = false;

    env.storage()
        .persistent()
        .set(&DataKey::BridgeRelayer(relayer), &record);
    Ok(())
}

pub fn submit_proof(
    env: &Env,
    relayer: Address,
    grant_id: u64,
    milestone_idx: u32,
    chain_id: ChainId,
    tx_hash: String,
) -> Result<(), ContractError> {
    relayer.require_auth();
    let record = get_relayer(env, &relayer).ok_or(ContractError::Unauthorized)?;

    if !record.is_active {
        return Err(ContractError::Unauthorized);
    }

    if !record.authorized_chains.contains(chain_id.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let proof = CrossChainProof {
        chain_id,
        tx_hash,
        relayer: relayer.clone(),
        verified_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&DataKey::CrossChainProof(grant_id, milestone_idx), &proof);
    Ok(())
}

pub fn get_proof(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<CrossChainProof> {
    env.storage()
        .persistent()
        .get(&DataKey::CrossChainProof(grant_id, milestone_idx))
}

pub fn has_valid_proof(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    if let Some(proof) = get_proof(env, grant_id, milestone_idx) {
        if let Some(relayer) = get_relayer(env, &proof.relayer) {
            return relayer.is_active && relayer.authorized_chains.contains(proof.chain_id);
        }
    }
    false
}

pub fn get_relayer(env: &Env, relayer: &Address) -> Option<BridgeRelayer> {
    env.storage()
        .persistent()
        .get(&DataKey::BridgeRelayer(relayer.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    fn chains(env: &Env, ids: &[ChainId]) -> Vec<ChainId> {
        let mut v = Vec::new(env);
        for id in ids {
            v.push_back(id.clone());
        }
        v
    }

    fn setup(env: &Env) -> (Address, Address) {
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let admin = Address::generate(env);
        env.as_contract(&contract_id, || Storage::set_global_admin(env, &admin));
        (contract_id, admin)
    }

    // Each auth'd call runs in its own contract frame: `mock_all_auths` rejects
    // a second `require_auth` for the same address inside one frame.
    fn register(
        env: &Env,
        cid: &Address,
        admin: &Address,
        relayer: &Address,
        authorized: Vec<ChainId>,
    ) -> Result<(), ContractError> {
        env.as_contract(cid, || {
            register_relayer(env, admin, relayer.clone(), authorized)
        })
    }

    fn deactivate(
        env: &Env,
        cid: &Address,
        admin: &Address,
        relayer: &Address,
    ) -> Result<(), ContractError> {
        env.as_contract(cid, || deactivate_relayer(env, admin, relayer.clone()))
    }

    fn submit(
        env: &Env,
        cid: &Address,
        relayer: &Address,
        grant_id: u64,
        milestone_idx: u32,
        chain_id: ChainId,
        tx_hash: String,
    ) -> Result<(), ContractError> {
        env.as_contract(cid, || {
            submit_proof(
                env,
                relayer.clone(),
                grant_id,
                milestone_idx,
                chain_id,
                tx_hash,
            )
        })
    }

    #[test]
    fn register_relayer_requires_global_admin() {
        let env = Env::default();
        let (cid, _admin) = setup(&env);
        let stranger = Address::generate(&env);
        let relayer = Address::generate(&env);
        assert_eq!(
            register(
                &env,
                &cid,
                &stranger,
                &relayer,
                chains(&env, &[ChainId::Ethereum])
            ),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn register_relayer_stores_active_record() {
        let env = Env::default();
        let (cid, admin) = setup(&env);
        let relayer = Address::generate(&env);
        register(
            &env,
            &cid,
            &admin,
            &relayer,
            chains(&env, &[ChainId::Ethereum]),
        )
        .unwrap();

        env.as_contract(&cid, || {
            let record = get_relayer(&env, &relayer).unwrap();
            assert!(record.is_active);
            assert!(record.authorized_chains.contains(ChainId::Ethereum));
        });
    }

    #[test]
    fn submit_proof_rejects_unregistered_relayer() {
        let env = Env::default();
        let (cid, _admin) = setup(&env);
        let relayer = Address::generate(&env);
        assert_eq!(
            submit(
                &env,
                &cid,
                &relayer,
                1,
                0,
                ChainId::Ethereum,
                String::from_str(&env, "0xabc")
            ),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn submit_proof_rejects_unauthorized_chain() {
        let env = Env::default();
        let (cid, admin) = setup(&env);
        let relayer = Address::generate(&env);
        register(
            &env,
            &cid,
            &admin,
            &relayer,
            chains(&env, &[ChainId::Ethereum]),
        )
        .unwrap();

        assert_eq!(
            submit(
                &env,
                &cid,
                &relayer,
                1,
                0,
                ChainId::Polygon,
                String::from_str(&env, "0xabc")
            ),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn submit_proof_rejects_deactivated_relayer() {
        let env = Env::default();
        let (cid, admin) = setup(&env);
        let relayer = Address::generate(&env);
        register(
            &env,
            &cid,
            &admin,
            &relayer,
            chains(&env, &[ChainId::Ethereum]),
        )
        .unwrap();
        deactivate(&env, &cid, &admin, &relayer).unwrap();

        assert_eq!(
            submit(
                &env,
                &cid,
                &relayer,
                1,
                0,
                ChainId::Ethereum,
                String::from_str(&env, "0xabc")
            ),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn deactivate_unregistered_relayer_errors() {
        let env = Env::default();
        let (cid, admin) = setup(&env);
        let relayer = Address::generate(&env);
        assert_eq!(
            deactivate(&env, &cid, &admin, &relayer),
            Err(ContractError::InvalidState)
        );
    }

    #[test]
    fn proof_lifecycle_and_milestone_submit_substitution() {
        let env = Env::default();
        let (cid, admin) = setup(&env);
        let relayer = Address::generate(&env);
        let tx_hash = String::from_str(&env, "0xdeadbeef");

        register(
            &env,
            &cid,
            &admin,
            &relayer,
            chains(&env, &[ChainId::Ethereum]),
        )
        .unwrap();
        submit(
            &env,
            &cid,
            &relayer,
            7,
            2,
            ChainId::Ethereum,
            tx_hash.clone(),
        )
        .unwrap();

        // This is exactly what `apply_milestone_submission` consults: a valid
        // proof means the caller-provided proof_url is replaced by the
        // relayer's cross-chain tx hash.
        env.as_contract(&cid, || {
            assert!(has_valid_proof(&env, 7, 2));
            assert_eq!(get_proof(&env, 7, 2).unwrap().tx_hash, tx_hash);
        });

        // Once the relayer is deactivated its stale proof must no longer
        // satisfy has_valid_proof, so milestone_submit falls back to the
        // caller-provided proof_url instead of the cross-chain hash.
        deactivate(&env, &cid, &admin, &relayer).unwrap();
        env.as_contract(&cid, || {
            assert!(!has_valid_proof(&env, 7, 2));
            // The proof record itself is still stored, just no longer trusted.
            assert!(get_proof(&env, 7, 2).is_some());
        });
    }
}
