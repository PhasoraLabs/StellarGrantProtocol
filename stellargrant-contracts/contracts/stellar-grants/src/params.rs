use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ParamRecord, ParamValue};
use soroban_sdk::{Address, Env, Symbol, Vec};

const MAX_HISTORY_SIZE: u32 = 20;

/// Set a parameter directly. Admin only for non-DAO params; DAO vote required for others.
pub fn set_param(
    env: &Env,
    caller: &Address,
    key: Symbol,
    value: ParamValue,
    description: soroban_sdk::String,
    requires_dao: bool,
) -> Result<(), ContractError> {
    caller.require_auth();

    // Check authorization
    if requires_dao {
        // In a real implementation, this would check DAO proposal approval
        // For now, we check admin status
        if Storage::get_global_admin(env) != Some(caller.clone()) {
            return Err(ContractError::Unauthorized);
        }
    } else {
        // Non-DAO params require admin
        if Storage::get_global_admin(env) != Some(caller.clone()) {
            return Err(ContractError::Unauthorized);
        }
    }

    // Save old value to history if exists
    if let Some(old_record) = Storage::get_param(env, &key) {
        let mut history = Storage::get_param_history(env, &key);

        // Evict oldest if at max size
        if history.len() >= MAX_HISTORY_SIZE {
            history.remove(0);
        }

        history.push_back(old_record);
        Storage::set_param_history(env, &key, &history);
    }

    let record = ParamRecord {
        key: key.clone(),
        value,
        set_by: caller.clone(),
        set_at: env.ledger().timestamp(),
        description,
        requires_dao_vote: requires_dao,
    };

    Storage::set_param(env, &key, &record);

    Ok(())
}

/// Get a parameter value by key.
pub fn get_param(env: &Env, key: &Symbol) -> Option<ParamRecord> {
    Storage::get_param(env, key)
}

/// Get a u32 param or return a default value.
pub fn get_u32(env: &Env, key: &Symbol, default: u32) -> u32 {
    if let Some(record) = get_param(env, key) {
        if let Some(val) = record.value.u32_val {
            return val;
        }
    }
    default
}

/// Get an i128 param or return a default value.
pub fn get_i128(env: &Env, key: &Symbol, default: i128) -> i128 {
    if let Some(record) = get_param(env, key) {
        if let Some(val) = record.value.i128_val {
            return val;
        }
    }
    default
}

/// Get a bool param or return a default value.
pub fn get_bool(env: &Env, key: &Symbol, default: bool) -> bool {
    if let Some(record) = get_param(env, key) {
        if let Some(val) = record.value.bool_val {
            return val;
        }
    }
    default
}

/// Return all registered param keys.
pub fn list_params(env: &Env) -> Vec<Symbol> {
    Storage::list_param_keys(env)
}

/// Return the change history for a param (last 20 changes).
pub fn param_history(env: &Env, key: &Symbol) -> Vec<ParamRecord> {
    Storage::get_param_history(env, key)
}
