use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ParamRecord, ParamValue};
use soroban_sdk::{Address, Env, Symbol, Vec};

const MAX_HISTORY_SIZE: u32 = 20;

/// Set a parameter directly. Admin only for non-DAO params; DAO-flagged
/// parameters are rejected until real DAO-proposal verification is wired up.
pub fn set_param(
    env: &Env,
    caller: &Address,
    key: Symbol,
    value: ParamValue,
    description: soroban_sdk::String,
    requires_dao: bool,
) -> Result<(), ContractError> {
    caller.require_auth();

    if requires_dao {
        return Err(ContractError::DaoVoteRequired);
    }

    if Storage::get_global_admin(env) != Some(caller.clone()) {
        return Err(ContractError::Unauthorized);
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

    crate::events::Events::emit_param_changed(env, key.clone(), caller.clone());

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ParamType;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, String, Symbol};

    fn make_value(val: u32) -> ParamValue {
        ParamValue {
            param_type: ParamType::U32,
            u32_val: Some(val),
            i128_val: None,
            bool_val: None,
            address_val: None,
            string_val: None,
        }
    }

    #[test]
    fn test_set_param_non_dao_by_admin_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let key = Symbol::new(&env, "max_amount");
        let desc = String::from_str(&env, "Maximum grant amount");
        set_param(&env, &admin, key.clone(), make_value(100), desc, false).unwrap();

        let record = get_param(&env, &key).unwrap();
        assert_eq!(record.value.u32_val, Some(100));
        assert!(!record.requires_dao_vote);
    }

    #[test]
    fn test_set_param_non_dao_by_non_admin_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let stranger = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let key = Symbol::new(&env, "max_amount");
        let desc = String::from_str(&env, "desc");
        let result = set_param(&env, &stranger, key, make_value(100), desc, false);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_set_param_dao_flagged_rejected_even_for_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let key = Symbol::new(&env, "quorum_bps");
        let desc = String::from_str(&env, "Quorum threshold");
        let result = set_param(&env, &admin, key, make_value(5001), desc, true);
        assert_eq!(result, Err(ContractError::DaoVoteRequired));
    }

    #[test]
    fn test_dao_and_non_dao_paths_differ() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let key = Symbol::new(&env, "fee_bps");
        let desc = String::from_str(&env, "Protocol fee");

        // Non-DAO path: succeeds
        let result_ok = set_param(
            &env,
            &admin,
            key.clone(),
            make_value(100),
            desc.clone(),
            false,
        );
        assert!(result_ok.is_ok());

        // DAO path: rejected
        let result_err = set_param(&env, &admin, key, make_value(200), desc, true);
        assert_eq!(result_err, Err(ContractError::DaoVoteRequired));
    }
}
