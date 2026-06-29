use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::governance;
use crate::storage::Storage;
use crate::types::{
    AutoApproveConfig, AutoApproveRecord, MilestoneState, VoteResult,
};

/// Configure auto-approve for a grant. Owner only.
pub fn set_config(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    config: AutoApproveConfig,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    Storage::set_auto_approve_config(env, grant_id, &config);
    Ok(())
}

/// Attempt auto-approve for a milestone. Anyone may call; enforces all conditions.
pub fn try_auto_approve(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<bool, ContractError> {
    let config =
        Storage::get_auto_approve_config(env, grant_id).ok_or(ContractError::AutoApproveNotEnabled)?;

    if !config.enabled {
        return Err(ContractError::AutoApproveNotEnabled);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    let milestone =
        Storage::get_milestone(env, grant_id, milestone_idx).ok_or(ContractError::MilestoneNotFound)?;

    if milestone.state != MilestoneState::Submitted {
        return Ok(false);
    }

    if let Some(existing) = Storage::get_auto_approve_record(env, grant_id, milestone_idx) {
        let _ = existing;
        return Ok(false);
    }

    let now = env.ledger().timestamp();
    let submission_time = milestone.submission_timestamp;
    let deadline = milestone.deadline.unwrap_or(0);

    let effective_deadline = if deadline > 0 { deadline } else { submission_time };
    let grace_end = effective_deadline.saturating_add(config.grace_period_seconds);

    if now < grace_end {
        return Err(ContractError::AutoApproveGracePeriodNotPassed);
    }

    let votes_cast = milestone.approvals + milestone.rejections;
    if votes_cast < config.min_votes_required {
        return Err(ContractError::AutoApproveInsufficientVotes);
    }

    let mut grant = Storage::get_grant_v(env, grant_id);
    let mut milestone = Storage::get_milestone_v(env, grant_id, milestone_idx);

    let vote_result = VoteResult {
        approved: true,
        quorum_reached: true,
        approval_pct: 100,
    };

    governance::finalize_milestone(&mut milestone, &vote_result);
    Storage::set_milestone(env, grant_id, milestone_idx, &milestone);

    let record = AutoApproveRecord {
        grant_id,
        milestone_idx,
        triggered_by: caller.clone(),
        triggered_at: now,
        votes_at_trigger: votes_cast,
    };
    Storage::set_auto_approve_record(env, grant_id, milestone_idx, &record);

    crate::events::Events::milestone_status_changed(
        env,
        grant_id,
        milestone_idx,
        MilestoneState::Approved,
    );

    Ok(true)
}

/// Return whether auto-approve conditions are currently met for a milestone.
pub fn can_auto_approve(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    let config = match Storage::get_auto_approve_config(env, grant_id) {
        Some(c) if c.enabled => c,
        _ => return false,
    };

    let grant = match Storage::get_grant(env, grant_id) {
        Some(g) => g,
        None => return false,
    };

    if milestone_idx >= grant.total_milestones {
        return false;
    }

    let milestone = match Storage::get_milestone(env, grant_id, milestone_idx) {
        Some(m) => m,
        None => return false,
    };

    if milestone.state != MilestoneState::Submitted {
        return false;
    }

    if Storage::get_auto_approve_record(env, grant_id, milestone_idx).is_some() {
        return false;
    }

    let now = env.ledger().timestamp();
    let submission_time = milestone.submission_timestamp;
    let deadline = milestone.deadline.unwrap_or(0);
    let effective_deadline = if deadline > 0 { deadline } else { submission_time };
    let grace_end = effective_deadline.saturating_add(config.grace_period_seconds);

    if now < grace_end {
        return false;
    }

    let votes_cast = milestone.approvals + milestone.rejections;
    votes_cast >= config.min_votes_required
}

/// Return the auto-approve config for a grant.
pub fn get_config(env: &Env, grant_id: u64) -> Option<AutoApproveConfig> {
    Storage::get_auto_approve_config(env, grant_id)
}

/// Return the auto-approve record if it was triggered.
pub fn get_record(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<AutoApproveRecord> {
    Storage::get_auto_approve_record(env, grant_id, milestone_idx)
}
