use soroban_sdk::{Address, Env, String, Symbol};
use crate::types::{GrantPauseRecord, ContractError};
use crate::storage::keys::DataKey;
use crate::storage::helpers;

pub fn pause(env: &Env, caller: &Address, grant_id: u64, reason: String, auto_unpause_at: Option<u64>) -> Result<(), ContractError> {
    caller.require_auth();
    let grant = helpers::get_grant(env, grant_id).ok_or(ContractError::NotFound)?;
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    
    let key = DataKey::GrantPaused(grant_id);
    let mut record = get_record(env, grant_id).unwrap_or_else(|| GrantPauseRecord {
        grant_id,
        paused_by: caller.clone(),
        paused_at: env.ledger().timestamp(),
        reason: reason.clone(),
        auto_unpause_at,
        unpause_history: soroban_sdk::Vec::new(env),
    });
    
    record.paused_by = caller.clone();
    record.paused_at = env.ledger().timestamp();
    record.reason = reason;
    record.auto_unpause_at = auto_unpause_at;
    
    // Clear auto_unpause_at if it was previously set to a past time to unpause
    if let Some(time) = auto_unpause_at {
        if time <= env.ledger().timestamp() {
             record.auto_unpause_at = None;
        }
    }
    
    env.storage().persistent().set(&key, &record);
    env.events().publish((Symbol::new(env, "grant_paused"), grant_id), caller.clone());
    Ok(())
}

pub fn unpause(env: &Env, caller: &Address, grant_id: u64) -> Result<(), ContractError> {
    caller.require_auth();
    let grant = helpers::get_grant(env, grant_id).ok_or(ContractError::NotFound)?;
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    
    let key = DataKey::GrantPaused(grant_id);
    if let Some(mut record) = get_record(env, grant_id) {
        let now = env.ledger().timestamp();
        record.unpause_history.push_back((caller.clone(), now));
        record.auto_unpause_at = Some(now); // Setting this to now unpauses it
        env.storage().persistent().set(&key, &record);
    }
    env.events().publish((Symbol::new(env, "grant_unpaused"), grant_id), caller.clone());
    Ok(())
}

pub fn require_not_paused(env: &Env, grant_id: u64) -> Result<(), ContractError> {
    if is_paused(env, grant_id) {
        return Err(ContractError::ContractPaused);
    }
    Ok(())
}

pub fn is_paused(env: &Env, grant_id: u64) -> bool {
    if let Some(record) = get_record(env, grant_id) {
        if let Some(auto_unpause) = record.auto_unpause_at {
            if env.ledger().timestamp() >= auto_unpause {
                return false;
            }
        }
        return true;
    }
    false
}

pub fn get_record(env: &Env, grant_id: u64) -> Option<GrantPauseRecord> {
    env.storage().persistent().get(&DataKey::GrantPaused(grant_id))
}

pub fn try_auto_unpause(env: &Env, grant_id: u64) -> bool {
    if let Some(mut record) = get_record(env, grant_id) {
        if let Some(auto_unpause) = record.auto_unpause_at {
            let now = env.ledger().timestamp();
            if now >= auto_unpause {
                // Add to history
                // We use the contract address as the unpauser if it's auto
                let contract_address = env.current_contract_address();
                record.unpause_history.push_back((contract_address, now));
                // It's already unpaused logically by time, but we just save the history update
                env.storage().persistent().set(&DataKey::GrantPaused(grant_id), &record);
                env.events().publish((Symbol::new(env, "grant_unpaused"), grant_id), auto_unpause);
                return true;
            }
        }
    }
    false
}
