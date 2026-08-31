use soroban_sdk::{token, Address, Env, String, Vec};

use crate::constants;
use crate::events::Events;
use crate::governance;
use crate::storage::Storage;
use crate::types::{
    BatchItemResult, BatchMilestoneVote, BatchResult, ContractError, GrantFund, GrantStatus,
    MilestoneState,
};

fn validate_batch_size(len: u32) -> Result<(), ContractError> {
    if len == 0 {
        return Err(ContractError::BatchEmpty);
    }
    if len > constants::MAX_BATCH_SIZE {
        return Err(ContractError::BatchTooLarge);
    }
    Ok(())
}

fn build_batch_result(results: Vec<BatchItemResult>) -> BatchResult {
    let total = results.len();
    let mut succeeded = 0u32;
    let mut failed = 0u32;
    for result in results.iter() {
        if result.success {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }
    BatchResult {
        total,
        succeeded,
        failed,
        results,
    }
}

fn item_success(index: u32) -> BatchItemResult {
    BatchItemResult {
        index,
        success: true,
        error_code: None,
    }
}

fn item_failure(index: u32, error: ContractError) -> BatchItemResult {
    BatchItemResult {
        index,
        success: false,
        error_code: Some(error as u32),
    }
}

/// Vote on multiple milestones in one call. Reviewer only.
pub fn batch_vote_milestones(
    env: &Env,
    reviewer: &Address,
    votes: Vec<BatchMilestoneVote>,
) -> Result<BatchResult, ContractError> {
    let batch_len = votes.len();
    validate_batch_size(batch_len)?;

    reviewer.require_auth();

    let mut results = Vec::new(env);
    for (index, vote) in votes.iter().enumerate() {
        let item = match try_vote_milestone(env, reviewer, &vote) {
            Ok(()) => item_success(index as u32),
            Err(error) => item_failure(index as u32, error),
        };
        results.push_back(item);
    }

    Ok(build_batch_result(results))
}

fn try_vote_milestone(
    env: &Env,
    reviewer: &Address,
    vote: &BatchMilestoneVote,
) -> Result<(), ContractError> {
    let mut grant = Storage::get_grant(env, vote.grant_id).ok_or(ContractError::GrantNotFound)?;

    if vote.milestone_idx >= grant.total_milestones {
        return Err(ContractError::InvalidInput);
    }

    let mut milestone = Storage::get_milestone(env, vote.grant_id, vote.milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;

    if milestone.state != MilestoneState::Submitted {
        return Err(ContractError::MilestoneNotSubmitted);
    }
    if !grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }
    if milestone.votes.contains_key(reviewer.clone()) {
        return Err(ContractError::AlreadyVoted);
    }

    governance::cast_vote(
        env,
        &mut grant,
        &mut milestone,
        reviewer,
        vote.approve,
        vote.reason.clone(),
    )?;

    Storage::set_milestone(env, vote.grant_id, vote.milestone_idx, &milestone);
    Ok(())
}

/// Fund multiple grants with the same token in one call. Funder only.
pub fn batch_fund_grants(
    env: &Env,
    funder: &Address,
    token: &Address,
    items: Vec<(u64, i128)>,
) -> Result<BatchResult, ContractError> {
    let batch_len = items.len();
    validate_batch_size(batch_len)?;

    funder.require_auth();

    let mut results = Vec::new(env);
    for (index, item) in items.iter().enumerate() {
        let (grant_id, amount) = item;
        let item = match try_fund_grant(env, funder, token, grant_id, amount) {
            Ok(()) => item_success(index as u32),
            Err(error) => item_failure(index as u32, error),
        };
        results.push_back(item);
    }

    Ok(build_batch_result(results))
}

fn try_fund_grant(
    env: &Env,
    funder: &Address,
    token: &Address,
    grant_id: u64,
    amount: i128,
) -> Result<(), ContractError> {
    if amount <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if grant.token != *token {
        return Err(ContractError::InvalidInput);
    }

    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }

    let new_escrow_balance = grant
        .escrow_balance
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;

    let mut funders = grant.funders.clone();
    let mut funder_found = false;
    for i in 0..funders.len() {
        let mut fund_entry = funders.get(i).unwrap();
        if fund_entry.funder == *funder {
            fund_entry.amount = fund_entry
                .amount
                .checked_add(amount)
                .ok_or(ContractError::InvalidInput)?;
            funders.set(i, fund_entry);
            funder_found = true;
            break;
        }
    }

    if !funder_found {
        funders.push_back(GrantFund {
            funder: funder.clone(),
            amount,
        });
    }

    let token_client = token::Client::new(env, &grant.token);
    let contract_address = env.current_contract_address();
    token_client.transfer(funder, &contract_address, &amount);

    grant.escrow_balance = new_escrow_balance;
    grant.funders = funders;

    Storage::set_grant(env, grant_id, &grant);
    Events::emit_grant_funded(env, grant_id, funder.clone(), amount, grant.escrow_balance);

    Ok(())
}

/// Cancel multiple grants. Admin or owner only.
pub fn batch_cancel_grants(
    env: &Env,
    caller: &Address,
    grant_ids: Vec<u64>,
    reason: String,
) -> Result<BatchResult, ContractError> {
    let batch_len = grant_ids.len();
    validate_batch_size(batch_len)?;

    caller.require_auth();

    let mut results = Vec::new(env);
    for (index, grant_id) in grant_ids.iter().enumerate() {
        let item = match try_cancel_grant(env, caller, grant_id, &reason) {
            Ok(()) => item_success(index as u32),
            Err(error) => item_failure(index as u32, error),
        };
        results.push_back(item);
    }

    Ok(build_batch_result(results))
}

fn try_cancel_grant(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    reason: &String,
) -> Result<(), ContractError> {
    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    let caller_is_owner = grant.owner == *caller;
    let caller_is_admin = Storage::get_global_admin(env) == Some(caller.clone());
    if !caller_is_owner && !caller_is_admin {
        return Err(ContractError::Unauthorized);
    }

    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }

    if grant.milestones_paid_out >= grant.total_milestones {
        return Err(ContractError::InvalidState);
    }

    let total_refundable = grant.escrow_balance;
    if total_refundable > 0 {
        let mut total_contributions: i128 = 0;
        for fund_entry in grant.funders.iter() {
            total_contributions += fund_entry.amount;
        }

        if total_contributions <= 0 {
            return Err(ContractError::InvalidInput);
        }

        let token_client = token::Client::new(env, &grant.token);
        let funders_len = grant.funders.len();
        let mut distributed = 0i128;

        for i in 0..funders_len {
            let fund_entry = grant.funders.get(i).unwrap();
            let is_last = i + 1 == funders_len;
            let refund_amount = if is_last {
                total_refundable - distributed
            } else {
                let amount = fund_entry
                    .amount
                    .checked_mul(total_refundable)
                    .ok_or(ContractError::InvalidInput)?
                    .checked_div(total_contributions)
                    .ok_or(ContractError::InvalidInput)?;
                distributed += amount;
                amount
            };

            if refund_amount > 0 {
                token_client.transfer(
                    &env.current_contract_address(),
                    &fund_entry.funder,
                    &refund_amount,
                );
                Events::emit_refund_issued(env, grant_id, fund_entry.funder.clone(), refund_amount);
            }
        }
    }

    grant.status = GrantStatus::Cancelled;
    grant.escrow_balance = 0;
    grant.reason = Some(reason.clone());
    grant.timestamp = env.ledger().timestamp();

    Storage::set_grant(env, grant_id, &grant);
    Events::emit_grant_cancelled(
        env,
        grant_id,
        caller.clone(),
        reason.clone(),
        total_refundable,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env, String, Vec};

    fn setup_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn create_grant(env: &Env, id: u64, owner: &Address, token: &Address) {
        let grant = crate::types::Grant {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Description"),
            token: token.clone(),
            status: GrantStatus::Active,
            total_amount: 100_000,
            milestone_amount: 50_000,
            reviewers: Vec::new(env),
            total_milestones: 3,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, id, &grant);

        // Create milestones
        for idx in 0..3 {
            let milestone = crate::types::Milestone {
                idx,
                description: String::from_str(env, "Milestone"),
                amount: 50_000,
                state: MilestoneState::Submitted,
                votes: soroban_sdk::Map::new(env),
                approvals: 0,
                rejections: 0,
                reasons: soroban_sdk::Map::new(env),
                status_updated_at: env.ledger().timestamp(),
                proof_url: None,
                submission_timestamp: env.ledger().timestamp(),
                deadline: None,
                reviewer_count_snapshot: 0,
            };
            Storage::set_milestone(env, id, idx, &milestone);
        }
    }

    fn setup_reviewer(env: &Env, grant_id: u64, reviewer: &Address) {
        let mut grant = Storage::get_grant(env, grant_id).unwrap();
        grant.reviewers.push_back(reviewer.clone());
        Storage::set_grant(env, grant_id, &grant);
    }

    // ── validate_batch_size ─────────────────────────────────────────────

    #[test]
    fn test_validate_batch_size_empty() {
        let env = setup_env();
        let result = validate_batch_size(0);
        assert_eq!(result, Err(ContractError::BatchEmpty));
    }

    #[test]
    fn test_validate_batch_size_too_large() {
        let env = setup_env();
        let result = validate_batch_size(constants::MAX_BATCH_SIZE + 1);
        assert_eq!(result, Err(ContractError::BatchTooLarge));
    }

    #[test]
    fn test_validate_batch_size_valid() {
        let env = setup_env();
        let result = validate_batch_size(1);
        assert_eq!(result, Ok(()));
        let result = validate_batch_size(constants::MAX_BATCH_SIZE);
        assert_eq!(result, Ok(()));
    }

    // ── build_batch_result ───────────────────────────────────────────────

    #[test]
    fn test_build_batch_result_all_success() {
        let env = setup_env();
        let mut results = Vec::new(&env);
        results.push_back(item_success(0));
        results.push_back(item_success(1));
        results.push_back(item_success(2));

        let batch_result = build_batch_result(results);
        assert_eq!(batch_result.total, 3);
        assert_eq!(batch_result.succeeded, 3);
        assert_eq!(batch_result.failed, 0);
        assert_eq!(batch_result.results.len(), 3);
    }

    #[test]
    fn test_build_batch_result_all_failure() {
        let env = setup_env();
        let mut results = Vec::new(&env);
        results.push_back(item_failure(0, ContractError::GrantNotFound));
        results.push_back(item_failure(1, ContractError::Unauthorized));
        results.push_back(item_failure(2, ContractError::InvalidInput));

        let batch_result = build_batch_result(results);
        assert_eq!(batch_result.total, 3);
        assert_eq!(batch_result.succeeded, 0);
        assert_eq!(batch_result.failed, 3);
        assert_eq!(batch_result.results.len(), 3);
    }

    #[test]
    fn test_build_batch_result_mixed() {
        let env = setup_env();
        let mut results = Vec::new(&env);
        results.push_back(item_success(0));
        results.push_back(item_failure(1, ContractError::GrantNotFound));
        results.push_back(item_success(2));
        results.push_back(item_failure(3, ContractError::Unauthorized));

        let batch_result = build_batch_result(results);
        assert_eq!(batch_result.total, 4);
        assert_eq!(batch_result.succeeded, 2);
        assert_eq!(batch_result.failed, 2);
        assert_eq!(batch_result.results.len(), 4);
    }

    // ── batch_vote_milestones ─────────────────────────────────────────────

    #[test]
    fn test_batch_vote_milestones_empty() {
        let env = setup_env();
        let reviewer = Address::generate(&env);
        let votes = Vec::new(&env);

        let result = batch_vote_milestones(&env, &reviewer, votes);
        assert_eq!(result, Err(ContractError::BatchEmpty));
    }

    #[test]
    fn test_batch_vote_milestones_too_large() {
        let env = setup_env();
        let reviewer = Address::generate(&env);
        let mut votes = Vec::new(&env);
        for _ in 0..(constants::MAX_BATCH_SIZE + 1) {
            votes.push_back(BatchMilestoneVote {
                grant_id: 1,
                milestone_idx: 0,
                approve: true,
                reason: String::from_str(&env, "test"),
            });
        }

        let result = batch_vote_milestones(&env, &reviewer, votes);
        assert_eq!(result, Err(ContractError::BatchTooLarge));
    }

    #[test]
    fn test_batch_vote_milestones_mixed_valid_invalid() {
        let env = setup_env();
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let token = Address::generate(&env);

        // Create grant 1 with reviewer
        create_grant(&env, 1, &owner, &token);
        setup_reviewer(&env, 1, &reviewer);

        // Create grant 2 without reviewer (will fail)
        create_grant(&env, 2, &owner, &token);

        // Create grant 3 with reviewer
        create_grant(&env, 3, &owner, &token);
        setup_reviewer(&env, 3, &reviewer);

        // Batch: grant 1 (valid), grant 2 (invalid - not reviewer), grant 3 (valid)
        let mut votes = Vec::new(&env);
        votes.push_back(BatchMilestoneVote {
            grant_id: 1,
            milestone_idx: 0,
            approve: true,
            reason: String::from_str(&env, "approve"),
        });
        votes.push_back(BatchMilestoneVote {
            grant_id: 2,
            milestone_idx: 0,
            approve: true,
            reason: String::from_str(&env, "approve"),
        });
        votes.push_back(BatchMilestoneVote {
            grant_id: 3,
            milestone_idx: 0,
            approve: true,
            reason: String::from_str(&env, "approve"),
        });

        let result = batch_vote_milestones(&env, &reviewer, votes).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);

        // Check that valid votes were recorded
        let ms1 = Storage::get_milestone(&env, 1, 0).unwrap();
        assert!(ms1.votes.contains_key(reviewer.clone()));

        let ms3 = Storage::get_milestone(&env, 3, 0).unwrap();
        assert!(ms3.votes.contains_key(reviewer.clone()));

        // Check that invalid vote didn't corrupt state
        let ms2 = Storage::get_milestone(&env, 2, 0).unwrap();
        assert!(!ms2.votes.contains_key(reviewer.clone()));
    }

    #[test]
    fn test_batch_vote_milestones_failed_item_doesnt_corrupt_others() {
        let env = setup_env();
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let token = Address::generate(&env);

        // Create grant 1 with reviewer
        create_grant(&env, 1, &owner, &token);
        setup_reviewer(&env, 1, &reviewer);

        // Create grant 2 (nonexistent - will fail)
        // Don't create grant 2

        // Create grant 3 with reviewer
        create_grant(&env, 3, &owner, &token);
        setup_reviewer(&env, 3, &reviewer);

        let mut votes = Vec::new(&env);
        votes.push_back(BatchMilestoneVote {
            grant_id: 1,
            milestone_idx: 0,
            approve: true,
            reason: String::from_str(&env, "approve"),
        });
        votes.push_back(BatchMilestoneVote {
            grant_id: 2, // doesn't exist
            milestone_idx: 0,
            approve: true,
            reason: String::from_str(&env, "approve"),
        });
        votes.push_back(BatchMilestoneVote {
            grant_id: 3,
            milestone_idx: 0,
            approve: true,
            reason: String::from_str(&env, "approve"),
        });

        let result = batch_vote_milestones(&env, &reviewer, votes).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);

        // Verify valid votes succeeded despite middle failure
        let ms1 = Storage::get_milestone(&env, 1, 0).unwrap();
        assert!(ms1.votes.contains_key(reviewer.clone()));

        let ms3 = Storage::get_milestone(&env, 3, 0).unwrap();
        assert!(ms3.votes.contains_key(reviewer.clone()));
    }

    // ── batch_fund_grants ─────────────────────────────────────────────────

    #[test]
    fn test_batch_fund_grants_empty() {
        let env = setup_env();
        let funder = Address::generate(&env);
        let token = Address::generate(&env);
        let items = Vec::new(&env);

        let result = batch_fund_grants(&env, &funder, &token, items);
        assert_eq!(result, Err(ContractError::BatchEmpty));
    }

    #[test]
    fn test_batch_fund_grants_mixed_valid_invalid() {
        let env = setup_env();
        let owner = Address::generate(&env);
        let funder = Address::generate(&env);
        let token = Address::generate(&env);

        // Create grant 1 with matching token
        create_grant(&env, 1, &owner, &token);

        // Create grant 2 with different token (will fail)
        let token2 = Address::generate(&env);
        create_grant(&env, 2, &owner, &token2);

        // Create grant 3 with matching token
        create_grant(&env, 3, &owner, &token);

        let mut items = Vec::new(&env);
        items.push_back((1, 1_000));
        items.push_back((2, 1_000)); // wrong token
        items.push_back((3, 1_000));

        let result = batch_fund_grants(&env, &funder, &token, items).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);

        // Check that valid funding succeeded
        let grant1 = Storage::get_grant(&env, 1).unwrap();
        assert_eq!(grant1.escrow_balance, 1_000);

        let grant3 = Storage::get_grant(&env, 3).unwrap();
        assert_eq!(grant3.escrow_balance, 1_000);

        // Check that invalid funding didn't corrupt state
        let grant2 = Storage::get_grant(&env, 2).unwrap();
        assert_eq!(grant2.escrow_balance, 0);
    }

    // ── batch_cancel_grants ───────────────────────────────────────────────

    #[test]
    fn test_batch_cancel_grants_empty() {
        let env = setup_env();
        let caller = Address::generate(&env);
        let grant_ids = Vec::new(&env);
        let reason = String::from_str(&env, "test");

        let result = batch_cancel_grants(&env, &caller, grant_ids, reason);
        assert_eq!(result, Err(ContractError::BatchEmpty));
    }

    #[test]
    fn test_batch_cancel_grants_mixed_valid_invalid() {
        let env = setup_env();
        let owner = Address::generate(&env);
        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);

        // Create grant 1 owned by owner
        create_grant(&env, 1, &owner, &token);

        // Create grant 2 owned by different person (caller can't cancel)
        let other = Address::generate(&env);
        create_grant(&env, 2, &other, &token);

        // Create grant 3 owned by owner
        create_grant(&env, 3, &owner, &token);

        let mut grant_ids = Vec::new(&env);
        grant_ids.push_back(1);
        grant_ids.push_back(2); // unauthorized
        grant_ids.push_back(3);

        let reason = String::from_str(&env, "test");
        let result = batch_cancel_grants(&env, &owner, grant_ids, reason).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);

        // Check that valid cancels succeeded
        let grant1 = Storage::get_grant(&env, 1).unwrap();
        assert_eq!(grant1.status, GrantStatus::Cancelled);

        let grant3 = Storage::get_grant(&env, 3).unwrap();
        assert_eq!(grant3.status, GrantStatus::Cancelled);

        // Check that invalid cancel didn't corrupt state
        let grant2 = Storage::get_grant(&env, 2).unwrap();
        assert_eq!(grant2.status, GrantStatus::Active);
    }

    #[test]
    fn test_batch_cancel_grants_failed_item_doesnt_corrupt_others() {
        let env = setup_env();
        let owner = Address::generate(&env);
        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);

        // Create grant 1
        create_grant(&env, 1, &owner, &token);

        // Don't create grant 2 (will fail)

        // Create grant 3
        create_grant(&env, 3, &owner, &token);

        let mut grant_ids = Vec::new(&env);
        grant_ids.push_back(1);
        grant_ids.push_back(2); // doesn't exist
        grant_ids.push_back(3);

        let reason = String::from_str(&env, "test");
        let result = batch_cancel_grants(&env, &admin, grant_ids, reason).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);

        // Verify valid cancels succeeded despite middle failure
        let grant1 = Storage::get_grant(&env, 1).unwrap();
        assert_eq!(grant1.status, GrantStatus::Cancelled);

        let grant3 = Storage::get_grant(&env, 3).unwrap();
        assert_eq!(grant3.status, GrantStatus::Cancelled);
    }
}
