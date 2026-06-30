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
