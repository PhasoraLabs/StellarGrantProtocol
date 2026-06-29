use soroban_sdk::{Address, Env, Vec};

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
                grant.status == GrantStatus::Active && grant.milestones_paid_out < grant.total_milestones
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
            if let Some(mut g) = Storage::get_grant(env, grant.id) {
                g.status = GrantStatus::Cancelled;
                g.reason = Some(soroban_sdk::String::from_str(env, "auto-expired by timer"));
                g.timestamp = env.ledger().timestamp();
                Storage::set_grant(env, grant.id, &g);
            }
        }
        TimerTriggerType::AutoCancel => {
            if let Some(mut g) = Storage::get_grant(env, grant.id) {
                g.status = GrantStatus::Cancelled;
                g.reason = Some(soroban_sdk::String::from_str(env, "auto-cancelled: not funded by deadline"));
                g.timestamp = env.ledger().timestamp();
                Storage::set_grant(env, grant.id, &g);
            }
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
