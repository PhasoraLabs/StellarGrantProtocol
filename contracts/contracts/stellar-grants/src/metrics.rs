use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ProtocolMetrics, TokenMetric};

// ── Metric field selector ─────────────────────────────────────────────────────

/// Internal enum for selecting which counter to increment.
#[derive(Clone, Copy)]
pub enum MetricField {
    GrantsCreated,
    GrantsActive,
    GrantsCompleted,
    GrantsCancelled,
    MilestonesApproved,
    MilestonesRejected,
    MilestonesPaid,
    ContributorsRegistered,
    DisputesRaised,
    DisputesResolved,
    BountiesCreated,
    BountiesAwarded,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_metrics(env: &Env) -> ProtocolMetrics {
    ProtocolMetrics {
        total_grants_created: 0,
        total_grants_active: 0,
        total_grants_completed: 0,
        total_grants_cancelled: 0,
        total_milestones_approved: 0,
        total_milestones_rejected: 0,
        total_milestones_paid: 0,
        total_contributors_registered: 0,
        total_disputes_raised: 0,
        total_disputes_resolved: 0,
        total_bounties_created: 0,
        total_bounties_awarded: 0,
        last_updated: env.ledger().timestamp(),
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Return the current protocol metrics snapshot.
pub fn get_metrics(env: &Env) -> ProtocolMetrics {
    Storage::get_protocol_metrics(env).unwrap_or_else(|| default_metrics(env))
}

/// Return token-specific financial metrics for a given token.
pub fn get_token_metrics(env: &Env, token: &Address) -> TokenMetric {
    Storage::get_token_metrics(env, token).unwrap_or(TokenMetric {
        token: token.clone(),
        total_locked: 0,
        total_paid_out: 0,
        total_refunded: 0,
    })
}

/// Increment a specific metric counter by `delta`. Infallible — never panics.
pub fn increment(env: &Env, field: MetricField, delta: u32) {
    let mut m = get_metrics(env);
    match field {
        MetricField::GrantsCreated => {
            m.total_grants_created = m.total_grants_created.saturating_add(delta);
        }
        MetricField::GrantsActive => {
            m.total_grants_active = m.total_grants_active.saturating_add(delta);
        }
        MetricField::GrantsCompleted => {
            m.total_grants_completed = m.total_grants_completed.saturating_add(delta);
            m.total_grants_active = m.total_grants_active.saturating_sub(delta);
        }
        MetricField::GrantsCancelled => {
            m.total_grants_cancelled = m.total_grants_cancelled.saturating_add(delta);
            m.total_grants_active = m.total_grants_active.saturating_sub(delta);
        }
        MetricField::MilestonesApproved => {
            m.total_milestones_approved = m.total_milestones_approved.saturating_add(delta);
        }
        MetricField::MilestonesRejected => {
            m.total_milestones_rejected = m.total_milestones_rejected.saturating_add(delta);
        }
        MetricField::MilestonesPaid => {
            m.total_milestones_paid = m.total_milestones_paid.saturating_add(delta);
        }
        MetricField::ContributorsRegistered => {
            m.total_contributors_registered = m.total_contributors_registered.saturating_add(delta);
        }
        MetricField::DisputesRaised => {
            m.total_disputes_raised = m.total_disputes_raised.saturating_add(delta);
        }
        MetricField::DisputesResolved => {
            m.total_disputes_resolved = m.total_disputes_resolved.saturating_add(delta);
        }
        MetricField::BountiesCreated => {
            m.total_bounties_created = m.total_bounties_created.saturating_add(delta);
        }
        MetricField::BountiesAwarded => {
            m.total_bounties_awarded = m.total_bounties_awarded.saturating_add(delta);
        }
    }
    m.last_updated = env.ledger().timestamp();
    Storage::set_protocol_metrics(env, &m);
}

/// Update token locked amount (positive = deposit, negative = release/refund).
/// Uses saturating arithmetic — never panics.
pub fn update_token_locked(env: &Env, token: &Address, delta: i128) {
    let mut tm = get_token_metrics(env, token);
    if delta > 0 {
        tm.total_locked = tm.total_locked.saturating_add(delta);
    } else {
        let decrease = delta.saturating_abs();
        if decrease > tm.total_locked {
            // Released more than locked (shouldn't happen in normal flow, but safe).
            tm.total_paid_out = tm.total_paid_out.saturating_add(decrease - tm.total_locked);
            tm.total_locked = 0;
        } else {
            tm.total_locked = tm.total_locked.saturating_sub(decrease);
            tm.total_paid_out = tm.total_paid_out.saturating_add(decrease);
        }
    }
    Storage::set_token_metrics(env, &tm);
}

/// Track a refund in token metrics.
pub fn record_token_refund(env: &Env, token: &Address, amount: i128) {
    let mut tm = get_token_metrics(env, token);
    tm.total_locked = tm.total_locked.saturating_sub(amount);
    tm.total_refunded = tm.total_refunded.saturating_add(amount);
    Storage::set_token_metrics(env, &tm);
}

/// Reset all protocol metrics to zero. Admin only (for testnet/migration use).
pub fn reset(env: &Env, admin: &Address) -> Result<(), ContractError> {
    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    Storage::set_protocol_metrics(env, &default_metrics(env));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> Env {
        Env::default()
    }

    #[test]
    fn test_default_metrics_are_zero() {
        let env = setup();
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_created, 0);
        assert_eq!(m.total_grants_active, 0);
        assert_eq!(m.total_grants_completed, 0);
        assert_eq!(m.total_grants_cancelled, 0);
        assert_eq!(m.total_milestones_approved, 0);
        assert_eq!(m.total_milestones_rejected, 0);
        assert_eq!(m.total_milestones_paid, 0);
        assert_eq!(m.total_contributors_registered, 0);
        assert_eq!(m.total_disputes_raised, 0);
        assert_eq!(m.total_disputes_resolved, 0);
        assert_eq!(m.total_bounties_created, 0);
        assert_eq!(m.total_bounties_awarded, 0);
    }

    #[test]
    fn test_increment_grants_created() {
        let env = setup();
        increment(&env, MetricField::GrantsCreated, 1);
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_created, 1);

        increment(&env, MetricField::GrantsCreated, 3);
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_created, 4);
    }

    #[test]
    fn test_increment_grants_active() {
        let env = setup();
        increment(&env, MetricField::GrantsActive, 2);
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_active, 2);
    }

    #[test]
    fn test_grants_completed_decrements_active() {
        let env = setup();
        increment(&env, MetricField::GrantsActive, 5);
        increment(&env, MetricField::GrantsCompleted, 2);
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_completed, 2);
        assert_eq!(m.total_grants_active, 3);
    }

    #[test]
    fn test_grants_cancelled_decrements_active() {
        let env = setup();
        increment(&env, MetricField::GrantsActive, 5);
        increment(&env, MetricField::GrantsCancelled, 1);
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_cancelled, 1);
        assert_eq!(m.total_grants_active, 4);
    }

    #[test]
    fn test_milestone_counters() {
        let env = setup();
        increment(&env, MetricField::MilestonesApproved, 3);
        increment(&env, MetricField::MilestonesRejected, 1);
        increment(&env, MetricField::MilestonesPaid, 2);
        let m = get_metrics(&env);
        assert_eq!(m.total_milestones_approved, 3);
        assert_eq!(m.total_milestones_rejected, 1);
        assert_eq!(m.total_milestones_paid, 2);
    }

    #[test]
    fn test_dispute_counters() {
        let env = setup();
        increment(&env, MetricField::DisputesRaised, 5);
        increment(&env, MetricField::DisputesResolved, 3);
        let m = get_metrics(&env);
        assert_eq!(m.total_disputes_raised, 5);
        assert_eq!(m.total_disputes_resolved, 3);
    }

    #[test]
    fn test_bounty_counters() {
        let env = setup();
        increment(&env, MetricField::BountiesCreated, 10);
        increment(&env, MetricField::BountiesAwarded, 7);
        let m = get_metrics(&env);
        assert_eq!(m.total_bounties_created, 10);
        assert_eq!(m.total_bounties_awarded, 7);
    }

    #[test]
    fn test_contributors_counter() {
        let env = setup();
        increment(&env, MetricField::ContributorsRegistered, 4);
        let m = get_metrics(&env);
        assert_eq!(m.total_contributors_registered, 4);
    }

    #[test]
    fn test_saturating_add_never_panics() {
        let env = setup();
        // Set to near max
        let mut m = get_metrics(&env);
        m.total_grants_created = u32::MAX - 1;
        Storage::set_protocol_metrics(&env, &m);

        increment(&env, MetricField::GrantsCreated, 5);
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_created, u32::MAX);
    }

    #[test]
    fn test_saturating_sub_never_panics_on_completed() {
        let env = setup();
        // Active is 0, completing 1 should saturate to 0
        increment(&env, MetricField::GrantsCompleted, 1);
        let m = get_metrics(&env);
        assert_eq!(m.total_grants_completed, 1);
        assert_eq!(m.total_grants_active, 0);
    }

    #[test]
    fn test_multi_action_sequence() {
        let env = setup();
        increment(&env, MetricField::GrantsCreated, 3);
        increment(&env, MetricField::GrantsActive, 3);
        increment(&env, MetricField::ContributorsRegistered, 5);
        increment(&env, MetricField::MilestonesApproved, 6);
        increment(&env, MetricField::MilestonesPaid, 4);
        increment(&env, MetricField::DisputesRaised, 1);
        increment(&env, MetricField::DisputesResolved, 1);
        increment(&env, MetricField::GrantsCompleted, 1);
        increment(&env, MetricField::GrantsCancelled, 1);

        let m = get_metrics(&env);
        assert_eq!(m.total_grants_created, 3);
        assert_eq!(m.total_grants_active, 1); // 3 - 1 - 1
        assert_eq!(m.total_grants_completed, 1);
        assert_eq!(m.total_grants_cancelled, 1);
        assert_eq!(m.total_milestones_approved, 6);
        assert_eq!(m.total_milestones_paid, 4);
        assert_eq!(m.total_disputes_raised, 1);
        assert_eq!(m.total_disputes_resolved, 1);
        assert_eq!(m.total_contributors_registered, 5);
    }

    #[test]
    fn test_token_metrics_default() {
        let env = setup();
        let token = Address::generate(&env);
        let tm = get_token_metrics(&env, &token);
        assert_eq!(tm.total_locked, 0);
        assert_eq!(tm.total_paid_out, 0);
        assert_eq!(tm.total_refunded, 0);
    }

    #[test]
    fn test_update_token_locked_positive() {
        let env = setup();
        let token = Address::generate(&env);
        update_token_locked(&env, &token, 1000);
        let tm = get_token_metrics(&env, &token);
        assert_eq!(tm.total_locked, 1000);
        assert_eq!(tm.total_paid_out, 0);
    }

    #[test]
    fn test_update_token_locked_negative() {
        let env = setup();
        let token = Address::generate(&env);
        update_token_locked(&env, &token, 1000);
        update_token_locked(&env, &token, -400);
        let tm = get_token_metrics(&env, &token);
        assert_eq!(tm.total_locked, 600);
        assert_eq!(tm.total_paid_out, 400);
    }

    #[test]
    fn test_record_token_refund() {
        let env = setup();
        let token = Address::generate(&env);
        update_token_locked(&env, &token, 1000);
        record_token_refund(&env, &token, 300);
        let tm = get_token_metrics(&env, &token);
        assert_eq!(tm.total_locked, 700);
        assert_eq!(tm.total_refunded, 300);
    }

    #[test]
    fn test_reset_clears_all() {
        let env = setup();
        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        increment(&env, MetricField::GrantsCreated, 10);
        increment(&env, MetricField::DisputesRaised, 5);

        reset(&env, &admin).unwrap();

        let m = get_metrics(&env);
        assert_eq!(m.total_grants_created, 0);
        assert_eq!(m.total_disputes_raised, 0);
    }

    #[test]
    fn test_reset_unauthorized() {
        let env = setup();
        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);

        let other = Address::generate(&env);
        assert!(reset(&env, &other).is_err());
    }
}
