use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::grant_index;
use crate::storage::Storage;
use crate::types::{
    DashboardView, ExportGrant, GrantDetailView, Milestone, MilestoneState, ProtocolMetrics,
    ReviewerProfile, ReviewerView,
};

const MAX_MULTI_GRANT_BATCH: u32 = 10;

fn build_export_grant(grant: &crate::types::Grant) -> ExportGrant {
    ExportGrant {
        id: grant.id,
        owner: grant.owner.clone(),
        title: grant.title.clone(),
        description: grant.description.clone(),
        token: grant.token.clone(),
        status: grant.status,
        total_amount: grant.total_amount,
        milestone_amount: grant.milestone_amount,
        total_milestones: grant.total_milestones,
        milestones_paid_out: grant.milestones_paid_out,
        escrow_balance: grant.escrow_balance,
        timestamp: grant.timestamp,
    }
}

fn compute_completion_pct(approved: u32, total: u32) -> u32 {
    if total == 0 {
        return 0;
    }
    (approved * 100) / total
}

/// Return all data needed for the grant detail page. Single RPC call.
pub fn grant_detail(env: &Env, grant_id: u64) -> Result<GrantDetailView, ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    let mut milestones = Vec::new(env);
    let mut approved_count: u32 = 0;
    let mut current_milestone_idx: u32 = 0;

    for idx in 0..grant.total_milestones {
        if let Some(ms) = Storage::get_milestone(env, grant_id, idx) {
            if ms.state == MilestoneState::Approved {
                approved_count += 1;
            }
            if ms.state == MilestoneState::Submitted {
                current_milestone_idx = idx;
            }
            milestones.push_back(ms);
        } else {
            milestones.push_back(Milestone {
                idx,
                description: soroban_sdk::String::from_str(&env, ""),
                amount: grant.milestone_amount,
                state: MilestoneState::Pending,
                votes: soroban_sdk::Map::new(env),
                approvals: 0,
                rejections: 0,
                reasons: soroban_sdk::Map::new(env),
                status_updated_at: 0,
                proof_url: None,
                submission_timestamp: 0,
                deadline: None,
                reviewer_count_snapshot: 0,
            });
        }
    }

    let completion_pct = compute_completion_pct(approved_count, grant.total_milestones);

    let funder_count = grant.funders.len();
    let reviewer_count = grant.reviewers.len();
    let escrow_balance = grant.escrow_balance;

    let mut reputation_scores = Vec::new(env);
    for reviewer in grant.reviewers.iter() {
        let rep = Storage::get_reviewer_reputation(env, reviewer.clone());
        reputation_scores.push_back((reviewer, rep));
    }

    Ok(GrantDetailView {
        grant,
        milestones,
        escrow_balance,
        funder_count,
        reviewer_count,
        current_milestone_idx,
        completion_pct,
        reputation_scores,
    })
}

/// Return all data needed for the protocol dashboard. Single RPC call.
pub fn dashboard(env: &Env) -> DashboardView {
    let active_grant_ids =
        grant_index::by_status(env, crate::types::GrantStatus::Active, 0, 1000);
    let active_grants = active_grant_ids.len();

    let protocol_metrics = get_or_default_metrics(env);

    let recent_grant_ids = grant_index::recent(env, 0, 10);

    let mut total_funded_usd: i128 = 0;
    let mut total_paid_out_usd: i128 = 0;

    for grant_id in active_grant_ids.iter() {
        if let Some(grant) = Storage::get_grant(env, grant_id) {
            total_funded_usd += grant.escrow_balance;
            total_paid_out_usd += grant.milestone_amount * grant.milestones_paid_out as i128;
        }
    }

    let contributor_count = protocol_metrics.total_contributors_registered;
    let reviewer_count = Storage::get_reviewer_allowlist(env).len();

    DashboardView {
        active_grants,
        total_funded_usd,
        total_paid_out_usd,
        total_contributors: contributor_count,
        total_reviewers: reviewer_count,
        recent_grant_ids,
        protocol_metrics,
    }
}

/// Return all data needed for a reviewer's personal dashboard.
pub fn reviewer_dashboard(env: &Env, reviewer: &Address) -> ReviewerView {
    let profile = Storage::get_reviewer_profile(env, reviewer).unwrap_or(ReviewerProfile {
        reviewer: reviewer.clone(),
        display_name: soroban_sdk::String::from_str(env, ""),
        expertise_tags: soroban_sdk::Vec::new(env),
        hourly_rate: None,
        reviews_completed: 0,
        average_turnaround_ledgers: 0,
        availability: crate::types::ReviewerAvailability::Available,
        registered_at: 0,
        reputation_score: 0,
    });

    let reputation = Storage::get_reviewer_reputation(env, reviewer.clone());

    let mut pending_votes = soroban_sdk::Vec::new(env);

    let active_grants = grant_index::by_status(env, crate::types::GrantStatus::Active, 0, 100);
    for grant_id in active_grants.iter() {
        if let Some(grant) = Storage::get_grant(env, grant_id) {
            if !grant.reviewers.contains(reviewer.clone()) {
                continue;
            }
            for idx in 0..grant.total_milestones {
                if let Some(ms) = Storage::get_milestone(env, grant_id, idx) {
                    if ms.state == MilestoneState::Submitted && !ms.votes.contains_key(reviewer.clone()) {
                        pending_votes.push_back((grant_id, idx));
                    }
                }
            }
        }
    }

    ReviewerView {
        reviewer: reviewer.clone(),
        profile,
        reputation,
        pending_votes,
        sla_breach_count: 0,
        pending_rewards: 0,
    }
}

/// Return detail views for multiple grants at once (max 10).
pub fn multi_grant_detail(
    env: &Env,
    grant_ids: Vec<u64>,
) -> Result<Vec<GrantDetailView>, ContractError> {
    if grant_ids.len() > MAX_MULTI_GRANT_BATCH {
        return Err(ContractError::BatchSizeExceeded);
    }

    let mut views = Vec::new(env);
    for grant_id in grant_ids.iter() {
        views.push_back(grant_detail(env, grant_id)?);
    }
    Ok(views)
}

/// Return minimal grant cards for a list of grant IDs (cheaper than full detail).
pub fn grant_cards(env: &Env, grant_ids: Vec<u64>) -> Vec<ExportGrant> {
    let mut cards = Vec::new(env);
    for grant_id in grant_ids.iter() {
        if let Some(grant) = Storage::get_grant(env, grant_id) {
            cards.push_back(build_export_grant(&grant));
        }
    }
    cards
}

fn get_or_default_metrics(env: &Env) -> ProtocolMetrics {
    Storage::get_protocol_metrics(env).unwrap_or(ProtocolMetrics {
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
        last_updated: 0,
    })
}
