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
                g.reason = Some(soroban_sdk::String::from_str(
                    env,
                    "auto-cancelled: not funded by deadline",
                ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Grant;
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::{Env, String, Vec};

    // Each state-changing call (`register_timer` / `cancel_timer`) runs `require_auth`
    // and must therefore execute in its own contract frame under `mock_all_auths`.
    fn setup(env: &Env) -> Address {
        env.mock_all_auths();
        env.register(crate::StellarGrantsContract, ())
    }

    #[allow(clippy::too_many_arguments)]
    fn setup_grant(
        env: &Env,
        grant_id: u64,
        owner: &Address,
        status: GrantStatus,
        escrow_balance: i128,
        total_amount: i128,
        milestones_paid_out: u32,
        total_milestones: u32,
    ) {
        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Grant"),
            description: String::from_str(env, "desc"),
            token: Address::generate(env),
            status,
            total_amount,
            milestone_amount: 100,
            reviewers: Vec::new(env),
            total_milestones,
            milestones_paid_out,
            escrow_balance,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);
    }

    #[test]
    fn timer_fires_only_once() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 0, 1000, 0, 3);
            register_timer(&env, &owner, 1, TimerTriggerType::AutoExpire, 0).unwrap();
        });

        env.as_contract(&cid, || {
            // First trigger fires the timer and routes through the AutoExpire action.
            assert_eq!(trigger_timers(&env, &caller, 1), 1);
            let grant = Storage::get_grant(&env, 1).unwrap();
            assert_eq!(grant.status, GrantStatus::Cancelled);
            assert_eq!(
                grant.reason,
                Some(String::from_str(&env, "auto-expired by timer"))
            );

            // Second trigger is a no-op — the timer.fired guard holds.
            assert_eq!(trigger_timers(&env, &caller, 1), 0);

            let timers = get_timers(&env, 1);
            assert_eq!(timers.len(), 1);
            let t = timers.get(0).unwrap();
            assert!(t.fired);
            assert_eq!(t.triggered_by, Some(caller.clone()));
        });
    }

    #[test]
    fn duplicate_unfired_registration_is_rejected() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 1000, 1000, 0, 3);
        });
        env.as_contract(&cid, || {
            register_timer(&env, &owner, 1, TimerTriggerType::AutoActivate, 100).unwrap();
        });
        env.as_contract(&cid, || {
            let err = register_timer(&env, &owner, 1, TimerTriggerType::AutoActivate, 200);
            assert_eq!(err, Err(ContractError::InvalidInput));
        });
        env.as_contract(&cid, || {
            // A different trigger type is still allowed.
            register_timer(&env, &owner, 1, TimerTriggerType::AutoReleaseLockup, 0).unwrap();
            assert_eq!(get_timers(&env, 1).len(), 2);
        });
    }

    #[test]
    fn re_registration_allowed_after_timer_fires() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);

        // AutoActivate is eligible (Active + fully funded) and its action is a no-op,
        // so the grant stays Active and we can observe a second registration.
        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 1000, 1000, 0, 3);
        });
        env.as_contract(&cid, || {
            register_timer(&env, &owner, 1, TimerTriggerType::AutoActivate, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 1), 1);
        });
        env.as_contract(&cid, || {
            register_timer(&env, &owner, 1, TimerTriggerType::AutoActivate, 0).unwrap();
            assert_eq!(get_timers(&env, 1).len(), 2);
        });
    }

    #[test]
    fn auto_expire_eligibility_gate() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);

        // Not eligible: all milestones already paid out.
        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 0, 1000, 2, 2);
            register_timer(&env, &owner, 1, TimerTriggerType::AutoExpire, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 1), 0);
            assert!(!get_timers(&env, 1).get(0).unwrap().fired);
        });

        // Eligible: milestones outstanding.
        env.as_contract(&cid, || {
            setup_grant(&env, 2, &owner, GrantStatus::Active, 0, 1000, 0, 2);
            register_timer(&env, &owner, 2, TimerTriggerType::AutoExpire, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 2), 1);
        });
    }

    #[test]
    fn auto_activate_eligibility_gate() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 500, 1000, 0, 3);
            register_timer(&env, &owner, 1, TimerTriggerType::AutoActivate, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 1), 0);
        });

        env.as_contract(&cid, || {
            setup_grant(&env, 2, &owner, GrantStatus::Active, 1000, 1000, 0, 3);
            register_timer(&env, &owner, 2, TimerTriggerType::AutoActivate, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 2), 1);
        });
    }

    #[test]
    fn auto_cancel_eligibility_gate_and_cleanup() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 1, 1000, 0, 3);
            register_timer(&env, &owner, 1, TimerTriggerType::AutoCancel, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 1), 0);
        });

        env.as_contract(&cid, || {
            setup_grant(&env, 2, &owner, GrantStatus::Active, 0, 1000, 0, 3);
            register_timer(&env, &owner, 2, TimerTriggerType::AutoCancel, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 2), 1);
            let grant = Storage::get_grant(&env, 2).unwrap();
            assert_eq!(grant.status, GrantStatus::Cancelled);
            assert_eq!(
                grant.reason,
                Some(String::from_str(
                    &env,
                    "auto-cancelled: not funded by deadline"
                ))
            );
        });
    }

    #[test]
    fn auto_release_lockup_eligibility_gate() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);

        // Not eligible: grant not Active.
        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Cancelled, 0, 1000, 0, 3);
            register_timer(&env, &owner, 1, TimerTriggerType::AutoReleaseLockup, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 1), 0);
        });

        env.as_contract(&cid, || {
            setup_grant(&env, 2, &owner, GrantStatus::Active, 0, 1000, 0, 3);
            register_timer(&env, &owner, 2, TimerTriggerType::AutoReleaseLockup, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 2), 1);
        });
    }

    #[test]
    fn custom_callback_is_always_eligible() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Cancelled, 0, 0, 5, 1);
            register_timer(&env, &owner, 1, TimerTriggerType::CustomCallback, 0).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 1), 1);
        });
    }

    #[test]
    fn timer_does_not_fire_before_fires_at() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let caller = Address::generate(&env);
        env.ledger().set_timestamp(1_000);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 0, 1000, 0, 3);
            register_timer(&env, &owner, 1, TimerTriggerType::AutoExpire, 5_000).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(trigger_timers(&env, &caller, 1), 0);
            assert_eq!(pending_timers(&env, 1).len(), 0);
        });

        env.ledger().set_timestamp(5_001);
        env.as_contract(&cid, || {
            assert_eq!(pending_timers(&env, 1).len(), 1);
            assert_eq!(trigger_timers(&env, &caller, 1), 1);
        });
    }

    #[test]
    fn cancel_timer_lifecycle() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 0, 1000, 0, 3);
            register_timer(&env, &owner, 1, TimerTriggerType::AutoExpire, 100).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(
                cancel_timer(&env, &stranger, 1, TimerTriggerType::AutoExpire),
                Err(ContractError::Unauthorized)
            );
        });
        env.as_contract(&cid, || {
            cancel_timer(&env, &owner, 1, TimerTriggerType::AutoExpire).unwrap();
            assert_eq!(get_timers(&env, 1).len(), 0);
        });
        env.as_contract(&cid, || {
            assert_eq!(
                cancel_timer(&env, &owner, 1, TimerTriggerType::AutoExpire),
                Err(ContractError::TimerNotFound)
            );
        });
    }

    #[test]
    fn register_and_trigger_on_missing_grant() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);

        env.as_contract(&cid, || {
            assert_eq!(
                register_timer(&env, &owner, 99, TimerTriggerType::AutoExpire, 0),
                Err(ContractError::GrantNotFound)
            );
            assert_eq!(trigger_timers(&env, &owner, 99), 0);
        });
    }

    #[test]
    fn only_owner_or_admin_can_register() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let admin = Address::generate(&env);
        let stranger = Address::generate(&env);

        env.as_contract(&cid, || {
            setup_grant(&env, 1, &owner, GrantStatus::Active, 0, 1000, 0, 3);
            Storage::set_global_admin(&env, &admin);
        });
        env.as_contract(&cid, || {
            assert_eq!(
                register_timer(&env, &stranger, 1, TimerTriggerType::AutoExpire, 0),
                Err(ContractError::Unauthorized)
            );
        });
        env.as_contract(&cid, || {
            register_timer(&env, &admin, 1, TimerTriggerType::AutoExpire, 0).unwrap();
            assert_eq!(get_timers(&env, 1).len(), 1);
        });
    }
}
