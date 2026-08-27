use soroban_sdk::{Address, Env, String, Vec};

use crate::circuit_breaker;
use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{PauseRecord, ProtocolModule};

fn require_global_admin(env: &Env, admin: &Address) -> Result<(), ContractError> {
    let global_admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if global_admin != *admin {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

/// Returns true if the contract is currently paused.
pub fn is_paused(env: &Env) -> bool {
    Storage::get_is_paused(env)
}

/// Pause the contract. Admin only. Stores a PauseRecord and trips all breakers.
pub fn pause(env: &Env, admin: &Address, reason: String) -> Result<(), ContractError> {
    admin.require_auth();
    require_global_admin(env, admin)?;

    if is_paused(env) {
        return Err(ContractError::InvalidState);
    }

    Storage::set_is_paused(env, true);

    let all_modules = [
        ProtocolModule::Grants,
        ProtocolModule::Streaming,
        ProtocolModule::Bounty,
        ProtocolModule::Dao,
        ProtocolModule::Staking,
        ProtocolModule::Vesting,
        ProtocolModule::MatchingPool,
        ProtocolModule::Crowdfund,
        ProtocolModule::Insurance,
        ProtocolModule::Relay,
        ProtocolModule::TokenSwap,
        ProtocolModule::Oracle,
    ];
    for m in all_modules.iter() {
        let _ = circuit_breaker::trip(env, admin, m.clone(), reason.clone(), None);
    }

    let record = PauseRecord {
        paused_by: admin.clone(),
        paused_at: env.ledger().timestamp(),
        unpaused_at: None,
        reason: reason.clone(),
    };
    Storage::append_pause_record(env, &record);
    Events::emit_contract_paused(env, admin.clone(), reason);

    Ok(())
}

/// Unpause the contract. Admin only. Updates the latest PauseRecord with unpaused_at.
pub fn unpause(env: &Env, admin: &Address) -> Result<(), ContractError> {
    admin.require_auth();
    require_global_admin(env, admin)?;

    if !is_paused(env) {
        return Err(ContractError::InvalidState);
    }

    Storage::set_is_paused(env, false);
    Storage::set_latest_pause_unpaused_at(env, env.ledger().timestamp());
    Events::emit_contract_unpaused(env, admin.clone());

    Ok(())
}

/// Guard function: returns Err(ContractPaused) if paused. Call at the top of entry points.
pub fn require_not_paused(env: &Env) -> Result<(), ContractError> {
    if is_paused(env) {
        return Err(ContractError::ContractPaused);
    }
    Ok(())
}

/// Return the full history of pause/unpause events.
pub fn pause_history(env: &Env) -> Vec<PauseRecord> {
    Storage::get_pause_history(env)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::storage::Storage;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env, String};

    #[test]
    fn test_pause_and_unpause() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        // Initially not paused
        assert!(!is_paused(&env));
        assert_eq!(require_not_paused(&env), Ok(()));

        // Pause by admin
        let reason = String::from_str(&env, "Critical bug");
        let result = pause(&env, &admin, reason.clone());
        assert_eq!(result, Ok(()));

        // Verify paused state
        assert!(is_paused(&env));
        assert_eq!(require_not_paused(&env), Err(ContractError::ContractPaused));

        let history = pause_history(&env);
        assert_eq!(history.len(), 1);
        let record = history.get(0).unwrap();
        assert_eq!(record.paused_by, admin);
        assert_eq!(record.reason, reason);
        assert!(record.unpaused_at.is_none());

        // Unpause by admin
        let result = unpause(&env, &admin);
        assert_eq!(result, Ok(()));

        // Verify unpaused state
        assert!(!is_paused(&env));
        assert_eq!(require_not_paused(&env), Ok(()));

        let history = pause_history(&env);
        let updated_record = history.get(0).unwrap();
        assert!(updated_record.unpaused_at.is_some());
    }

    #[test]
    fn test_pause_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let random_caller = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        // Random caller attempts to pause
        let reason = String::from_str(&env, "Trying to pause");
        let result = pause(&env, &random_caller, reason);
        assert_eq!(result, Err(ContractError::Unauthorized));

        // Verify still not paused
        assert!(!is_paused(&env));
    }

    #[test]
    fn test_unpause_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let random_caller = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        pause(&env, &admin, String::from_str(&env, "Reason")).unwrap();

        // Random caller attempts to unpause
        let result = unpause(&env, &random_caller);
        assert_eq!(result, Err(ContractError::Unauthorized));

        // Verify still paused
        assert!(is_paused(&env));
    }

    #[test]
    fn test_pause_already_paused() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        pause(&env, &admin, String::from_str(&env, "First reason")).unwrap();

        let result = pause(&env, &admin, String::from_str(&env, "Second reason"));
        assert_eq!(result, Err(ContractError::InvalidState));
    }

    #[test]
    fn test_unpause_not_paused() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let result = unpause(&env, &admin);
        assert_eq!(result, Err(ContractError::InvalidState));
    }
}
