use soroban_sdk::{Address, Env, String, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{GrantStatus, TimerRecord, TimerTriggerType};

/// Register a new timer for a grant. Owner or protocol (for defaults).
pub fn register_timer(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    trigger_type: TimerTriggerType,
    fires_at: u64,
) -> Result<(), ContractError> {
    #[cfg(not(test))]
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    let is_owner = grant.owner == *caller;
    let is_admin = Storage::get_global_admin(env) == Some(caller.clone());

    if !is_owner && !is_admin {
        return Err(ContractError::Unauthorized);
    }

    let mut timers = Storage::get_grant_timers(env, grant_id);

    for existing in timers.iter() {
        if existing.trigger_type == trigger_type && !existing.fired {
            return Err(ContractError::InvalidInput);
        }
    }

    let record = TimerRecord {
        grant_id,
        trigger_type,
        fires_at,
        fires_at_ledger: None,
        fired: false,
        fired_at: None,
        triggered_by: None,
    };

    timers.push_back(record);
    Storage::set_grant_timers(env, grant_id, &timers);

    Ok(())
}

/// Attempt to fire all eligible timers for a grant. Anyone can call.
pub fn trigger_timers(env: &Env, caller: &Address, grant_id: u64) -> u32 {
    let grant = match Storage::get_grant(env, grant_id) {
        Some(g) => g,
        None => return 0,
    };

    let mut timers = Storage::get_grant_timers(env, grant_id);
    let mut fired_count: u32 = 0;
    let now = env.ledger().timestamp();

    for i in 0..timers.len() {
        let mut timer = timers.get(i).unwrap();

        if timer.fired || timer.grant_id != grant_id {
            continue;
        }

        if now < timer.fires_at {
            continue;
        }

        let eligible = match timer.trigger_type {
            TimerTriggerType::AutoExpire => {
                grant.status == GrantStatus::Active
                    && grant.milestones_paid_out < grant.total_milestones
            }
            TimerTriggerType::AutoActivate => {
                grant.status == GrantStatus::Active && grant.escrow_balance >= grant.total_amount
            }
            TimerTriggerType::AutoCancel => {
                grant.status == GrantStatus::Active && grant.escrow_balance == 0
            }
            TimerTriggerType::AutoReleaseLockup => grant.status == GrantStatus::Active,
            TimerTriggerType::CustomCallback => true,
        };

        if !eligible {
            continue;
        }

        let success = execute_timer_action(env, &grant, &timer);

        if success {
            timer.fired = true;
            timer.fired_at = Some(now);
            timer.triggered_by = Some(caller.clone());
            timers.set(i, timer.clone());
            fired_count += 1;
        }
    }

    if fired_count > 0 {
        Storage::set_grant_timers(env, grant_id, &timers);
    }

    fired_count
}

/// Return all timers for a grant.
pub fn get_timers(env: &Env, grant_id: u64) -> Vec<TimerRecord> {
    Storage::get_grant_timers(env, grant_id)
}

/// Return only unfired, eligible (past fires_at) timers.
pub fn pending_timers(env: &Env, grant_id: u64) -> Vec<TimerRecord> {
    let timers = Storage::get_grant_timers(env, grant_id);
    let now = env.ledger().timestamp();
    let mut pending = Vec::new(env);

    for timer in timers.iter() {
        if !timer.fired && now >= timer.fires_at {
            pending.push_back(timer);
        }
    }

    pending
}

/// Cancel a timer (owner or admin only).
pub fn cancel_timer(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    trigger_type: TimerTriggerType,
) -> Result<(), ContractError> {
    #[cfg(not(test))]
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let is_owner = grant.owner == *caller;
    let is_admin = Storage::get_global_admin(env) == Some(caller.clone());

    if !is_owner && !is_admin {
        return Err(ContractError::Unauthorized);
    }

    let mut timers = Storage::get_grant_timers(env, grant_id);
    let mut found = false;

    for i in 0..timers.len() {
        let timer = timers.get(i).unwrap();
        if timer.trigger_type == trigger_type && !timer.fired {
            timers.remove(i);
            found = true;
            break;
        }
    }

    if !found {
        return Err(ContractError::TimerNotFound);
    }

    Storage::set_grant_timers(env, grant_id, &timers);
    Ok(())
}

/// Returns `true` when the action was actually performed, `false` for
/// unimplemented trigger types that should not be marked as fired.
fn execute_timer_action(env: &Env, grant: &crate::types::Grant, timer: &TimerRecord) -> bool {
    match timer.trigger_type {
        TimerTriggerType::AutoExpire => {
            let reason = String::from_str(env, "auto-expired by timer");
            let _ = cancel_grant_internal(env, grant.id, &reason);
            true
        }
        TimerTriggerType::AutoCancel => {
            let reason = String::from_str(env, "auto-cancelled: not funded by deadline");
            let _ = cancel_grant_internal(env, grant.id, &reason);
            true
        }
        TimerTriggerType::AutoActivate
        | TimerTriggerType::AutoReleaseLockup
        | TimerTriggerType::CustomCallback => {
            // Not yet implemented — don't claim success.
            false
        }
    }
}

/// Internal cancellation helper that performs full escrow/index cleanup.
/// Called by both cancel_grant (with auth) and timer triggers (permissionless).
fn cancel_grant_internal(env: &Env, grant_id: u64, reason: &String) -> Result<(), ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }

    // Forfeit collateral if applicable.
    if let Some(req) = crate::collateral::get_requirement(env, grant_id) {
        let forfeit_reason = String::from_str(env, "grant cancelled by timer");
        let _ = crate::collateral::forfeit(
            env,
            &grant.owner,
            grant_id,
            &grant.owner,
            req.forfeit_on_abandon_bps,
            forfeit_reason,
        );
    }

    let total_refundable = grant.escrow_balance;
    if total_refundable > 0 {
        // Use the configured refund policy if set, otherwise refund all.
        if crate::refund::has_policy(env, grant_id) {
            let _ = crate::refund::execute_refund(env, grant_id, &grant.owner);
        } else {
            let _ = crate::escrow::refund_all(env, grant_id);
        }
    }

    let mut g = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let old_status = g.status;
    g.status = GrantStatus::Cancelled;
    g.escrow_balance = 0;
    g.reason = Some(reason.clone());
    g.timestamp = env.ledger().timestamp();

    // Move grant out of Active index and into Cancelled index.
    crate::grant_index::on_status_changed(env, grant_id, old_status, GrantStatus::Cancelled);

    Storage::set_grant(env, grant_id, &g);
    crate::data_export::set_last_updated(env, grant_id, env.ledger().timestamp());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grant_index;
    use crate::storage::Storage;
    use crate::types::{Grant, GrantStatus, TimerTriggerType};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env};

    fn make_grant(env: &Env, owner: &Address) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: soroban_sdk::String::from_str(env, "Test Grant"),
            description: soroban_sdk::String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 10_000,
            milestone_amount: 5_000,
            reviewers: soroban_sdk::Vec::new(env),
            total_milestones: 3,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: soroban_sdk::Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        }
    }

    fn set_ledger(env: &Env, timestamp: u64) {
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp,
            protocol_version: 25,
            sequence_number: 100,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 100_000,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 1_000_000,
        });
    }

    fn with_setup(f: impl FnOnce(&Env, &Address, &Address)) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_global_admin(&env, &admin);
            let grant = make_grant(&env, &owner);
            Storage::set_grant(&env, 1, &grant);
            grant_index::on_grant_created(&env, 1, &owner, &grant.token, GrantStatus::Active);
            set_ledger(&env, 1_000);
            f(&env, &admin, &owner);
        });
    }

    fn patch_grant(env: &Env, f: impl FnOnce(&mut Grant)) {
        let mut grant = Storage::get_grant(env, 1).unwrap();
        f(&mut grant);
        Storage::set_grant(env, 1, &grant);
    }

    #[test]
    fn register_and_trigger_twice_is_noop_on_second_call() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::CustomCallback, 1_000).unwrap();

            let first = trigger_timers(env, owner, 1);
            assert_eq!(first, 1);
            let timers = get_timers(env, 1);
            assert!(timers.get(0).unwrap().fired);

            let second = trigger_timers(env, owner, 1);
            assert_eq!(second, 0);
            let timers = get_timers(env, 1);
            assert_eq!(timers.len(), 1);
            assert!(timers.get(0).unwrap().fired);
        });
    }

    #[test]
    fn duplicate_unfired_registration_of_same_trigger_type_is_rejected() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::AutoExpire, 2_000).unwrap();
            let err = register_timer(env, owner, 1, TimerTriggerType::AutoExpire, 3_000);
            assert_eq!(err, Err(ContractError::InvalidInput));
            assert_eq!(get_timers(env, 1).len(), 1);
        });
    }

    #[test]
    fn different_trigger_types_can_be_registered_together() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::AutoExpire, 2_000).unwrap();
            register_timer(env, owner, 1, TimerTriggerType::AutoCancel, 2_000).unwrap();
            assert_eq!(get_timers(env, 1).len(), 2);
        });
    }

    #[test]
    fn register_after_fired_same_trigger_type_is_allowed() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::CustomCallback, 1_000).unwrap();
            assert_eq!(trigger_timers(env, owner, 1), 1);
            register_timer(env, owner, 1, TimerTriggerType::CustomCallback, 2_000).unwrap();
            assert_eq!(get_timers(env, 1).len(), 2);
        });
    }

    #[test]
    fn trigger_before_fires_at_is_noop() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::CustomCallback, 5_000).unwrap();
            assert_eq!(trigger_timers(env, owner, 1), 0);
            assert!(!get_timers(env, 1).get(0).unwrap().fired);
        });
    }

    #[test]
    fn auto_expire_eligible_cancels_grant_and_updates_indexes() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::AutoExpire, 1_000).unwrap();

            assert_eq!(trigger_timers(env, owner, 1), 1);

            let grant = Storage::get_grant(env, 1).unwrap();
            assert_eq!(grant.status, GrantStatus::Cancelled);
            assert_eq!(grant.escrow_balance, 0);
            assert!(grant.reason.is_some());
            assert!(!grant_index::by_status(env, GrantStatus::Active, 0, 10).contains(1));
            assert!(grant_index::by_status(env, GrantStatus::Cancelled, 0, 10).contains(1));
        });
    }

    #[test]
    fn auto_expire_ineligible_when_all_milestones_paid() {
        with_setup(|env, _admin, owner| {
            patch_grant(env, |g| g.milestones_paid_out = g.total_milestones);
            register_timer(env, owner, 1, TimerTriggerType::AutoExpire, 1_000).unwrap();

            assert_eq!(trigger_timers(env, owner, 1), 0);
            assert_eq!(
                Storage::get_grant(env, 1).unwrap().status,
                GrantStatus::Active
            );
            assert!(!get_timers(env, 1).get(0).unwrap().fired);
        });
    }

    #[test]
    fn auto_activate_eligible_when_fully_funded() {
        with_setup(|env, _admin, owner| {
            patch_grant(env, |g| g.escrow_balance = g.total_amount);
            register_timer(env, owner, 1, TimerTriggerType::AutoActivate, 1_000).unwrap();

            assert_eq!(trigger_timers(env, owner, 1), 1);
            assert!(get_timers(env, 1).get(0).unwrap().fired);
            assert_eq!(
                Storage::get_grant(env, 1).unwrap().status,
                GrantStatus::Active
            );
        });
    }

    #[test]
    fn auto_activate_ineligible_when_underfunded() {
        with_setup(|env, _admin, owner| {
            patch_grant(env, |g| g.escrow_balance = g.total_amount - 1);
            register_timer(env, owner, 1, TimerTriggerType::AutoActivate, 1_000).unwrap();

            assert_eq!(trigger_timers(env, owner, 1), 0);
            assert!(!get_timers(env, 1).get(0).unwrap().fired);
        });
    }

    #[test]
    fn auto_cancel_eligible_when_unfunded_cleans_indexes() {
        with_setup(|env, _admin, owner| {
            patch_grant(env, |g| g.escrow_balance = 0);
            register_timer(env, owner, 1, TimerTriggerType::AutoCancel, 1_000).unwrap();

            assert_eq!(trigger_timers(env, owner, 1), 1);
            let grant = Storage::get_grant(env, 1).unwrap();
            assert_eq!(grant.status, GrantStatus::Cancelled);
            assert!(!grant_index::by_status(env, GrantStatus::Active, 0, 10).contains(1));
            assert!(grant_index::by_status(env, GrantStatus::Cancelled, 0, 10).contains(1));
        });
    }

    #[test]
    fn auto_cancel_ineligible_when_escrow_nonzero() {
        with_setup(|env, _admin, owner| {
            patch_grant(env, |g| g.escrow_balance = 1);
            register_timer(env, owner, 1, TimerTriggerType::AutoCancel, 1_000).unwrap();

            assert_eq!(trigger_timers(env, owner, 1), 0);
            assert_eq!(
                Storage::get_grant(env, 1).unwrap().status,
                GrantStatus::Active
            );
        });
    }

    #[test]
    fn auto_release_lockup_eligible_only_when_active() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::AutoReleaseLockup, 1_000).unwrap();
            assert_eq!(trigger_timers(env, owner, 1), 1);
        });

        with_setup(|env, _admin, owner| {
            patch_grant(env, |g| g.status = GrantStatus::Completed);
            register_timer(env, owner, 1, TimerTriggerType::AutoReleaseLockup, 1_000).unwrap();
            assert_eq!(trigger_timers(env, owner, 1), 0);
        });
    }

    #[test]
    fn custom_callback_eligible_regardless_of_grant_status() {
        with_setup(|env, _admin, owner| {
            patch_grant(env, |g| g.status = GrantStatus::Completed);
            register_timer(env, owner, 1, TimerTriggerType::CustomCallback, 1_000).unwrap();
            assert_eq!(trigger_timers(env, owner, 1), 1);
        });
    }

    #[test]
    fn cancel_timer_removes_unfired_timer() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::AutoExpire, 5_000).unwrap();
            cancel_timer(env, owner, 1, TimerTriggerType::AutoExpire).unwrap();
            assert_eq!(get_timers(env, 1).len(), 0);
            assert_eq!(pending_timers(env, 1).len(), 0);
            assert_eq!(trigger_timers(env, owner, 1), 0);
        });
    }

    #[test]
    fn cancel_timer_missing_is_timer_not_found() {
        with_setup(|env, _admin, owner| {
            let err = cancel_timer(env, owner, 1, TimerTriggerType::AutoExpire);
            assert_eq!(err, Err(ContractError::TimerNotFound));
        });
    }

    #[test]
    fn cancel_already_fired_timer_is_timer_not_found() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::CustomCallback, 1_000).unwrap();
            assert_eq!(trigger_timers(env, owner, 1), 1);
            let err = cancel_timer(env, owner, 1, TimerTriggerType::CustomCallback);
            assert_eq!(err, Err(ContractError::TimerNotFound));
        });
    }

    #[test]
    fn unauthorized_caller_cannot_register_or_cancel() {
        with_setup(|env, _admin, owner| {
            let stranger = Address::generate(env);
            let err = register_timer(env, &stranger, 1, TimerTriggerType::AutoExpire, 2_000);
            assert_eq!(err, Err(ContractError::Unauthorized));

            register_timer(env, owner, 1, TimerTriggerType::AutoExpire, 2_000).unwrap();
            let err = cancel_timer(env, &stranger, 1, TimerTriggerType::AutoExpire);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn admin_can_register_timer() {
        with_setup(|env, admin, _owner| {
            register_timer(env, admin, 1, TimerTriggerType::AutoExpire, 2_000).unwrap();
            assert_eq!(get_timers(env, 1).len(), 1);
        });
    }

    #[test]
    fn trigger_unknown_grant_returns_zero() {
        with_setup(|env, _admin, owner| {
            assert_eq!(trigger_timers(env, owner, 999), 0);
        });
    }

    #[test]
    fn pending_timers_lists_due_unfired_only() {
        with_setup(|env, _admin, owner| {
            register_timer(env, owner, 1, TimerTriggerType::CustomCallback, 500).unwrap();
            register_timer(env, owner, 1, TimerTriggerType::AutoActivate, 9_000).unwrap();
            let pending = pending_timers(env, 1);
            assert_eq!(pending.len(), 1);
            assert_eq!(
                pending.get(0).unwrap().trigger_type,
                TimerTriggerType::CustomCallback
            );
        });
    }
}
