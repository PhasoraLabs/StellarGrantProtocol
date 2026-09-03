use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::storage::{DataKey, Storage, VotingKey};
use crate::types::{QuadraticVoteRecord, VoiceCredits};

/// Allocate voice credits to a voter for a grant (called when reviewer is added).
pub fn allocate_credits(
    env: &Env,
    voter: &Address,
    grant_id: u64,
    additional_credits: u32,
) -> Result<(), ContractError> {
    let existing = Storage::get_voice_credits(env, voter, grant_id);
    let record = VoiceCredits {
        voter: voter.clone(),
        grant_id,
        total_credits: existing.as_ref().map(|r| r.total_credits).unwrap_or(0).saturating_add(additional_credits),
        spent_credits: existing.map(|r| r.spent_credits).unwrap_or(0),
    };
    Storage::set_voice_credits(env, &record);
    Ok(())
}

fn get_voter_cumulative_votes(env: &Env, grant_id: u64, milestone_idx: u32, voter: &Address) -> u32 {
    let key = DataKey::Voting(VotingKey::CumulativeVotes(grant_id, milestone_idx, voter.clone()));
    env.storage().persistent().get(&key).unwrap_or(0)
}

fn set_voter_cumulative_votes(env: &Env, grant_id: u64, milestone_idx: u32, voter: &Address, votes: u32) {
    let key = DataKey::Voting(VotingKey::CumulativeVotes(grant_id, milestone_idx, voter.clone()));
    env.storage().persistent().set(&key, &votes);
}

/// Compute the credit cost for casting `votes` unit votes: cost = votes^2.
pub fn credit_cost(votes: u32) -> u32 {
    votes.saturating_mul(votes)
}

/// Cast `votes` unit votes on a milestone. Cost = votes^2 in voice credits.
pub fn cast_qv_vote(
    env: &Env,
    voter: &Address,
    grant_id: u64,
    milestone_idx: u32,
    additional_votes: u32,
    in_favor: bool,
) -> Result<QuadraticVoteRecord, ContractError> {
    voter.require_auth();

    if additional_votes == 0 {
        return Err(ContractError::InvalidInput);
    }

    let mut credits =
        Storage::get_voice_credits(env, voter, grant_id).ok_or(ContractError::VoterNotAllocated)?;

    let prior_votes = get_voter_cumulative_votes(env, grant_id, milestone_idx, voter);
    let new_total_votes = prior_votes.saturating_add(additional_votes);
    let marginal_cost = credit_cost(new_total_votes).saturating_sub(credit_cost(prior_votes));
    let available = credits.total_credits.saturating_sub(credits.spent_credits);
    
    if marginal_cost > available {
        return Err(ContractError::InsufficientVoiceCredits);
    }

    credits.spent_credits = credits
        .spent_credits
        .checked_add(marginal_cost)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_voice_credits(env, &credits);
    set_voter_cumulative_votes(env, grant_id, milestone_idx, voter, new_total_votes);

    let record = QuadraticVoteRecord {
        voter: voter.clone(),
        milestone_idx,
        votes_cast: additional_votes,
        credits_spent: marginal_cost,
        in_favor,
    };

    let mut votes_list = Storage::get_qv_votes(env, grant_id, milestone_idx);
    votes_list.push_back(record.clone());
    Storage::set_qv_votes(env, grant_id, milestone_idx, &votes_list);

    Ok(record)
}

/// Return remaining voice credits for a voter on a grant.
pub fn remaining_credits(env: &Env, voter: &Address, grant_id: u64) -> u32 {
    match Storage::get_voice_credits(env, voter, grant_id) {
        Some(c) => c.total_credits.saturating_sub(c.spent_credits),
        None => 0,
    }
}

/// Tally quadratic votes for a milestone. Returns (weighted_for, weighted_against).
pub fn tally_votes(env: &Env, grant_id: u64, milestone_idx: u32) -> (u32, u32) {
    let votes = Storage::get_qv_votes(env, grant_id, milestone_idx);
    let mut for_votes: u32 = 0;
    let mut against_votes: u32 = 0;
    for record in votes.iter() {
        if record.in_favor {
            for_votes = for_votes.saturating_add(record.votes_cast);
        } else {
            against_votes = against_votes.saturating_add(record.votes_cast);
        }
    }
    (for_votes, against_votes)
}

/// Determine approval using QV tally vs quorum threshold (>50% of total votes cast).
pub fn is_approved_qv(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    let (for_v, against_v) = tally_votes(env, grant_id, milestone_idx);
    let total = for_v.saturating_add(against_v);
    if total == 0 {
        return false;
    }
    for_v * 2 > total
}

/// Return all QV vote records for a milestone.
pub fn get_qv_votes(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<QuadraticVoteRecord> {
    Storage::get_qv_votes(env, grant_id, milestone_idx)
}

// ── Unit Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    #[test]
    fn test_credit_cost_squaring() {
        assert_eq!(credit_cost(1), 1);
        assert_eq!(credit_cost(2), 4);
        assert_eq!(credit_cost(3), 9);
        assert_eq!(credit_cost(4), 16);
    }

    #[test]
    fn test_overspend_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let voter = Address::generate(&env);
        allocate_credits(&env, &voter, 1, 9).unwrap();
        // costs 9, exactly at limit
        cast_qv_vote(&env, &voter, 1, 0, 3, true).unwrap();
        // next vote costs at least 1, but 0 remaining
        let err = cast_qv_vote(&env, &voter, 1, 0, 1, true).unwrap_err();
        assert_eq!(err, ContractError::InsufficientVoiceCredits);
    }

    #[test]
    fn test_tally_aggregates_correctly() {
        let env = Env::default();
        env.mock_all_auths();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let carol = Address::generate(&env);
        allocate_credits(&env, &alice, 1, 100).unwrap();
        allocate_credits(&env, &bob, 1, 100).unwrap();
        allocate_credits(&env, &carol, 1, 100).unwrap();
        cast_qv_vote(&env, &alice, 1, 0, 3, true).unwrap(); // 3 for
        cast_qv_vote(&env, &bob, 1, 0, 2, true).unwrap(); // 2 for
        cast_qv_vote(&env, &carol, 1, 0, 4, false).unwrap(); // 4 against
        let (for_v, against_v) = tally_votes(&env, 1, 0);
        assert_eq!(for_v, 5);
        assert_eq!(against_v, 4);
        assert!(is_approved_qv(&env, 1, 0));
    }

    #[test]
    fn test_no_voter_allocation_returns_error() {
        let env = Env::default();
        env.mock_all_auths();
        let voter = Address::generate(&env);
        let err = cast_qv_vote(&env, &voter, 1, 0, 1, true).unwrap_err();
        assert_eq!(err, ContractError::VoterNotAllocated);
    }
}
