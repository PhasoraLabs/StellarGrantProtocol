use soroban_sdk::{token, Address, Env};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::ContractError;

#[allow(dead_code)]
fn basis_points_of(amount: i128, bps: u32) -> Result<i128, ContractError> {
    amount
        .checked_mul(bps as i128)
        .ok_or(ContractError::InvalidInput)?
        .checked_div(10_000)
        .ok_or(ContractError::InvalidInput)
}

#[allow(dead_code)]
pub fn compute_fee(gross: i128, fee_bps: u32) -> Result<i128, ContractError> {
    if fee_bps == 0 || gross <= 0 {
        return Ok(0);
    }
    basis_points_of(gross, fee_bps)
}

#[allow(dead_code)]
pub fn deduct_and_transfer(
    env: &Env,
    gross: i128,
    token: &Address,
    treasury: &Address,
    grant_id: u64,
    milestone_idx: u32,
    fee_bps: u32,
) -> Result<i128, ContractError> {
    let fee = compute_fee(gross, fee_bps)?;
    if fee <= 0 {
        return Ok(gross);
    }

    let config = crate::config::get_config(env);
    let reviewer_reward_bps = config.reviewer_reward_pool_bps;

    // Split fee: reviewer reward pool + treasury
    let reviewer_reward_amount = basis_points_of(fee, reviewer_reward_bps)?;
    let treasury_amount = fee
        .checked_sub(reviewer_reward_amount)
        .ok_or(ContractError::InvalidInput)?;

    // Transfer treasury share to treasury
    if treasury_amount > 0 {
        token::Client::new(env, token).transfer(
            &env.current_contract_address(),
            treasury,
            &treasury_amount,
        );
    }

    // Fund reviewer reward pool
    if reviewer_reward_amount > 0 {
        crate::reviewer_reward::fund_pool(env, token, reviewer_reward_amount);
    }

    Storage::add_fees_collected(env, token, fee);

    Events::emit_fee_collected(
        env,
        grant_id,
        milestone_idx,
        fee,
        token.clone(),
        treasury.clone(),
    );

    // Issue #569: if the grant owner was referred, accrue a share of this fee to
    // their referrer's pending rewards. No-op when no referral record exists.
    if let Some(grant) = Storage::get_grant(env, grant_id) {
        crate::referral::trigger_reward(env, &grant.owner, token, fee)?;
    }

    gross.checked_sub(fee).ok_or(ContractError::InvalidInput)
}

pub fn total_fees_collected(env: &Env, token: &Address) -> i128 {
    Storage::get_fees_collected(env, token)
}

#[allow(dead_code)]
pub fn set_treasury(env: &Env, admin: &Address, treasury: &Address) -> Result<(), ContractError> {
    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    Storage::set_treasury(env, treasury);
    Ok(())
}

#[allow(dead_code)]
pub fn get_treasury(env: &Env) -> Result<Address, ContractError> {
    Storage::get_treasury(env).ok_or(ContractError::InvalidInput)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
