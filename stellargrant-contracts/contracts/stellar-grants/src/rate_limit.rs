use soroban_sdk::{Address, Env};

use crate::config;
use crate::constants::{
    RATE_LIMIT_BOUNTY_CREATE_MAX, RATE_LIMIT_BOUNTY_CREATE_WINDOW,
    RATE_LIMIT_CONTRIBUTOR_REGISTER_MAX, RATE_LIMIT_CONTRIBUTOR_REGISTER_WINDOW,
    RATE_LIMIT_DISPUTE_RAISE_MAX, RATE_LIMIT_DISPUTE_RAISE_WINDOW, RATE_LIMIT_GRANT_CREATE_MAX,
    RATE_LIMIT_GRANT_CREATE_WINDOW, RATE_LIMIT_MILESTONE_SUBMIT_MAX,
    RATE_LIMIT_MILESTONE_SUBMIT_WINDOW,
};
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{RateLimitAction, RateLimitRecord};

fn is_admin(env: &Env, address: &Address) -> bool {
    Storage::get_global_admin(env) == Some(address.clone())
}

fn effective_limit(env: &Env, action: &RateLimitAction) -> (u32, u64) {
    let (base_max, window) = limit_for(action);
    let multiplier = config::get_config(env).rate_limit_multiplier.max(1);
    (base_max.saturating_mul(multiplier), window)
}

/// Check if `address` is within rate limit for `action`.
pub fn check_and_increment(
    env: &Env,
    address: &Address,
    action: RateLimitAction,
) -> Result<(), ContractError> {
    if is_admin(env, address) {
        return Ok(());
    }

    let (max_per_window, window_duration) = effective_limit(env, &action);
    let now = env.ledger().timestamp();

    let mut record =
        Storage::get_rate_limit_record(env, address, &action).unwrap_or(RateLimitRecord {
            address: address.clone(),
            action: action.clone(),
            count: 0,
            window_start: now,
            window_duration,
            max_per_window,
        });

    if now.saturating_sub(record.window_start) > record.window_duration {
        record.count = 0;
        record.window_start = now;
        record.window_duration = window_duration;
        record.max_per_window = max_per_window;
    }

    if record.count >= max_per_window {
        return Err(ContractError::InvalidInput);
    }

    record.count = record.count.saturating_add(1);
    Storage::set_rate_limit_record(env, address, &action, &record);
    Ok(())
}

/// Return the current rate limit record for an address and action.
pub fn get_record(
    env: &Env,
    address: &Address,
    action: RateLimitAction,
) -> Option<RateLimitRecord> {
    Storage::get_rate_limit_record(env, address, &action)
}

/// Manually reset the rate limit record for an address. Admin only.
pub fn reset_record(
    env: &Env,
    admin: &Address,
    address: &Address,
    action: RateLimitAction,
) -> Result<(), ContractError> {
    admin.require_auth();
    if !is_admin(env, admin) {
        return Err(ContractError::Unauthorized);
    }

    let (max_per_window, window_duration) = effective_limit(env, &action);
    let record = RateLimitRecord {
        address: address.clone(),
        action,
        count: 0,
        window_start: env.ledger().timestamp(),
        window_duration,
        max_per_window,
    };
    Storage::set_rate_limit_record(env, address, &record.action, &record);
    Ok(())
}

/// Return the configured limit for an action (from constants).
pub fn limit_for(action: &RateLimitAction) -> (u32, u64) {
    match action {
        RateLimitAction::GrantCreate => {
            (RATE_LIMIT_GRANT_CREATE_MAX, RATE_LIMIT_GRANT_CREATE_WINDOW)
        }
        RateLimitAction::MilestoneSubmit => (
            RATE_LIMIT_MILESTONE_SUBMIT_MAX,
            RATE_LIMIT_MILESTONE_SUBMIT_WINDOW,
        ),
        RateLimitAction::ContributorRegister => (
            RATE_LIMIT_CONTRIBUTOR_REGISTER_MAX,
            RATE_LIMIT_CONTRIBUTOR_REGISTER_WINDOW,
        ),
        RateLimitAction::DisputeRaise => (
            RATE_LIMIT_DISPUTE_RAISE_MAX,
            RATE_LIMIT_DISPUTE_RAISE_WINDOW,
        ),
        RateLimitAction::BountyCreate => (
            RATE_LIMIT_BOUNTY_CREATE_MAX,
            RATE_LIMIT_BOUNTY_CREATE_WINDOW,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env};

    fn with_contract<F, R>(env: &Env, f: F) -> R
    where
        F: FnOnce(&Address, &Address) -> R,
    {
        let admin = Address::generate(env);
        let user = Address::generate(env);
        let contract_id = env.register(StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            Storage::set_global_admin(env, &admin);
            Storage::set_protocol_config(env, &config::default_config());
            f(&admin, &user)
        })
    }

    #[test]
    fn test_exceed_limit_returns_error() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, |_admin, user| {
            let action = RateLimitAction::GrantCreate;
            let (max, _) = limit_for(&action);

            for _ in 0..max {
                check_and_increment(&env, user, action.clone()).unwrap();
            }
            assert_eq!(
                check_and_increment(&env, user, action),
                Err(ContractError::InvalidInput)
            );
        });
    }

    #[test]
    fn test_window_reset_allows_new_actions() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, |_admin, user| {
            let action = RateLimitAction::MilestoneSubmit;
            let (_, window) = limit_for(&action);

            for _ in 0..limit_for(&action).0 {
                check_and_increment(&env, user, action.clone()).unwrap();
            }
            assert_eq!(
                check_and_increment(&env, user, action.clone()),
                Err(ContractError::InvalidInput)
            );

            env.ledger()
                .set_timestamp(env.ledger().timestamp().saturating_add(window + 1));
            check_and_increment(&env, user, action).unwrap();
        });
    }

    #[test]
    fn test_admin_bypasses_limit() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, |admin, _user| {
            let action = RateLimitAction::DisputeRaise;
            let (max, _) = limit_for(&action);

            for _ in 0..max.saturating_add(5) {
                check_and_increment(&env, admin, action.clone()).unwrap();
            }
        });
    }

    #[test]
    fn test_reset_record_clears_count() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, |admin, user| {
            let action = RateLimitAction::ContributorRegister;

            check_and_increment(&env, user, action.clone()).unwrap();
            reset_record(&env, admin, user, action.clone()).unwrap();

            let record = get_record(&env, user, action).unwrap();
            assert_eq!(record.count, 0);
        });
    }
}
