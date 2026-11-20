use soroban_sdk::{token, Address, Env, String, Vec};

use crate::constants;
use crate::events::Events;
use crate::governance;
use crate::storage::Storage;
use crate::types::{
    BatchItemResult, BatchMilestoneVote, BatchResult, ContractError, GrantFund, GrantStatus,
    MilestoneState,
};

const MAX_BATCH_CANCEL_FUNDERS_PER_GRANT: u32 = constants::MAX_BATCH_CANCEL_FUNDERS_PER_GRANT;

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

/// Vote on multiple milestones in one call. Reviewer only.
/// Any failed vote aborts the batch and rolls back earlier changes.
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
        try_vote_milestone(env, reviewer, &vote)?;
        results.push_back(item_success(index as u32));
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
/// Any failed funding operation aborts the batch and rolls back earlier changes.
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
        try_fund_grant(env, funder, token, grant_id, amount)?;
        results.push_back(item_success(index as u32));
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
    crate::reentrancy::protect(env)?;
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

    grant.escrow_balance = new_escrow_balance;
    grant.funders = funders;
    Storage::set_grant(env, grant_id, &grant);

    let token_client = token::Client::new(env, &grant.token);
    let contract_address = env.current_contract_address();
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(funder, &contract_address, &amount);
        Ok(())
    })?;

    Events::emit_grant_funded(env, grant_id, funder.clone(), amount, grant.escrow_balance);

    Ok(())
}

/// Cancel multiple grants. Admin or owner only.
/// Any failed cancellation aborts the batch and rolls back earlier changes.
pub fn batch_cancel_grants(
    env: &Env,
    caller: &Address,
    grant_ids: Vec<u64>,
    reason: String,
) -> Result<BatchResult, ContractError> {
    let batch_len = grant_ids.len();
    validate_batch_size(batch_len)?;

    caller.require_auth();

    let mut total_nested_funders = 0u32;
    for grant_id in grant_ids.iter() {
        if let Some(grant) = Storage::get_grant(env, grant_id) {
            if grant.funders.len() > MAX_BATCH_CANCEL_FUNDERS_PER_GRANT {
                return Err(ContractError::BatchTooLarge);
            }
            total_nested_funders = total_nested_funders
                .checked_add(grant.funders.len())
                .ok_or(ContractError::BatchTooLarge)?;
            if total_nested_funders
                > (constants::MAX_BATCH_SIZE * MAX_BATCH_CANCEL_FUNDERS_PER_GRANT)
            {
                return Err(ContractError::BatchTooLarge);
            }
        }
    }

    let mut results = Vec::new(env);
    for (index, grant_id) in grant_ids.iter().enumerate() {
        try_cancel_grant(env, caller, grant_id, &reason)?;
        results.push_back(item_success(index as u32));
    }

    Ok(build_batch_result(results))
}

pub fn try_cancel_grant(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    reason: &String,
) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;

    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.funders.len() > MAX_BATCH_CANCEL_FUNDERS_PER_GRANT {
        return Err(ContractError::BatchTooLarge);
    }

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
    grant.status = GrantStatus::Cancelled;
    grant.escrow_balance = 0;
    grant.reason = Some(reason.clone());
    grant.timestamp = env.ledger().timestamp();
    Storage::set_grant(env, grant_id, &grant);

    if total_refundable > 0 {
        let mut total_contributions: i128 = 0;
        for fund_entry in grant.funders.iter() {
            total_contributions = total_contributions
                .checked_add(fund_entry.amount)
                .ok_or(ContractError::InvalidInput)?;
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
                crate::reentrancy::protect_external_call(env, || {
                    token_client.transfer(
                        &env.current_contract_address(),
                        &fund_entry.funder,
                        &refund_amount,
                    );
                    Ok(())
                })?;
                Events::emit_refund_issued(env, grant_id, fund_entry.funder.clone(), refund_amount);
            }
        }
    }

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
    use soroban_sdk::{token, Address, Env, String, Vec};

    fn setup_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    /// Like `setup_env`, but also registers a contract instance so the
    /// storage-backed helpers below (`create_grant`, `setup_reviewer`, and
    /// the batch functions themselves) can run inside `env.as_contract`.
    fn setup_contract_env() -> (Env, Address) {
        let env = setup_env();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        (env, contract_id)
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
                reviewer_list_snapshot: soroban_sdk::Vec::new(env),
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
        let _env = setup_env();
        let result = validate_batch_size(0);
        assert_eq!(result, Err(ContractError::BatchEmpty));
    }

    #[test]
    fn test_validate_batch_size_too_large() {
        let _env = setup_env();
        let result = validate_batch_size(constants::MAX_BATCH_SIZE + 1);
        assert_eq!(result, Err(ContractError::BatchTooLarge));
    }

    #[test]
    fn test_validate_batch_size_valid() {
        let _env = setup_env();
        let result = validate_batch_size(1);
        assert_eq!(result, Ok(()));
        let result = validate_batch_size(constants::MAX_BATCH_SIZE);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_try_cancel_grant_rejects_reentry() {
        let (env, contract_id) = setup_contract_env();
        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);
            create_grant(&env, 1, &owner, &token);
            env.storage()
                .temporary()
                .set(&crate::reentrancy::ReentrancyKey::ExternalCallGuard, &());

            let reason = String::from_str(&env, "reentry");
            assert_eq!(
                try_cancel_grant(&env, &owner, 1, &reason),
                Err(ContractError::Reentrancy)
            );
        });
    }

    #[test]
    fn test_batch_cancel_grants_rejects_too_many_funders_per_grant() {
        let (env, contract_id) = setup_contract_env();
        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);
            let mut grant = crate::types::Grant {
                id: 1,
                owner: owner.clone(),
                title: String::from_str(&env, "Test Grant"),
                description: String::from_str(&env, "Description"),
                token: token.clone(),
                status: GrantStatus::Active,
                total_amount: 100_000,
                milestone_amount: 50_000,
                reviewers: Vec::new(&env),
                total_milestones: 3,
                milestones_paid_out: 0,
                escrow_balance: 0,
                funders: Vec::new(&env),
                reason: None,
                timestamp: env.ledger().timestamp(),
                require_compliance: None,
            };
            for _ in 0..(constants::MAX_BATCH_CANCEL_FUNDERS_PER_GRANT + 1) {
                grant.funders.push_back(GrantFund {
                    funder: Address::generate(&env),
                    amount: 10,
                });
            }
            Storage::set_grant(&env, 1, &grant);

            let grant_ids = Vec::from_array(&env, [1u64]);
            let result =
                batch_cancel_grants(&env, &owner, grant_ids, String::from_str(&env, "too many"));
            assert_eq!(result, Err(ContractError::BatchTooLarge));
        });
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
                reason: Some(String::from_str(&env, "test")),
            });
        }

        let result = batch_vote_milestones(&env, &reviewer, votes);
        assert_eq!(result, Err(ContractError::BatchTooLarge));
    }

    #[test]
    fn test_batch_vote_milestones_failure_rolls_back_earlier_votes() {
        let (env, contract_id) = setup_contract_env();
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let token = Address::generate(&env);
        env.as_contract(&contract_id, || {
            create_grant(&env, 1, &owner, &token);
            setup_reviewer(&env, 1, &reviewer);
        });

        let mut votes = Vec::new(&env);
        votes.push_back(BatchMilestoneVote {
            grant_id: 1,
            milestone_idx: 0,
            approve: true,
            reason: Some(String::from_str(&env, "approve")),
        });
        votes.push_back(BatchMilestoneVote {
            grant_id: 2,
            milestone_idx: 0,
            approve: true,
            reason: Some(String::from_str(&env, "approve")),
        });

        assert_eq!(
            client.try_batch_vote_milestones(&reviewer, &votes),
            Err(Ok(ContractError::GrantNotFound.into()))
        );

        let milestone =
            env.as_contract(&contract_id, || Storage::get_milestone(&env, 1, 0).unwrap());
        assert!(!milestone.votes.contains_key(reviewer));
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
    fn test_batch_fund_grants_failure_rolls_back_earlier_transfers() {
        let (env, contract_id) = setup_contract_env();
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let token_admin = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let funder = Address::generate(&env);
        token::StellarAssetClient::new(&env, &token).mint(&funder, &1_000);
        let owner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            create_grant(&env, 1, &owner, &token);
        });

        let items = Vec::from_array(&env, [(1u64, 1_000i128), (2u64, 1_000i128)]);
        assert_eq!(
            client.try_batch_fund_grants(&funder, &token, &items),
            Err(Ok(ContractError::GrantNotFound.into()))
        );

        let grant = env.as_contract(&contract_id, || Storage::get_grant(&env, 1).unwrap());
        assert_eq!(grant.escrow_balance, 0);
        assert!(grant.funders.is_empty());
        let token_client = token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&funder), 1_000);
        assert_eq!(token_client.balance(&contract_id), 0);
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
    fn test_batch_cancel_grants_failure_rolls_back_earlier_cancellations() {
        let (env, contract_id) = setup_contract_env();
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        env.as_contract(&contract_id, || {
            create_grant(&env, 1, &owner, &token);
        });

        let grant_ids = Vec::from_array(&env, [1u64, 2u64]);
        let reason = String::from_str(&env, "test");
        assert_eq!(
            client.try_batch_cancel_grants(&owner, &grant_ids, &reason),
            Err(Ok(ContractError::GrantNotFound.into()))
        );

        let grant = env.as_contract(&contract_id, || Storage::get_grant(&env, 1).unwrap());
        assert_eq!(grant.status, GrantStatus::Active);
    }
}
