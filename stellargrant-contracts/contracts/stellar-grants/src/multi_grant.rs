use crate::errors::ContractError;
use crate::storage::keys::{DataKey, GrantKey};
use crate::storage::Storage;
use crate::types::{BatchResult, GrantPortfolio, GrantStatus, PortfolioFilter, PortfolioStats};
use soroban_sdk::{Address, Env, Vec};

/// Return portfolio stats for an owner address.
pub fn get_portfolio_stats(env: &Env, owner: &Address) -> PortfolioStats {
    let owner_index_key = DataKey::Grant(GrantKey::OwnerIndex(owner.clone()));
    let grant_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&owner_index_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut stats = PortfolioStats {
        owner: owner.clone(),
        total_grants: 0,
        active_grants: 0,
        completed_grants: 0,
        total_funded: 0,
        total_paid_out: 0,
        total_in_escrow: 0,
        unique_contributors: 0,
        unique_reviewers: 0,
        avg_completion_rate_bps: 0,
    };

    let mut contributors = Vec::new(env);
    let mut reviewers = Vec::new(env);
    let mut completion_rates_sum: u32 = 0;

    for i in 0..grant_ids.len() {
        if let Some(grant_id) = grant_ids.get(i) {
            if let Some(grant) = Storage::get_grant(env, grant_id) {
                stats.total_grants = stats.total_grants.saturating_add(1);
                stats.total_funded = stats.total_funded.saturating_add(grant.total_amount);
                stats.total_in_escrow = stats.total_in_escrow.saturating_add(grant.escrow_balance);

                // Calculate paid out
                if grant.status == GrantStatus::Completed {
                    stats.completed_grants = stats.completed_grants.saturating_add(1);
                    let paid_out = grant
                        .total_amount
                        .saturating_sub(grant.escrow_balance)
                        .max(0);
                    stats.total_paid_out = stats.total_paid_out.saturating_add(paid_out);
                } else if grant.status == GrantStatus::Active {
                    stats.active_grants = stats.active_grants.saturating_add(1);
                }

                // Track contributors
                if !contributors.contains(grant.owner.clone()) {
                    contributors.push_back(grant.owner.clone());
                }

                // Track reviewers
                for j in 0..grant.reviewers.len() {
                    if let Some(reviewer) = grant.reviewers.get(j) {
                        if !reviewers.contains(reviewer.clone()) {
                            reviewers.push_back(reviewer.clone());
                        }
                    }
                }

                // Completion rate: milestones_paid_out / total_milestones * 10000
                if grant.total_milestones > 0 {
                    let rate = (grant.milestones_paid_out as u32 * 10000)
                        .checked_div(grant.total_milestones)
                        .unwrap_or(0);
                    completion_rates_sum = completion_rates_sum.saturating_add(rate);
                }
            }
        }
    }

    stats.unique_contributors = contributors.len() as u32;
    stats.unique_reviewers = reviewers.len() as u32;

    // Average completion rate
    if stats.total_grants > 0 {
        stats.avg_completion_rate_bps = completion_rates_sum
            .checked_div(stats.total_grants)
            .unwrap_or(0);
    }

    stats
}

/// Return all grant IDs matching a filter, paginated.
pub fn filter_grants(env: &Env, filter: PortfolioFilter, offset: u32, limit: u32) -> Vec<u64> {
    // Get all grants for the owner (if specified)
    let grant_ids = if let Some(owner) = &filter.owner {
        let owner_index_key = DataKey::Grant(GrantKey::OwnerIndex(owner.clone()));
        env.storage()
            .persistent()
            .get(&owner_index_key)
            .unwrap_or_else(|| Vec::new(env))
    } else {
        // If no owner filter, return empty (would need global index for this case)
        return Vec::new(env);
    };

    let mut results = Vec::new(env);
    let len = grant_ids.len() as u32;
    let end = if offset + limit > len {
        len
    } else {
        offset + limit
    };

    if offset < len {
        for i in offset..end {
            if let Some(grant_id) = grant_ids.get(i) {
                if let Some(grant) = Storage::get_grant(env, grant_id) {
                    // Apply all filters
                    let matches_status =
                        filter.status.is_none() || filter.status == Some(grant.status);
                    let matches_token =
                        filter.token.is_none() || filter.token == Some(grant.token.clone());
                    let matches_amount = {
                        let min_ok = filter.min_amount.is_none()
                            || filter.min_amount <= Some(grant.total_amount);
                        let max_ok = filter.max_amount.is_none()
                            || filter.max_amount >= Some(grant.total_amount);
                        min_ok && max_ok
                    };

                    if matches_status && matches_token && matches_amount {
                        results.push_back(grant_id);
                    }
                }
            }
        }
    }

    results
}

/// Add a reviewer to multiple grants in one call. Owner of all grants only.
pub fn batch_add_reviewer(
    env: &Env,
    owner: &Address,
    grant_ids: Vec<u64>,
    reviewer: &Address,
) -> Result<BatchResult, ContractError> {
    let mut result = BatchResult {
        successful: 0,
        failed: 0,
        total: grant_ids.len() as u32,
    };

    for i in 0..grant_ids.len() {
        if let Some(grant_id) = grant_ids.get(i) {
            match add_reviewer_to_grant(env, owner, grant_id, reviewer) {
                Ok(()) => {
                    result.successful = result.successful.saturating_add(1);
                }
                Err(_) => {
                    result.failed = result.failed.saturating_add(1);
                }
            }
        }
    }

    Ok(result)
}

/// Remove a reviewer from multiple grants. Owner only.
pub fn batch_remove_reviewer(
    env: &Env,
    owner: &Address,
    grant_ids: Vec<u64>,
    reviewer: &Address,
) -> Result<BatchResult, ContractError> {
    let mut result = BatchResult {
        successful: 0,
        failed: 0,
        total: grant_ids.len() as u32,
    };

    for i in 0..grant_ids.len() {
        if let Some(grant_id) = grant_ids.get(i) {
            match remove_reviewer_from_grant(env, owner, grant_id, reviewer) {
                Ok(()) => {
                    result.successful = result.successful.saturating_add(1);
                }
                Err(_) => {
                    result.failed = result.failed.saturating_add(1);
                }
            }
        }
    }

    Ok(result)
}

/// Return aggregate escrow balance across all grants for an owner.
pub fn total_escrow_balance(env: &Env, owner: &Address, token: &Address) -> i128 {
    let owner_index_key = DataKey::Grant(GrantKey::OwnerIndex(owner.clone()));
    let grant_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&owner_index_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut total: i128 = 0;

    for i in 0..grant_ids.len() {
        if let Some(grant_id) = grant_ids.get(i) {
            if let Some(grant) = Storage::get_grant(env, grant_id) {
                if grant.token == *token {
                    total = total.saturating_add(grant.escrow_balance);
                }
            }
        }
    }

    total
}

/// Return the n most recently active grants for an owner.
pub fn recent_grants(env: &Env, owner: &Address, n: u32) -> Vec<u64> {
    let owner_index_key = DataKey::Grant(GrantKey::OwnerIndex(owner.clone()));
    let grant_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&owner_index_key)
        .unwrap_or_else(|| Vec::new(env));

    // Collect grants with timestamps
    let mut grants_with_time: Vec<(u64, u64)> = Vec::new(env);

    for i in 0..grant_ids.len() {
        if let Some(grant_id) = grant_ids.get(i) {
            if let Some(grant) = Storage::get_grant(env, grant_id) {
                grants_with_time.push_back((grant_id, grant.timestamp));
            }
        }
    }

    // Simple bubble sort by timestamp (descending)
    for i in 0..grants_with_time.len() {
        for j in 0..(grants_with_time.len().saturating_sub(1).saturating_sub(i)) {
            if let (Some(a), Some(b)) = (grants_with_time.get(j), grants_with_time.get(j + 1)) {
                if a.1 < b.1 {
                    // Swap
                    let temp = a.clone();
                    grants_with_time.set(j, b.clone());
                    grants_with_time.set(j + 1, temp);
                }
            }
        }
    }

    // Extract grant IDs and limit to n
    let mut result = Vec::new(env);
    let limit = if n < grants_with_time.len() {
        n
    } else {
        grants_with_time.len()
    };

    for i in 0..limit {
        if let Some((grant_id, _)) = grants_with_time.get(i) {
            result.push_back(grant_id);
        }
    }

    result
}

/// Build a full GrantPortfolio view for an owner.
pub fn get_portfolio(env: &Env, owner: &Address) -> GrantPortfolio {
    let stats = get_portfolio_stats(env, owner);

    let owner_index_key = DataKey::Grant(GrantKey::OwnerIndex(owner.clone()));
    let grant_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&owner_index_key)
        .unwrap_or_else(|| Vec::new(env));

    GrantPortfolio {
        owner: owner.clone(),
        grant_ids,
        stats,
    }
}

// ── Helper Functions ─────────────────────────────────────────────────────────

fn add_reviewer_to_grant(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    reviewer: &Address,
) -> Result<(), ContractError> {
    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    // Check if reviewer already in list
    if grant.reviewers.contains(reviewer.clone()) {
        return Ok(());
    }

    // Add reviewer
    grant.reviewers.push_back(reviewer.clone());
    Storage::set_grant(env, grant_id, &grant);

    Ok(())
}

fn remove_reviewer_from_grant(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    reviewer: &Address,
) -> Result<(), ContractError> {
    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    // Find and remove reviewer
    let mut found = false;
    let mut new_reviewers = Vec::new(env);

    for i in 0..grant.reviewers.len() {
        if let Some(r) = grant.reviewers.get(i) {
            if r != *reviewer {
                new_reviewers.push_back(r);
            } else {
                found = true;
            }
        }
    }

    if !found {
        return Err(ContractError::InvalidInput);
    }

    grant.reviewers = new_reviewers;
    Storage::set_grant(env, grant_id, &grant);

    Ok(())
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_stats_aggregation() {
        let env = soroban_sdk::Env::default();
        let owner = Address::random(&env);

        // Stats for non-existent owner should return empty stats
        let stats = get_portfolio_stats(&env, &owner);
        assert_eq!(stats.total_grants, 0);
        assert_eq!(stats.active_grants, 0);
        assert_eq!(stats.total_funded, 0);
    }

    #[test]
    fn test_filter_grants_by_status() {
        let env = soroban_sdk::Env::default();
        let owner = Address::random(&env);

        // Create a filter
        let filter = PortfolioFilter {
            owner: Some(owner.clone()),
            status: Some(GrantStatus::Active),
            token: None,
            category_id: None,
            min_amount: None,
            max_amount: None,
        };

        // Filter should return empty when no grants exist
        let results = filter_grants(&env, filter, 0, 10);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_total_escrow_balance() {
        let env = soroban_sdk::Env::default();
        let owner = Address::random(&env);
        let token = Address::random(&env);

        // Balance for non-existent owner should be 0
        let balance = total_escrow_balance(&env, &owner, &token);
        assert_eq!(balance, 0);
    }

    #[test]
    fn test_recent_grants_empty() {
        let env = soroban_sdk::Env::default();
        let owner = Address::random(&env);

        let recent = recent_grants(&env, &owner, 5);
        assert_eq!(recent.len(), 0);
    }

    #[test]
    fn test_batch_result_structure() {
        let result = BatchResult {
            successful: 5,
            failed: 2,
            total: 7,
        };

        assert_eq!(result.successful + result.failed, result.total);
    }

    #[test]
    fn test_portfolio_filter_all_none() {
        let env = soroban_sdk::Env::default();

        let filter = PortfolioFilter {
            owner: None,
            status: None,
            token: None,
            category_id: None,
            min_amount: None,
            max_amount: None,
        };

        // Filter with no owner returns empty
        let results = filter_grants(&env, filter, 0, 10);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_get_portfolio_structure() {
        let env = soroban_sdk::Env::default();
        let owner = Address::random(&env);

        let portfolio = get_portfolio(&env, &owner);
        assert_eq!(portfolio.owner, owner);
        assert_eq!(portfolio.stats.owner, owner);
    }
}
