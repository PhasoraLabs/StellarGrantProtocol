use soroban_sdk::{contractevent, Address, Env, Map, String, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ExtensionRequest, ExtensionStatus, MilestoneState};

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

    // Issue #953: Reject if milestone is already in a terminal state
    if matches!(
        milestone.state,
        MilestoneState::Approved | MilestoneState::Rejected | MilestoneState::Paid
    ) {
        return Err(ContractError::InvalidState);
    }

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
    if !grant.reviewers.contains(reviewer.clone()) {
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

    // Issue #952: Use the same reviewer_count_snapshot that milestone approval uses
    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;
    let total_reviewers = milestone.reviewer_count_snapshot as usize;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{Grant, GrantStatus, Milestone, MilestoneState};
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Env, Map, String, Vec};

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let reviewer1 = Address::generate(&env);
        let reviewer2 = Address::generate(&env);
        (env, owner, reviewer1, reviewer2)
    }

    fn create_grant_with_milestone(
        env: &Env,
        owner: &Address,
        reviewers: &Vec<Address>,
        grant_id: u64,
        deadline: u64,
    ) {
        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Test"),
            description: String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: reviewers.clone(),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 1000,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);

        let milestone = Milestone {
            idx: 0,
            description: String::from_str(env, "MS1"),
            amount: 500,
            state: MilestoneState::Submitted,
            votes: Map::new(env),
            approvals: 0,
            rejections: 0,
            reasons: Map::new(env),
            status_updated_at: 0,
            proof_url: Some(String::from_str(env, "https://proof")),
            submission_timestamp: env.ledger().timestamp(),
            deadline: Some(deadline),
            reviewer_count_snapshot: reviewers.len() as u32,
        };
        Storage::set_milestone(env, grant_id, 0, &milestone);
    }

    #[test]
    fn test_request_extension() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);

        request_extension(
            &env,
            &owner,
            1,
            0,
            2000,
            String::from_str(&env, "Need more time"),
        )
        .unwrap();

        let req = get_request(&env, 1, 0).unwrap();
        assert_eq!(req.status, ExtensionStatus::Pending);
        assert_eq!(req.new_deadline, 2000);
        assert_eq!(req.original_deadline, 1000);
    }

    #[test]
    fn test_get_request_returns_none_when_no_request() {
        let (env, _, _, _) = setup();
        assert!(get_request(&env, 999, 0).is_none());
    }

    #[test]
    #[should_panic]
    fn test_request_extension_unauthorized() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);

        let other = Address::generate(&env);
        request_extension(&env, &other, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_request_extension_deadline_not_in_future() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);

        // new_deadline <= now
        request_extension(&env, &owner, 1, 0, 400, String::from_str(&env, "reason")).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_request_extension_duplicate_pending() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);

        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "r1")).unwrap();
        // Second request on same milestone while first is pending
        request_extension(&env, &owner, 1, 0, 3000, String::from_str(&env, "r2")).unwrap();
    }

    #[test]
    fn test_vote_extension_approve() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);
        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();

        // First approve vote (need majority = 2/2 + 1 = 2)
        let status = vote_extension(&env, &r1, 1, 0, true).unwrap();
        assert_eq!(status, ExtensionStatus::Pending); // Not yet majority

        // Second approve vote reaches majority
        let status = vote_extension(&env, &r2, 1, 0, true).unwrap();
        assert_eq!(status, ExtensionStatus::Approved);

        // Verify deadline was updated
        let milestone = Storage::get_milestone(&env, 1, 0).unwrap();
        assert_eq!(milestone.deadline, Some(2000));
    }

    #[test]
    fn test_vote_extension_deny() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);
        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();

        vote_extension(&env, &r1, 1, 0, false).unwrap();
        let status = vote_extension(&env, &r2, 1, 0, false).unwrap();
        assert_eq!(status, ExtensionStatus::Denied);

        // Deadline should NOT change
        let milestone = Storage::get_milestone(&env, 1, 0).unwrap();
        assert_eq!(milestone.deadline, Some(1000));
    }

    #[test]
    #[should_panic]
    fn test_vote_extension_double_vote() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);
        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();

        vote_extension(&env, &r1, 1, 0, true).unwrap();
        vote_extension(&env, &r1, 1, 0, true).unwrap(); // double vote
    }

    #[test]
    #[should_panic]
    fn test_vote_extension_non_reviewer() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);
        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();

        let rando = Address::generate(&env);
        vote_extension(&env, &rando, 1, 0, true).unwrap();
    }

    #[test]
    fn test_withdraw_request() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);
        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();

        withdraw_request(&env, &owner, 1, 0).unwrap();

        // Request should be removed
        assert!(get_request(&env, 1, 0).is_none());
    }

    #[test]
    #[should_panic]
    fn test_withdraw_request_unauthorized() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);
        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();

        let other = Address::generate(&env);
        withdraw_request(&env, &other, 1, 0).unwrap();
    }

    #[test]
    fn test_is_overdue() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        // Before deadline
        env.ledger().with_mut(|li| li.timestamp = 500);
        assert!(!is_overdue(&env, 1, 0));

        // After deadline
        env.ledger().with_mut(|li| li.timestamp = 1001);
        assert!(is_overdue(&env, 1, 0));
    }

    #[test]
    fn test_extension_history() {
        let (env, owner, r1, r2) = setup();
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(r1.clone());
        reviewers.push_back(r2.clone());
        create_grant_with_milestone(&env, &owner, &reviewers, 1, 1000);

        env.ledger().with_mut(|li| li.timestamp = 500);
        request_extension(&env, &owner, 1, 0, 2000, String::from_str(&env, "reason")).unwrap();

        // Approve it
        vote_extension(&env, &r1, 1, 0, true).unwrap();
        vote_extension(&env, &r2, 1, 0, true).unwrap();

        let history = get_extension_history(&env, 1);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().status, ExtensionStatus::Approved);
    }
}
