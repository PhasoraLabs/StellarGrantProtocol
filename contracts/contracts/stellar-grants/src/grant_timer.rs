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

        execute_timer_action(env, &grant, &timer);

        timer.fired = true;
        timer.fired_at = Some(now);
        timer.triggered_by = Some(caller.clone());
        timers.set(i, timer.clone());
        fired_count += 1;

        crate::events::Events::milestone_status_changed(
            env,
            grant_id,
            0,
            crate::types::MilestoneState::Pending,
        );
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

fn execute_timer_action(env: &Env, grant: &crate::types::Grant, timer: &TimerRecord) {
    match timer.trigger_type {
        TimerTriggerType::AutoExpire => {
            let reason = String::from_str(env, "auto-expired by timer");
            let _ = cancel_grant_internal(env, grant.id, &reason);
        }
        TimerTriggerType::AutoCancel => {
            let reason = String::from_str(env, "auto-cancelled: not funded by deadline");
            let _ = cancel_grant_internal(env, grant.id, &reason);
        }
        TimerTriggerType::AutoActivate => {
            // Grant is already Active; this is a no-op marker
        }
        TimerTriggerType::AutoReleaseLockup => {
            // Release lockup logic placeholder
        }
        TimerTriggerType::CustomCallback => {
            // Custom callback placeholder
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
