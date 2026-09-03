use crate::storage::helpers::Storage;
use crate::storage::keys::DataKey;
use crate::types::{ContractError, GrantPauseRecord};
use soroban_sdk::{Address, Env, String, Symbol};

pub fn pause(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    reason: String,
    auto_unpause_at: Option<u64>,
) -> Result<(), ContractError> {
    caller.require_auth();
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
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

    if let Some(time) = auto_unpause_at {
        if time <= env.ledger().timestamp() {
            record.auto_unpause_at = None;
        }
    }

    env.storage().persistent().set(&key, &record);
    env.events()
        .publish((Symbol::new(env, "grant_paused"), grant_id), caller.clone());
    Ok(())
}

pub fn unpause(env: &Env, caller: &Address, grant_id: u64) -> Result<(), ContractError> {
    caller.require_auth();
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }

    let key = DataKey::GrantPaused(grant_id);
    let mut record = get_record(env, grant_id).ok_or(ContractError::InvalidState)?;
    let now = env.ledger().timestamp();
    record.unpause_history.push_back((caller.clone(), now));
    record.auto_unpause_at = Some(now);
    env.storage().persistent().set(&key, &record);
    env.events().publish(
        (Symbol::new(env, "grant_unpaused"), grant_id),
        caller.clone(),
    );
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
    env.storage()
        .persistent()
        .get(&DataKey::GrantPaused(grant_id))
}

pub fn try_auto_unpause(env: &Env, grant_id: u64) -> bool {
    if let Some(mut record) = get_record(env, grant_id) {
        if let Some(auto_unpause) = record.auto_unpause_at {
            let now = env.ledger().timestamp();
            if now >= auto_unpause {
                let contract_address = env.current_contract_address();
                record.unpause_history.push_back((contract_address, now));
                env.storage()
                    .persistent()
                    .set(&DataKey::GrantPaused(grant_id), &record);
                env.events()
                    .publish((Symbol::new(env, "grant_unpaused"), grant_id), auto_unpause);
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::Grant;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Env, String, Vec};

    fn with_contract(env: &Env, f: impl FnOnce()) {
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, f);
    }

    fn with_contract_id(env: &Env, contract_id: &Address, f: impl FnOnce()) {
        env.as_contract(contract_id, f);
    }

    fn setup_grant(env: &Env, owner: &Address, grant_id: u64) {
        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "desc"),
            token: Address::generate(env),
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 3,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            status: crate::types::GrantStatus::Active,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);
    }

    #[test]
    fn test_only_owner_can_pause() {
        let env = Env::default();
        env.mock_all_auths();

        with_contract(&env, || {
            let owner = Address::generate(&env);
            let stranger = Address::generate(&env);
            let grant_id = 1;
            setup_grant(&env, &owner, grant_id);

            let result = pause(
                &env,
                &stranger,
                grant_id,
                String::from_str(&env, "test"),
                None,
            );
            assert_eq!(result, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn test_only_owner_can_unpause() {
        let env = Env::default();
        env.mock_all_auths();

        with_contract(&env, || {
            let owner = Address::generate(&env);
            let stranger = Address::generate(&env);
            let grant_id = 1;
            setup_grant(&env, &owner, grant_id);

            pause(
                &env,
                &owner,
                grant_id,
                String::from_str(&env, "reason"),
                None,
            )
            .unwrap();

            let result = unpause(&env, &stranger, grant_id);
            assert_eq!(result, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn test_pause_and_unpause_flow() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(crate::StellarGrantsContract, ());
        let grant_id = 1;
        let owner = Address::generate(&env);

        with_contract_id(&env, &contract_id, || {
            setup_grant(&env, &owner, grant_id);
            assert!(!is_paused(&env, grant_id));

            pause(
                &env,
                &owner,
                grant_id,
                String::from_str(&env, "maintenance"),
                None,
            )
            .unwrap();
            assert!(is_paused(&env, grant_id));
        });

        with_contract_id(&env, &contract_id, || {
            unpause(&env, &owner, grant_id).unwrap();
            assert!(!is_paused(&env, grant_id));
        });
    }

    #[test]
    fn test_require_not_paused_blocks_when_paused() {
        let env = Env::default();
        env.mock_all_auths();

        with_contract(&env, || {
            let owner = Address::generate(&env);
            let grant_id = 1;
            setup_grant(&env, &owner, grant_id);

            assert!(require_not_paused(&env, grant_id).is_ok());

            pause(&env, &owner, grant_id, String::from_str(&env, "test"), None).unwrap();
            assert_eq!(
                require_not_paused(&env, grant_id),
                Err(ContractError::ContractPaused)
            );
        });
    }

    #[test]
    fn test_auto_unpause_elapsed_is_paused_false() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(crate::StellarGrantsContract, ());
        let grant_id = 1;
        let now = env.ledger().timestamp();
        let future = now + 100;

        with_contract_id(&env, &contract_id, || {
            let owner = Address::generate(&env);
            setup_grant(&env, &owner, grant_id);

            pause(
                &env,
                &owner,
                grant_id,
                String::from_str(&env, "auto"),
                Some(future),
            )
            .unwrap();
            assert!(is_paused(&env, grant_id));
        });

        env.ledger().set_timestamp(future + 1);

        with_contract_id(&env, &contract_id, || {
            assert!(!is_paused(&env, grant_id));
        });
    }

    #[test]
    fn test_try_auto_unpause_records_history() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(crate::StellarGrantsContract, ());
        let grant_id = 1;
        let now = env.ledger().timestamp();
        let future = now + 100;

        with_contract_id(&env, &contract_id, || {
            let owner = Address::generate(&env);
            setup_grant(&env, &owner, grant_id);

            pause(
                &env,
                &owner,
                grant_id,
                String::from_str(&env, "auto"),
                Some(future),
            )
            .unwrap();
        });

        env.ledger().set_timestamp(future + 1);

        with_contract_id(&env, &contract_id, || {
            let result = try_auto_unpause(&env, grant_id);
            assert!(result);

            let record = get_record(&env, grant_id).unwrap();
            assert_eq!(record.unpause_history.len(), 1);
        });
    }

    #[test]
    fn test_unpause_never_paused_grant_returns_invalid_state() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(crate::StellarGrantsContract, ());
        let grant_id = 1;
        let owner = Address::generate(&env);

        with_contract_id(&env, &contract_id, || {
            setup_grant(&env, &owner, grant_id);
            assert!(!is_paused(&env, grant_id));

            let result = unpause(&env, &owner, grant_id);
            assert_eq!(result, Err(ContractError::InvalidState));
        });
    }
}
