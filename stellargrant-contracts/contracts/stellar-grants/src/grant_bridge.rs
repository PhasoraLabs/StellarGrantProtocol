use soroban_sdk::{Address, Env, String, Vec};
use crate::types::{ChainId, BridgeRelayer, CrossChainProof, ContractError};
use crate::storage::keys::DataKey;

pub fn register_relayer(env: &Env, admin: &Address, relayer: Address, authorized_chains: Vec<ChainId>) -> Result<(), ContractError> {
    admin.require_auth();
    if crate::storage::helpers::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    
    let record = BridgeRelayer {
        address: relayer.clone(),
        is_active: true,
        registered_at: env.ledger().timestamp(),
        authorized_chains,
    };
    
    env.storage().persistent().set(&DataKey::BridgeRelayer(relayer), &record);
    Ok(())
}

pub fn deactivate_relayer(env: &Env, admin: &Address, relayer: Address) -> Result<(), ContractError> {
    admin.require_auth();
    if crate::storage::helpers::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    
    let mut record = get_relayer(env, &relayer).ok_or(ContractError::InvalidState)?; 
    record.is_active = false;
    
    env.storage().persistent().set(&DataKey::BridgeRelayer(relayer), &record);
    Ok(())
}

pub fn submit_proof(env: &Env, relayer: Address, grant_id: u64, milestone_idx: u32, chain_id: ChainId, tx_hash: String) -> Result<(), ContractError> {
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
    
    env.storage().persistent().set(&DataKey::CrossChainProof(grant_id, milestone_idx), &proof);
    Ok(())
}

pub fn get_proof(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<CrossChainProof> {
    env.storage().persistent().get(&DataKey::CrossChainProof(grant_id, milestone_idx))
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
    env.storage().persistent().get(&DataKey::BridgeRelayer(relayer.clone()))
}
