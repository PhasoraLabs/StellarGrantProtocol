use soroban_sdk::{token, Address, Env, String, Vec};

use crate::constants::MAX_BOUNTY_SUBMISSIONS;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{BountyGrant, BountyStatus, BountySubmission, ContractError};

/// Publish a new bounty-mode grant. The owner deposits the full prize amount
/// up front; it is held in escrow by the contract until a winner is selected
/// or the bounty is cancelled.
#[allow(clippy::too_many_arguments)]
pub fn create_bounty(
    env: &Env,
    owner: &Address,
    title: String,
    description: String,
    token: &Address,
    prize_amount: i128,
    submission_window_ledgers: u32,
) -> Result<u64, ContractError> {
    if prize_amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }
    if title.is_empty() {
        return Err(ContractError::InvalidInput);
    }

    let id = Storage::next_bounty_id(env);
    let now = env.ledger().timestamp();
    let submission_deadline = now.saturating_add(submission_window_ledgers as u64);

    token::Client::new(env, token).transfer(owner, &env.current_contract_address(), &prize_amount);

    let bounty = BountyGrant {
        id,
        owner: owner.clone(),
        title: title.clone(),
        description,
        token: token.clone(),
        prize_amount,
        status: BountyStatus::Open,
        submission_deadline,
        winner: None,
        created_at: now,
    };

    Storage::set_bounty(env, &bounty);
    Events::emit_bounty_created(env, id, owner.clone(), title, prize_amount, submission_deadline);

    Ok(id)
}

/// Submit a solution to an open bounty. One submission per contributor.
pub fn submit_solution(
    env: &Env,
    bounty_id: u64,
    submitter: &Address,
    proof_url: String,
) -> Result<(), ContractError> {
    let bounty = Storage::get_bounty(env, bounty_id).ok_or(ContractError::BountyNotFound)?;

    if bounty.status != BountyStatus::Open {
        return Err(ContractError::BountyNotOpen);
    }
    if env.ledger().timestamp() > bounty.submission_deadline {
        return Err(ContractError::SubmissionWindowClosed);
    }
    if Storage::get_bounty_submission(env, bounty_id, submitter).is_some() {
        return Err(ContractError::AlreadyVoted);
    }

    let submitters = Storage::get_bounty_submitters(env, bounty_id);
    if submitters.len() >= MAX_BOUNTY_SUBMISSIONS {
        return Err(ContractError::ReviewerLimitExceeded);
    }

    let submission = BountySubmission {
        bounty_id,
        submitter: submitter.clone(),
        proof_url,
        submitted_at: env.ledger().timestamp(),
    };

    Storage::set_bounty_submission(env, &submission);
    Storage::add_bounty_submitter(env, bounty_id, submitter);

    Events::emit_bounty_submission_received(env, bounty_id, submitter.clone());

    Ok(())
}

/// Move a bounty into review (closing it to new submissions) once the owner
/// is ready to pick a winner. Owner only.
pub fn start_review(env: &Env, caller: &Address, bounty_id: u64) -> Result<(), ContractError> {
    let mut bounty = Storage::get_bounty(env, bounty_id).ok_or(ContractError::BountyNotFound)?;

    if bounty.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    if bounty.status != BountyStatus::Open {
        return Err(ContractError::BountyNotOpen);
    }

    bounty.status = BountyStatus::UnderReview;
    Storage::set_bounty(env, &bounty);
    Ok(())
}

/// Select the winning submission and pay out the full prize. Owner only.
pub fn select_winner(
    env: &Env,
    caller: &Address,
    bounty_id: u64,
    winner: &Address,
) -> Result<(), ContractError> {
    let mut bounty = Storage::get_bounty(env, bounty_id).ok_or(ContractError::BountyNotFound)?;

    if bounty.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    if bounty.status != BountyStatus::Open && bounty.status != BountyStatus::UnderReview {
        return Err(ContractError::BountyAlreadyResolved);
    }
    if Storage::get_bounty_submission(env, bounty_id, winner).is_none() {
        return Err(ContractError::SubmissionNotFound);
    }

    token::Client::new(env, &bounty.token).transfer(
        &env.current_contract_address(),
        winner,
        &bounty.prize_amount,
    );

    bounty.status = BountyStatus::Awarded;
    bounty.winner = Some(winner.clone());
    Storage::set_bounty(env, &bounty);

    Events::emit_bounty_awarded(env, bounty_id, winner.clone(), bounty.prize_amount);

    Ok(())
}

/// Cancel an unresolved bounty and refund the prize to the owner.
pub fn cancel_bounty(env: &Env, caller: &Address, bounty_id: u64) -> Result<(), ContractError> {
    let mut bounty = Storage::get_bounty(env, bounty_id).ok_or(ContractError::BountyNotFound)?;

    if bounty.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    if bounty.status != BountyStatus::Open && bounty.status != BountyStatus::UnderReview {
        return Err(ContractError::BountyAlreadyResolved);
    }

    token::Client::new(env, &bounty.token).transfer(
        &env.current_contract_address(),
        &bounty.owner,
        &bounty.prize_amount,
    );

    bounty.status = BountyStatus::Cancelled;
    Storage::set_bounty(env, &bounty);

    Events::emit_bounty_cancelled(env, bounty_id, caller.clone(), bounty.prize_amount);

    Ok(())
}

pub fn get_bounty(env: &Env, bounty_id: u64) -> Option<BountyGrant> {
    Storage::get_bounty(env, bounty_id)
}

pub fn get_submission(
    env: &Env,
    bounty_id: u64,
    submitter: &Address,
) -> Option<BountySubmission> {
    Storage::get_bounty_submission(env, bounty_id, submitter)
}

pub fn list_submitters(env: &Env, bounty_id: u64) -> Vec<Address> {
    Storage::get_bounty_submitters(env, bounty_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn make_bounty(env: &Env, owner: &Address, token: &Address) -> u64 {
        Storage::next_bounty_id(env);
        let id = 1u64;
        let bounty = BountyGrant {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "Bounty"),
            description: String::from_str(env, "desc"),
            token: token.clone(),
            prize_amount: 1000,
            status: BountyStatus::Open,
            submission_deadline: env.ledger().timestamp() + 1000,
            winner: None,
            created_at: env.ledger().timestamp(),
        };
        Storage::set_bounty(env, &bounty);
        id
    }

    #[test]
    fn test_submit_solution_to_unknown_bounty_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let submitter = Address::generate(&env);
        let result = submit_solution(&env, 999, &submitter, String::from_str(&env, "proof"));
        assert_eq!(result, Err(ContractError::BountyNotFound));
    }

    #[test]
    fn test_submit_solution_twice_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let submitter = Address::generate(&env);
        let id = make_bounty(&env, &owner, &token);

        submit_solution(&env, id, &submitter, String::from_str(&env, "proof1")).unwrap();
        let result = submit_solution(&env, id, &submitter, String::from_str(&env, "proof2"));
        assert_eq!(result, Err(ContractError::AlreadyVoted));
    }

    #[test]
    fn test_submit_solution_after_deadline_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let submitter = Address::generate(&env);
        let id = make_bounty(&env, &owner, &token);

        env.ledger().with_mut(|l| {
            l.timestamp += 10_000;
        });

        let result = submit_solution(&env, id, &submitter, String::from_str(&env, "proof"));
        assert_eq!(result, Err(ContractError::SubmissionWindowClosed));
    }

    #[test]
    fn test_select_winner_unauthorized_caller_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let stranger = Address::generate(&env);
        let submitter = Address::generate(&env);
        let id = make_bounty(&env, &owner, &token);
        submit_solution(&env, id, &submitter, String::from_str(&env, "proof")).unwrap();

        let result = select_winner(&env, &stranger, id, &submitter);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_select_winner_without_submission_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let non_submitter = Address::generate(&env);
        let id = make_bounty(&env, &owner, &token);

        let result = select_winner(&env, &owner, id, &non_submitter);
        assert_eq!(result, Err(ContractError::SubmissionNotFound));
    }

    #[test]
    fn test_cancel_bounty_by_non_owner_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let stranger = Address::generate(&env);
        let id = make_bounty(&env, &owner, &token);

        let result = cancel_bounty(&env, &stranger, id);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_start_review_closes_to_new_submissions() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let submitter = Address::generate(&env);
        let id = make_bounty(&env, &owner, &token);

        start_review(&env, &owner, id).unwrap();
        let bounty = get_bounty(&env, id).unwrap();
        assert_eq!(bounty.status, BountyStatus::UnderReview);

        let result = submit_solution(&env, id, &submitter, String::from_str(&env, "proof"));
        assert_eq!(result, Err(ContractError::BountyNotOpen));
    }

    #[test]
    fn test_list_submitters_tracks_all_entrants() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let s1 = Address::generate(&env);
        let s2 = Address::generate(&env);
        let id = make_bounty(&env, &owner, &token);

        submit_solution(&env, id, &s1, String::from_str(&env, "p1")).unwrap();
        submit_solution(&env, id, &s2, String::from_str(&env, "p2")).unwrap();

        let submitters = list_submitters(&env, id);
        assert_eq!(submitters.len(), 2);
    }
}
