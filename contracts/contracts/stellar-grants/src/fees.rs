use soroban_sdk::{token, Address, Env};

use crate::config;
use crate::storage::Storage;
use crate::types::ContractError;

/// Compute protocol fee from gross amount. Used in tests for fee calculation validation.
#[allow(dead_code)]
pub fn compute_fee(gross: i128, fee_bps: u32) -> Result<i128, ContractError> {
    if fee_bps == 0 || gross <= 0 {
        return Ok(0);
    }
    crate::math::basis_points_of(gross, fee_bps)
}

/// Compute net amount after protocol fee deduction.
pub fn net_of_fee(gross: i128, fee_bps: u32) -> Result<i128, ContractError> {
    let fee = compute_fee(gross, fee_bps)?;
    gross.checked_sub(fee).ok_or(ContractError::InvalidInput)
}

/// Deduct protocol fee from a gross milestone amount, split it among the
/// reviewer-reward pool, the staker revenue-share pool, and the treasury,
/// then return the net amount payable to the grant recipient.
///
/// Called automatically by the payment flow in `lib.rs`.
pub fn deduct_and_split_fee(
    env: &Env,
    token: &Address,
    gross_amount: i128,
) -> Result<i128, ContractError> {
    if gross_amount <= 0 {
        return Ok(0);
    }

    let cfg = config::get_config(env);
    let total_fee = compute_fee(gross_amount, cfg.protocol_fee_bps)?;
    if total_fee <= 0 {
        return Ok(gross_amount);
    }

    // Split the fee according to reviewer_reward_pool_bps and revenue_share_pool_bps.
    // The remaining portion stays with the protocol (treasury, collected as fees).
    let reviewer_share = crate::math::basis_points_of(total_fee, cfg.reviewer_reward_pool_bps)?;
    let revenue_share = crate::math::basis_points_of(total_fee, cfg.revenue_share_pool_bps)?;
    let treasury_share = total_fee
        .checked_sub(reviewer_share)
        .and_then(|r| r.checked_sub(revenue_share))
        .ok_or(ContractError::InvalidInput)?;

    if reviewer_share > 0 {
        crate::reviewer_reward::fund_pool(env, token, reviewer_share);
    }
    if revenue_share > 0 {
        crate::revenue_share::deposit_revenue(env, token, revenue_share);
    }
    if treasury_share > 0 {
        Storage::add_fees_collected(env, token, treasury_share);
    }

    let net = gross_amount
        .checked_sub(total_fee)
        .ok_or(ContractError::InvalidInput)?;
    Ok(net)
}

pub fn total_fees_collected(env: &Env, token: &Address) -> i128 {
    Storage::get_fees_collected(env, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;
    use crate::storage::keys::{DataKey, ReviewerRewardKey};
    use crate::types::{ProtocolConfig, ReviewerRewardPool};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    #[test]
    fn test_compute_fee_zero_bps() {
        let result = compute_fee(1_000_000, 0).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compute_fee_one_percent() {
        let result = compute_fee(1_000_000, 100).unwrap();
        assert_eq!(result, 10_000);
    }

    #[test]
    fn test_compute_fee_large_amount() {
        let result = compute_fee(100_000_000, 250).unwrap();
        assert_eq!(result, 2_500_000);
    }

    #[test]
    fn test_compute_fee_negative_gross_returns_zero() {
        let result = compute_fee(-1, 100).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_fee_accumulation_across_calls() {
        assert_eq!(compute_fee(1_000_000, 100).unwrap(), 10_000);
        assert_eq!(compute_fee(2_000_000, 100).unwrap(), 20_000);
    }

    #[test]
    fn test_net_of_fee_subtracts_correctly() {
        let net = net_of_fee(1_000_000, 100).unwrap();
        assert_eq!(net, 990_000);
    }

    #[test]
    fn test_deduct_and_split_fee_respects_reviewer_reward_split() {
        let env = Env::default();
        let token = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            // Set a config with 10% protocol fee, 50% to reviewer reward pool
            let mut cfg = default_config();
            cfg.protocol_fee_bps = 1000; // 10%
            cfg.reviewer_reward_pool_bps = 5000; // 50% of fee -> reviewer pool
            cfg.revenue_share_pool_bps = 0;
            Storage::set_protocol_config(&env, &cfg);

            let gross: i128 = 1_000_000;
            let net = deduct_and_split_fee(&env, &token, gross).unwrap();

            // 10% fee = 100_000
            // 50% of fee = 50_000 to reviewer pool
            assert_eq!(net, 900_000);

            let pool_key = DataKey::ReviewerReward(ReviewerRewardKey::Pool(token.clone()));
            let pool: ReviewerRewardPool = env
                .storage()
                .persistent()
                .get(&pool_key)
                .expect("pool should exist");
            assert_eq!(pool.balance, 50_000);

            // The other 50_000 goes to treasury fees collected
            let treasury_fees = Storage::get_fees_collected(&env, &token);
            assert_eq!(treasury_fees, 50_000);
        });
    }

    #[test]
    fn test_deduct_and_split_fee_respects_both_splits() {
        let env = Env::default();
        let token = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let mut cfg = default_config();
            cfg.protocol_fee_bps = 1000; // 10%
            cfg.reviewer_reward_pool_bps = 3000; // 30% of fee -> reviewer
            cfg.revenue_share_pool_bps = 2000; // 20% of fee -> revenue share
            Storage::set_protocol_config(&env, &cfg);

            let gross: i128 = 1_000_000;
            let net = deduct_and_split_fee(&env, &token, gross).unwrap();

            // 10% fee = 100_000
            // 30% of fee = 30_000 to reviewer
            // 20% of fee = 20_000 to revenue share
            // 50% of fee = 50_000 to treasury
            assert_eq!(net, 900_000);

            let pool_key = DataKey::ReviewerReward(ReviewerRewardKey::Pool(token.clone()));
            let pool: ReviewerRewardPool = env
                .storage()
                .persistent()
                .get(&pool_key)
                .expect("pool should exist");
            assert_eq!(pool.balance, 30_000);

            let treasury_fees = Storage::get_fees_collected(&env, &token);
            assert_eq!(treasury_fees, 50_000);
        });
    }

    #[test]
    fn test_deduct_and_split_zero_fee_no_ops() {
        let env = Env::default();
        let token = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let mut cfg = default_config();
            cfg.protocol_fee_bps = 0;
            Storage::set_protocol_config(&env, &cfg);

            let net = deduct_and_split_fee(&env, &token, 1_000_000).unwrap();
            assert_eq!(net, 1_000_000);
        });
    }

    #[test]
    fn test_deduct_and_split_no_revenue_share_leaves_more_to_treasury() {
        let env = Env::default();
        let token = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let mut cfg = default_config();
            cfg.protocol_fee_bps = 500; // 5%
            cfg.reviewer_reward_pool_bps = 2000; // 20% of fee -> reviewer
            cfg.revenue_share_pool_bps = 0; // 0% to revenue share
            Storage::set_protocol_config(&env, &cfg);

            let gross: i128 = 100_000;
            let net = deduct_and_split_fee(&env, &token, gross).unwrap();

            // 5% fee = 5000
            // 20% of fee = 1000 to reviewer
            // 0% to revenue share
            // 80% = 4000 to treasury
            assert_eq!(net, 95_000);

            let treasury_fees = Storage::get_fees_collected(&env, &token);
            assert_eq!(treasury_fees, 4000);
        });
    }
}
