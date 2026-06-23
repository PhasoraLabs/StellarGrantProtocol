use soroban_sdk::{Address, Env, String};

use crate::constants::MAX_BIO_LEN;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, Grant, Milestone, MilestoneState};

pub struct VoteResult {
    pub approved: bool,
    pub quorum_reached: bool,
    pub approval_pct: u32,
}

/// Cast a vote (approve = true, reject = false) on a milestone.
/// Enforces: reviewer is registered, has not already voted, milestone is Submitted.
/// Handles reputation-weighted tallying, quorum detection, voter rewards,
/// milestone state finalization, and event emission.
pub fn cast_vote(
    env: &Env,
    grant: &mut Grant,
    milestone: &mut Milestone,
    reviewer: &Address,
    approve: bool,
    reason: Option<String>,
) -> Result<VoteResult, ContractError> {
    if milestone.state != MilestoneState::Submitted {
        env.panic_with_error(ContractError::MilestoneNotSubmitted);
    }
    if !grant.reviewers.contains(reviewer.clone()) {
        env.panic_with_error(ContractError::Unauthorized);
    }
    if milestone.votes.contains_key(reviewer.clone()) {
        env.panic_with_error(ContractError::AlreadyVoted);
    }

    if let Some(ref r) = reason {
        if r.len() > MAX_BIO_LEN {
            return Err(ContractError::InvalidInput);
        }
        milestone.reasons.set(reviewer.clone(), r.clone());
    }

    let reputation = Storage::get_reviewer_reputation(env, reviewer.clone());
    milestone.votes.set(reviewer.clone(), approve);
    if approve {
        milestone.approvals += reputation;
    } else {
        milestone.rejections += reputation;
    }

    let mut total_weight: u32 = 0;
    for r in grant.reviewers.iter() {
        total_weight += Storage::get_reviewer_reputation(env, r);
    }

    let approval_quorum = quorum_reached(milestone.approvals, total_weight);
    let rejection_quorum = quorum_reached(milestone.rejections, total_weight);
    let vote_finalized = approval_quorum || rejection_quorum;

    let total_votes = milestone.approvals + milestone.rejections;
    let pct = approval_percentage(milestone.approvals, total_votes);

    let result = VoteResult {
        approved: approval_quorum,
        quorum_reached: vote_finalized,
        approval_pct: pct,
    };

    if vote_finalized {
        milestone.status_updated_at = env.ledger().timestamp();

        // Reward voters who aligned with the final outcome
        for (voter, voted_approve) in milestone.votes.iter() {
            if voted_approve == approval_quorum {
                let rep = Storage::get_reviewer_reputation(env, voter.clone());
                Storage::set_reviewer_reputation(env, voter.clone(), rep.saturating_add(1));
            }
        }

        let grant_id = grant.id;
        let milestone_idx = milestone.idx;
        finalize_milestone(milestone, &result);
        let new_state = if result.approved {
            MilestoneState::Approved
        } else {
            MilestoneState::Rejected
        };
        Events::milestone_status_changed(env, grant_id, milestone_idx, new_state);
    }

    Events::milestone_voted(
        env,
        grant.id,
        milestone.idx,
        reviewer.clone(),
        approve,
        reason,
    );

    Ok(result)
}

/// Compute whether quorum is reached given current approvals and total reviewers.
/// Quorum = strictly more than 50% of reviewers approved.
pub fn quorum_reached(approvals: u32, total_reviewers: u32) -> bool {
    if total_reviewers == 0 {
        return false;
    }
    approvals * 2 > total_reviewers
}

/// Compute the approval percentage (0-100) from votes cast.
pub fn approval_percentage(approvals: u32, total_voters: u32) -> u32 {
    if total_voters == 0 {
        return 0;
    }
    (approvals * 100) / total_voters
}

/// Finalize milestone state based on vote outcome.
/// Sets MilestoneState::Approved or MilestoneState::Rejected.
pub fn finalize_milestone(milestone: &mut Milestone, result: &VoteResult) {
    if result.approved {
        milestone.state = MilestoneState::Approved;
    } else {
        milestone.state = MilestoneState::Rejected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum_zero_reviewers() {
        assert!(!quorum_reached(0, 0));
        assert!(!quorum_reached(1, 0));
    }

    #[test]
    fn test_quorum_single_reviewer() {
        assert!(quorum_reached(1, 1)); // 1/1 = 100%
        assert!(!quorum_reached(0, 1)); // 0/1 = 0%
    }

    #[test]
    fn test_quorum_exact_50_percent_not_reached() {
        assert!(!quorum_reached(1, 2)); // exactly 50%, needs strictly more
        assert!(!quorum_reached(2, 4)); // exactly 50%
    }

    #[test]
    fn test_quorum_51_percent_reached() {
        assert!(quorum_reached(2, 3)); // 66% > 50%
        assert!(quorum_reached(3, 4)); // 75% > 50%
        assert!(quorum_reached(3, 5)); // 60% > 50%
    }

    #[test]
    fn test_quorum_all_reject_path() {
        // If all reject, 0 approvals → no approval quorum
        assert!(!quorum_reached(0, 3));
        // But rejection quorum check: quorum_reached(3, 3) = true
        assert!(quorum_reached(3, 3));
    }

    #[test]
    fn test_approval_percentage_zero_voters() {
        assert_eq!(approval_percentage(0, 0), 0);
    }

    #[test]
    fn test_approval_percentage_full() {
        assert_eq!(approval_percentage(5, 5), 100);
    }

    #[test]
    fn test_approval_percentage_none() {
        assert_eq!(approval_percentage(0, 5), 0);
    }

    #[test]
    fn test_approval_percentage_partial() {
        assert_eq!(approval_percentage(1, 2), 50);
        assert_eq!(approval_percentage(2, 3), 66);
    }

    #[test]
    fn test_finalize_milestone_approved() {
        let env = soroban_sdk::Env::default();
        let mut milestone = crate::types::Milestone {
            idx: 0,
            description: soroban_sdk::String::from_str(&env, "test"),
            amount: 100,
            state: MilestoneState::Submitted,
            votes: soroban_sdk::Map::new(&env),
            approvals: 3,
            rejections: 0,
            reasons: soroban_sdk::Map::new(&env),
            status_updated_at: 0,
            proof_url: None,
            submission_timestamp: 0,
        };
        let result = VoteResult {
            approved: true,
            quorum_reached: true,
            approval_pct: 100,
        };
        finalize_milestone(&mut milestone, &result);
        assert_eq!(milestone.state, MilestoneState::Approved);
    }

    #[test]
    fn test_finalize_milestone_rejected() {
        let env = soroban_sdk::Env::default();
        let mut milestone = crate::types::Milestone {
            idx: 0,
            description: soroban_sdk::String::from_str(&env, "test"),
            amount: 100,
            state: MilestoneState::Submitted,
            votes: soroban_sdk::Map::new(&env),
            approvals: 0,
            rejections: 3,
            reasons: soroban_sdk::Map::new(&env),
            status_updated_at: 0,
            proof_url: None,
            submission_timestamp: 0,
        };
        let result = VoteResult {
            approved: false,
            quorum_reached: true,
            approval_pct: 0,
        };
        finalize_milestone(&mut milestone, &result);
        assert_eq!(milestone.state, MilestoneState::Rejected);
    }
}
