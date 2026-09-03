use soroban_sdk::{contractevent, Address, Bytes, Env, Vec};

use crate::constants::MAX_HOOKS_PER_EVENT;
use crate::cross_contract;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{HookCallResult, HookEvent, HookRegistration};

// ── Events ────────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookTriggered {
    pub event: u32,
    pub hook_index: u32,
    pub success: bool,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookRegisteredEvent {
    pub event: u32,
    pub hook_index: u32,
    pub target_contract: Address,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Register an external contract hook for an event. Admin only.
pub fn register_hook(
    env: &Env,
    admin: &Address,
    event: HookEvent,
    target_contract: Address,
    max_gas_budget: u32,
) -> Result<u32, ContractError> {
    admin.require_auth();

    let mut hooks = Storage::get_hook_registry(env, &event);

    if hooks.len() >= MAX_HOOKS_PER_EVENT {
        return Err(ContractError::HookLimitExceeded);
    }

    let hook_index = hooks.len();

    let registration = HookRegistration {
        event: event.clone(),
        target_contract: target_contract.clone(),
        registered_by: admin.clone(),
        registered_at: env.ledger().timestamp(),
        is_active: true,
        max_gas_budget,
    };

    hooks.push_back(registration);
    Storage::set_hook_registry(env, &event, &hooks);

    HookRegisteredEvent {
        event: event as u32,
        hook_index,
        target_contract,
    }
    .publish(env);

    Ok(hook_index)
}

/// Deactivate a hook. Admin only.
pub fn deactivate_hook(
    env: &Env,
    admin: &Address,
    event: HookEvent,
    hook_index: u32,
) -> Result<(), ContractError> {
    admin.require_auth();

    let mut hooks = Storage::get_hook_registry(env, &event);

    if hook_index >= hooks.len() {
        return Err(ContractError::HookNotFound);
    }

    let mut hook = hooks.get(hook_index).ok_or(ContractError::HookNotFound)?;
    if !hook.is_active {
        return Err(ContractError::HookAlreadyInactive);
    }

    hook.is_active = false;
    hooks.set(hook_index, hook);
    Storage::set_hook_registry(env, &event, &hooks);

    Ok(())
}

/// Trigger all active hooks for an event.
/// Hook failures are captured but do not revert the parent transaction.
pub fn trigger(env: &Env, event: HookEvent, payload: Bytes) -> Vec<HookCallResult> {
    let hooks = Storage::get_hook_registry(env, &event);
    let mut results: Vec<HookCallResult> = Vec::new(env);

    let event_u32 = event as u32;

    for (idx, hook) in hooks.iter().enumerate() {
        if !hook.is_active {
            continue;
        }

        let success = cross_contract::call_hook_receiver(
            env,
            &hook.target_contract,
            event_u32,
            payload.clone(),
        );
        let error_code: Option<u32> = if success { None } else { Some(1) };

        let call_result = HookCallResult {
            hook_index: idx as u32,
            success,
            error_code,
        };

        HookTriggered {
            event: event_u32,
            hook_index: idx as u32,
            success,
        }
        .publish(env);

        results.push_back(call_result);
    }

    results
}

/// Return all registered hooks for an event.
pub fn get_hooks(env: &Env, event: HookEvent) -> Vec<HookRegistration> {
    Storage::get_hook_registry(env, &event)
}

/// Check if any hooks are registered for an event.
pub fn has_hooks(env: &Env, event: HookEvent) -> bool {
    let hooks = Storage::get_hook_registry(env, &event);
    for hook in hooks.iter() {
        if hook.is_active {
            return true;
        }
    }
    false
}

// ── Unit Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Bytes, Env};

    #[test]
    fn test_register_and_deactivate_hook() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let target = Address::generate(&env);

        let idx =
            register_hook(&env, &admin, HookEvent::GrantCreated, target.clone(), 1000).unwrap();
        assert_eq!(idx, 0);
        assert!(has_hooks(&env, HookEvent::GrantCreated));

        deactivate_hook(&env, &admin, HookEvent::GrantCreated, 0).unwrap();
        assert!(!has_hooks(&env, HookEvent::GrantCreated));
    }

    #[test]
    fn test_max_hooks_limit_enforced() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        for _ in 0..MAX_HOOKS_PER_EVENT {
            let target = Address::generate(&env);
            register_hook(&env, &admin, HookEvent::MilestoneApproved, target, 1000).unwrap();
        }
        let extra = Address::generate(&env);
        let err =
            register_hook(&env, &admin, HookEvent::MilestoneApproved, extra, 1000).unwrap_err();
        assert_eq!(err, ContractError::HookLimitExceeded);
    }

    #[test]
    fn test_deactivated_hook_skipped_in_get_hooks() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let target = Address::generate(&env);

        register_hook(&env, &admin, HookEvent::GrantCreated, target, 500).unwrap();
        deactivate_hook(&env, &admin, HookEvent::GrantCreated, 0).unwrap();

        let hooks = get_hooks(&env, HookEvent::GrantCreated);
        assert_eq!(hooks.len(), 1);
        assert!(!hooks.get(0).unwrap().is_active);
    }

    #[test]
    fn test_trigger_empty_hooks_returns_empty_results() {
        let env = Env::default();
        env.mock_all_auths();
        let payload = Bytes::new(&env);
        let results = trigger(&env, HookEvent::DisputeResolved, payload);
        assert_eq!(results.len(), 0);
    }
}
