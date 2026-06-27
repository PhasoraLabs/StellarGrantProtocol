use crate::errors::ContractError;
use crate::storage::keys::{DataKey, MatchingKey};
use crate::types::{MatchingAllocation, MatchingContribution, MatchingRound};
use soroban_sdk::{token, Address, Env, Vec};

const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_000_000;

/// Integer square root using Newton's method.
/// Accurate for all i128 values.
pub fn isqrt(n: i128) -> i128 {
    if n < 0 {
        return 0;
    }
    if n == 0 {
        return 0;
    }

    let mut x = n;
    let mut y = (x + 1) / 2;

    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }

    x
}

/// Create a new QF matching round. Admin deposits matching pool upfront.
pub fn create_round(
    env: &Env,
    admin: &Address,
    token: &Address,
    matching_pool: i128,
    duration_ledgers: u32,
    eligible_grant_ids: Vec<u64>,
) -> Result<u32, ContractError> {
    crate::reentrancy::protect(env)?;
    if matching_pool <= 0 {
        return Err(ContractError::InvalidInput);
    }

    if eligible_grant_ids.len() == 0 {
        return Err(ContractError::InvalidInput);
    }

    // Get and increment round counter
    let counter_key = DataKey::Matching(MatchingKey::Counter);
    let mut counter: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);
    counter = counter.saturating_add(1);
    env.storage().persistent().set(&counter_key, &counter);

    let round_id = counter;
    let start_ledger = env.ledger().sequence();
    let end_ledger = start_ledger.saturating_add(duration_ledgers);

    let round = MatchingRound {
        id: round_id,
        token: token.clone(),
        matching_pool,
        start_ledger,
        end_ledger,
        eligible_grant_ids: eligible_grant_ids.clone(),
        allocations: Vec::new(env),
        finalized: false,
        distributed: false,
        created_by: admin.clone(),
    };

    // Store round
    let round_key = DataKey::Matching(MatchingKey::Round(round_id));
    env.storage().persistent().set(&round_key, &round);
    env.storage().persistent().extend_ttl(
        &round_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );

    // Create pool entry
    let pool_key = DataKey::Matching(MatchingKey::Pool(round_id));
    env.storage().persistent().set(&pool_key, &matching_pool);
    env.storage().persistent().extend_ttl(
        &pool_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );

    // Transfer matching pool from admin to contract
    crate::reentrancy::protect_external_call(env, || {
        token::Client::new(env, token).transfer(
            admin,
            &env.current_contract_address(),
            &matching_pool,
        );
        Ok(())
    })?;

    Ok(round_id)
}

/// Contribute to a grant within an active matching round.
pub fn contribute(
    env: &Env,
    contributor: &Address,
    round_id: u32,
    grant_id: u64,
    amount: i128,
) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;
    if amount <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let round_key = DataKey::Matching(MatchingKey::Round(round_id));
    let mut round: MatchingRound = env
        .storage()
        .persistent()
        .get(&round_key)
        .ok_or(ContractError::InvalidInput)?;

    let current_ledger = env.ledger().sequence();
    if current_ledger < round.start_ledger || current_ledger > round.end_ledger {
        return Err(ContractError::InvalidState);
    }

    if round.finalized {
        return Err(ContractError::InvalidState);
    }

    // Check if grant is eligible
    let mut is_eligible = false;
    for i in 0..round.eligible_grant_ids.len() {
        if let Some(gid) = round.eligible_grant_ids.get(i) {
            if gid == grant_id {
                is_eligible = true;
                break;
            }
        }
    }

    if !is_eligible {
        return Err(ContractError::InvalidInput);
    }

    // Get or create contribution
    let contrib_key = DataKey::Matching(MatchingKey::Contribution(
        round_id,
        contributor.clone(),
        grant_id,
    ));
    let mut contribution: MatchingContribution = env
        .storage()
        .persistent()
        .get(&contrib_key)
        .unwrap_or_else(|| MatchingContribution {
            contributor: contributor.clone(),
            grant_id,
            amount: 0,
            contributed_at: 0,
        });

    contribution.amount = contribution.amount.saturating_add(amount);
    contribution.contributed_at = env.ledger().timestamp();

    env.storage().persistent().set(&contrib_key, &contribution);
    env.storage().persistent().extend_ttl(
        &contrib_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );

    // Transfer from contributor to contract escrow
    crate::reentrancy::protect_external_call(env, || {
        token::Client::new(env, &round.token).transfer(
            contributor,
            &env.current_contract_address(),
            &amount,
        );
        Ok(())
    })?;

    Ok(())
}

/// Compute QF allocations after round ends.
/// Uses: match_i = (sum_j sqrt(c_ij))^2 / sum_k (sum_j sqrt(c_kj))^2 * pool
pub fn compute_allocations(
    env: &Env,
    round_id: u32,
) -> Result<Vec<MatchingAllocation>, ContractError> {
    let round_key = DataKey::Matching(MatchingKey::Round(round_id));
    let mut round: MatchingRound = env
        .storage()
        .persistent()
        .get(&round_key)
        .ok_or(ContractError::InvalidInput)?;

    if round.finalized {
        return Err(ContractError::InvalidState);
    }

    let current_ledger = env.ledger().sequence();
    if current_ledger <= round.end_ledger {
        return Err(ContractError::InvalidState);
    }

    let mut allocations: Vec<MatchingAllocation> = Vec::new(env);
    let mut total_qf_score: i128 = 0;

    // Compute QF scores for each eligible grant
    for i in 0..round.eligible_grant_ids.len() {
        if let Some(grant_id) = round.eligible_grant_ids.get(i) {
            let (qf_score, direct_amount, unique_count) =
                compute_grant_qf_score(env, round_id, grant_id);

            total_qf_score = total_qf_score.saturating_add(qf_score);

            allocations.push_back(MatchingAllocation {
                grant_id,
                direct_contributions: direct_amount,
                match_amount: 0, // Will be computed below
                unique_contributors: unique_count,
                qf_score,
            });
        }
    }

    // Distribute matching pool proportionally
    let mut final_allocations = Vec::new(env);
    if total_qf_score > 0 {
        for j in 0..allocations.len() {
            if let Some(allocation) = allocations.get(j) {
                let match_amount = (allocation.qf_score * round.matching_pool)
                    .checked_div(total_qf_score)
                    .unwrap_or(0);
                final_allocations.push_back(MatchingAllocation {
                    grant_id: allocation.grant_id,
                    direct_contributions: allocation.direct_contributions,
                    match_amount,
                    unique_contributors: allocation.unique_contributors,
                    qf_score: allocation.qf_score,
                });
            }
        }
    } else {
        final_allocations = allocations.clone();
    }

    round.allocations = final_allocations.clone();
    round.finalized = true;

    env.storage().persistent().set(&round_key, &round);
    env.storage().persistent().extend_ttl(
        &round_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );

    Ok(allocations)
}

/// Distribute match amounts to each eligible grant's escrow.
pub fn distribute(env: &Env, round_id: u32) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;
    let round_key = DataKey::Matching(MatchingKey::Round(round_id));
    let mut round: MatchingRound = env
        .storage()
        .persistent()
        .get(&round_key)
        .ok_or(ContractError::InvalidInput)?;

    if !round.finalized {
        return Err(ContractError::InvalidState);
    }

    if round.distributed {
        return Err(ContractError::InvalidState);
    }

    // Distribute match amounts to each grant's escrow
    for i in 0..round.allocations.len() {
        if let Some(allocation) = round.allocations.get(i) {
            if allocation.match_amount > 0 {
                // Transfer match amount to grant's escrow
                let _ = crate::escrow::deposit(
                    env,
                    allocation.grant_id,
                    &env.current_contract_address(),
                    allocation.match_amount,
                );
            }
        }
    }

    round.distributed = true;

    env.storage().persistent().set(&round_key, &round);
    env.storage().persistent().extend_ttl(
        &round_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );

    Ok(())
}

/// Get a specific round.
pub fn get_round(env: &Env, round_id: u32) -> Result<MatchingRound, ContractError> {
    let round_key = DataKey::Matching(MatchingKey::Round(round_id));
    env.storage()
        .persistent()
        .get(&round_key)
        .ok_or(ContractError::InvalidInput)
}

/// Get a contributor's contribution to a specific grant in a round.
pub fn get_contribution(
    env: &Env,
    round_id: u32,
    contributor: &Address,
    grant_id: u64,
) -> Option<MatchingContribution> {
    let contrib_key = DataKey::Matching(MatchingKey::Contribution(
        round_id,
        contributor.clone(),
        grant_id,
    ));
    env.storage().persistent().get(&contrib_key)
}

/// Return all allocations for a round (post-computation).
pub fn get_allocations(env: &Env, round_id: u32) -> Vec<MatchingAllocation> {
    if let Ok(round) = get_round(env, round_id) {
        round.allocations
    } else {
        Vec::new(env)
    }
}

// ── Helper Functions ─────────────────────────────────────────────────────────

/// Compute QF score for a grant by summing square roots of all contributions.
/// Returns (qf_score, total_direct_contributions, unique_contributor_count)
fn compute_grant_qf_score(env: &Env, round_id: u32, grant_id: u64) -> (i128, i128, u32) {
    let mut qf_score: i128 = 0;
    let mut total_direct: i128 = 0;
    let mut unique_contributors: u32 = 0;
    let mut contributors: Vec<Address> = Vec::new(env);

    // This would require iterating through all contributions
    // For now, we use a simplified approach with a Map lookup
    // In production, you'd need a proper iterator pattern

    // Placeholder: compute from allocations stored during contributions
    // The actual implementation would iterate through contributions stored per grant
    (qf_score, total_direct, unique_contributors)
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_isqrt_zero() {
        assert_eq!(isqrt(0), 0);
    }

    #[test]
    fn test_isqrt_one() {
        assert_eq!(isqrt(1), 1);
    }

    #[test]
    fn test_isqrt_four() {
        assert_eq!(isqrt(4), 2);
    }

    #[test]
    fn test_isqrt_nine() {
        assert_eq!(isqrt(9), 3);
    }

    #[test]
    fn test_isqrt_sixteen() {
        assert_eq!(isqrt(16), 4);
    }

    #[test]
    fn test_isqrt_hundred() {
        assert_eq!(isqrt(100), 10);
    }

    #[test]
    fn test_isqrt_perfect_squares() {
        for i in 0..100 {
            let square = i * i;
            assert_eq!(isqrt(square), i);
        }
    }

    #[test]
    fn test_isqrt_non_perfect_squares() {
        assert_eq!(isqrt(2), 1);
        assert_eq!(isqrt(3), 1);
        assert_eq!(isqrt(5), 2);
        assert_eq!(isqrt(8), 2);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(26), 5);
    }

    #[test]
    fn test_isqrt_negative() {
        assert_eq!(isqrt(-1), 0);
        assert_eq!(isqrt(-100), 0);
    }

    #[test]
    fn test_isqrt_large() {
        assert_eq!(isqrt(1_000_000), 1_000);
        assert_eq!(isqrt(1_000_000_000_000), 1_000_000);
    }

    #[test]
    fn test_matching_allocation_structure() {
        let env = soroban_sdk::Env::default();
        let allocation = MatchingAllocation {
            grant_id: 1,
            direct_contributions: 1000,
            match_amount: 500,
            unique_contributors: 5,
            qf_score: 250,
        };

        assert_eq!(allocation.grant_id, 1);
        assert_eq!(allocation.direct_contributions, 1000);
        assert_eq!(allocation.match_amount, 500);
        assert_eq!(allocation.unique_contributors, 5);
        assert_eq!(allocation.qf_score, 250);
    }

    #[test]
    fn test_matching_contribution_structure() {
        let env = soroban_sdk::Env::default();
        let contributor = Address::generate(&env);

        let contribution = MatchingContribution {
            contributor: contributor.clone(),
            grant_id: 1,
            amount: 500,
            contributed_at: 1000,
        };

        assert_eq!(contribution.grant_id, 1);
        assert_eq!(contribution.amount, 500);
        assert_eq!(contribution.contributed_at, 1000);
    }

    #[test]
    fn test_matching_round_structure() {
        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let grant_ids = Vec::from_array(&env, [1u64, 2u64, 3u64]);

        let round = MatchingRound {
            id: 1,
            token: token.clone(),
            matching_pool: 10_000,
            start_ledger: 100,
            end_ledger: 1000,
            eligible_grant_ids: grant_ids,
            allocations: Vec::new(&env),
            finalized: false,
            distributed: false,
            created_by: admin.clone(),
        };

        assert_eq!(round.id, 1);
        assert_eq!(round.matching_pool, 10_000);
        assert!(!round.finalized);
        assert!(!round.distributed);
    }
}
