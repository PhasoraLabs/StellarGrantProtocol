use soroban_sdk::{contractevent, Address, Env, Map, String, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ExtensionRequest, ExtensionStatus};

// ── Events ──────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionRequested {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub requested_by: Address,
    pub new_deadline: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionApproved {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub new_deadline: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionDenied {
    pub grant_id: u64,
    pub milestone_idx: u32,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionWithdrawn {
    pub grant_id: u64,
    pub milestone_idx: u32,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Contributor (grant owner) requests a deadline extension for a milestone.
pub fn request_extension(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    milestone_idx: u32,
    new_deadline: u64,
    reason: String,
) -> Result<(), ContractError> {
    contributor.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }

    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;

    // Only one pending request per milestone at a time.
    if let Some(existing) = Storage::get_extension_request(env, grant_id, milestone_idx) {
        if existing.status == ExtensionStatus::Pending {
            return Err(ContractError::InvalidState);
        }
    }

    let original_deadline = milestone.deadline.unwrap_or(0);
    let now = env.ledger().timestamp();
    if new_deadline <= now || new_deadline <= original_deadline {
        return Err(ContractError::InvalidInput);
    }

    let request = ExtensionRequest {
        grant_id,
        milestone_idx,
        requested_by: contributor.clone(),
        original_deadline,
        new_deadline,
        reason,
        status: ExtensionStatus::Pending,
        votes_approve: 0,
        votes_deny: 0,
        reviewer_votes: Map::new(env),
        requested_at: now,
        resolved_at: None,
    };
    Storage::set_extension_request(env, &request);

    ExtensionRequested {
        grant_id,
        milestone_idx,
        requested_by: contributor.clone(),
        new_deadline,
    }
    .publish(env);

    Ok(())
}

/// Reviewer votes on an extension request. Returns the resulting status.
pub fn vote_extension(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    milestone_idx: u32,
    approve: bool,
) -> Result<ExtensionStatus, ContractError> {
    reviewer.require_auth();

    let mut request = Storage::get_extension_request(env, grant_id, milestone_idx)
        .ok_or(ContractError::ExtensionRequestNotFound)?;

    if request.status != ExtensionStatus::Pending {
        return Err(ContractError::ExtensionAlreadyResolved);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    // Check eligibility against the reviewer list snapshot taken at submission time (#1145).
    // This prevents reviewers added mid-vote from voting on the extension request.
    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;
    let eligible_reviewers = if milestone.reviewer_list_snapshot.is_empty() {
        &grant.reviewers
    } else {
        &milestone.reviewer_list_snapshot
    };
    if !eligible_reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }
    if request.reviewer_votes.contains_key(reviewer.clone()) {
        return Err(ContractError::AlreadyVoted);
    }

    request.reviewer_votes.set(reviewer.clone(), approve);
    if approve {
        request.votes_approve = request.votes_approve.saturating_add(1);
    } else {
        request.votes_deny = request.votes_deny.saturating_add(1);
    }

    let total_reviewers = grant.reviewers.len();
    let majority = total_reviewers / 2 + 1;

    if request.votes_approve >= majority {
        // Approved: update milestone deadline without touching vote/submission state.
        request.status = ExtensionStatus::Approved;
        request.resolved_at = Some(env.ledger().timestamp());

        let mut milestone = Storage::get_milestone(env, grant_id, milestone_idx)
            .ok_or(ContractError::MilestoneNotFound)?;
        milestone.deadline = Some(request.new_deadline);
        Storage::set_milestone(env, grant_id, milestone_idx, &milestone);

        Storage::set_extension_request(env, &request);
        Storage::push_extension_history(env, &request);

        ExtensionApproved {
            grant_id,
            milestone_idx,
            new_deadline: request.new_deadline,
        }
        .publish(env);
    } else if request.votes_deny >= majority {
        request.status = ExtensionStatus::Denied;
        request.resolved_at = Some(env.ledger().timestamp());

        Storage::set_extension_request(env, &request);
        Storage::push_extension_history(env, &request);

        ExtensionDenied {
            grant_id,
            milestone_idx,
        }
        .publish(env);
    } else {
        Storage::set_extension_request(env, &request);
    }

    Ok(request.status)
}

/// Contributor withdraws a pending extension request.
pub fn withdraw_request(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    contributor.require_auth();

    let mut request = Storage::get_extension_request(env, grant_id, milestone_idx)
        .ok_or(ContractError::ExtensionRequestNotFound)?;

    if request.status != ExtensionStatus::Pending {
        return Err(ContractError::ExtensionAlreadyResolved);
    }
    if request.requested_by != *contributor {
        return Err(ContractError::Unauthorized);
    }

    request.status = ExtensionStatus::Withdrawn;
    request.resolved_at = Some(env.ledger().timestamp());
    Storage::push_extension_history(env, &request);
    Storage::remove_extension_request(env, grant_id, milestone_idx);

    ExtensionWithdrawn {
        grant_id,
        milestone_idx,
    }
    .publish(env);

    Ok(())
}

/// Return the current extension request for a milestone.
pub fn get_request(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<ExtensionRequest> {
    Storage::get_extension_request(env, grant_id, milestone_idx)
}

/// Check if a milestone is currently past its deadline (considering any approved extension).
pub fn is_overdue(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    match Storage::get_milestone(env, grant_id, milestone_idx) {
        Some(m) => match m.deadline {
            Some(d) => env.ledger().timestamp() > d,
            None => false,
        },
        None => false,
    }
}

/// Return all resolved extension requests for a grant (including historical).
pub fn get_extension_history(env: &Env, grant_id: u64) -> Vec<ExtensionRequest> {
    Storage::get_extension_history(env, grant_id)
}
