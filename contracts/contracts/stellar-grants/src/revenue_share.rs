use soroban_sdk::{contractevent, token, Address, Env, Vec};

use crate::constants::EPOCH_DURATION_SECONDS;
use crate::errors::ContractError;
use crate::math::safe_add;
use crate::storage::Storage;
use crate::types::{RevenueEpoch, StakerEpochRecord};

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochFinalized {
    pub epoch_id: u32,
    pub total_revenue: i128,
    pub total_stake_weight: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueClaimed {
    pub staker: Address,
    pub epoch_id: u32,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StakeRegistered {
    pub staker: Address,
    pub epoch_id: u32,
    pub weight: i128,
}

fn epoch_bounds(epoch_id: u32) -> (u64, u64) {
    let start_at = (epoch_id as u64)
        .checked_mul(EPOCH_DURATION_SECONDS)
        .unwrap_or(u64::MAX);
    let end_at = start_at
        .checked_add(EPOCH_DURATION_SECONDS)
        .unwrap_or(u64::MAX);
    (start_at, end_at)
}

fn epoch_id_for(env: &Env) -> u32 {
    (env.ledger().timestamp() / EPOCH_DURATION_SECONDS) as u32
}

fn empty_epoch(epoch_id: u32, token: &Address) -> RevenueEpoch {
    let (start_at, end_at) = epoch_bounds(epoch_id);
    RevenueEpoch {
        id: epoch_id,
        start_at,
        end_at,
        total_revenue: 0,
        token: token.clone(),
        total_stake_weight: 0,
        finalized: false,
        claimed_count: 0,
    }
}

/// Register a staker's stake weight for an epoch. This is the missing path
/// that populates `StakerEpochRecord`/`total_stake_weight` so that `claim`
/// can actually succeed after an epoch is finalized.
pub fn stake(
    env: &Env,
    staker: &Address,
    epoch_id: u32,
    weight: i128,
) -> Result<(), ContractError> {
    staker.require_auth();
    if weight <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let mut record = Storage::get_staker_epoch_record(env, staker, epoch_id).unwrap_or_else(|| {
        StakerEpochRecord {
            staker: staker.clone(),
            epoch_id,
            stake_weight: 0,
            claimable: 0,
            claimed: false,
            claimed_at: None,
        }
    });
    record.stake_weight = record
        .stake_weight
        .checked_add(weight)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_staker_epoch_record(env, &record);

    let mut epoch = Storage::get_revenue_epoch(env, epoch_id)
        .unwrap_or_else(|| empty_epoch(epoch_id, &env.current_contract_address()));
    epoch.total_stake_weight = epoch
        .total_stake_weight
        .checked_add(weight)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_revenue_epoch(env, &epoch);

    StakeRegistered {
        staker: staker.clone(),
        epoch_id,
        weight,
    }
    .publish(env);

    Ok(())
}

/// Deposit revenue into the current epoch pool. Called by fees.rs.
pub fn deposit_revenue(env: &Env, token: &Address, amount: i128) {
    if amount <= 0 {
        return;
    }

    let epoch_id = epoch_id_for(env);
    let mut epoch =
        Storage::get_revenue_epoch(env, epoch_id).unwrap_or_else(|| empty_epoch(epoch_id, token));

    if epoch.finalized {
        let next_epoch_id = epoch_id.saturating_add(1);
        let mut next_epoch = Storage::get_revenue_epoch(env, next_epoch_id)
            .unwrap_or_else(|| empty_epoch(next_epoch_id, token));
        next_epoch.total_revenue = safe_add(next_epoch.total_revenue, amount).unwrap_or(i128::MAX);
        next_epoch.token = token.clone();
        Storage::set_revenue_epoch(env, &next_epoch);
        return;
    }

    epoch.total_revenue = safe_add(epoch.total_revenue, amount).unwrap_or(i128::MAX);
    epoch.token = token.clone();
    Storage::set_revenue_epoch(env, &epoch);
}

/// Finalize an epoch (admin or permissionless after epoch ends).
pub fn finalize_epoch(env: &Env, caller: &Address, epoch_id: u32) -> Result<(), ContractError> {
    let mut epoch = Storage::get_revenue_epoch(env, epoch_id)
        .unwrap_or_else(|| empty_epoch(epoch_id, &env.current_contract_address()));
    if epoch.finalized {
        return Err(ContractError::InvalidState);
    }

    let now = env.ledger().timestamp();
    if now <= epoch.end_at {
        caller.require_auth();
        if Storage::get_global_admin(env) != Some(caller.clone()) {
            return Err(ContractError::Unauthorized);
        }
    }

    epoch.finalized = true;
    Storage::set_revenue_epoch(env, &epoch);

    EpochFinalized {
        epoch_id,
        total_revenue: epoch.total_revenue,
        total_stake_weight: epoch.total_stake_weight,
    }
    .publish(env);

    Ok(())
}

/// Compute a staker's claimable revenue for an epoch.
pub fn compute_claim(env: &Env, staker: &Address, epoch_id: u32) -> i128 {
    let epoch = match Storage::get_revenue_epoch(env, epoch_id) {
        Some(epoch) if epoch.finalized && epoch.total_revenue > 0 => epoch,
        _ => return 0,
    };
    if epoch.total_stake_weight <= 0 {
        return 0;
    }

    let record = match Storage::get_staker_epoch_record(env, staker, epoch_id) {
        Some(record) if !record.claimed && record.stake_weight > 0 => record,
        _ => return 0,
    };
    if record.claimable > 0 {
        return record.claimable;
    }

    record
        .stake_weight
        .checked_mul(epoch.total_revenue)
        .and_then(|product| product.checked_div(epoch.total_stake_weight))
        .unwrap_or(0)
}

/// Staker claims their revenue share for a finalized epoch.
pub fn claim(env: &Env, staker: &Address, epoch_id: u32) -> Result<i128, ContractError> {
    crate::reentrancy::protect(env)?;
    staker.require_auth();

    let mut epoch = Storage::get_revenue_epoch(env, epoch_id).ok_or(ContractError::InvalidState)?;
    if !epoch.finalized {
        return Err(ContractError::InvalidState);
    }

    let mut record = Storage::get_staker_epoch_record(env, staker, epoch_id)
        .ok_or(ContractError::InvalidState)?;
    if record.claimed {
        return Err(ContractError::InvalidState);
    }

    let claimable = compute_claim(env, staker, epoch_id);
    if claimable <= 0 {
        return Err(ContractError::NoRewardsToClaim);
    }

    record.claimable = claimable;
    record.claimed = true;
    record.claimed_at = Some(env.ledger().timestamp());
    Storage::set_staker_epoch_record(env, &record);

    epoch.claimed_count = epoch.claimed_count.checked_add(1).unwrap_or(u32::MAX);
    Storage::set_revenue_epoch(env, &epoch);

    let token_client = token::Client::new(env, &epoch.token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(&env.current_contract_address(), staker, &claimable);
        Ok(())
    })?;

    RevenueClaimed {
        staker: staker.clone(),
        epoch_id,
        amount: claimable,
    }
    .publish(env);

    Ok(claimable)
}

/// Return the current epoch record.
pub fn current_epoch(env: &Env) -> RevenueEpoch {
    let epoch_id = epoch_id_for(env);
    Storage::get_revenue_epoch(env, epoch_id)
        .unwrap_or_else(|| empty_epoch(epoch_id, &env.current_contract_address()))
}

/// Return all unclaimed epoch IDs for a staker.
pub fn unclaimed_epochs(env: &Env, staker: &Address) -> Vec<u32> {
    let mut ids = Vec::new(env);
    let current_id = epoch_id_for(env);
    for epoch_id in 0..=current_id {
        if compute_claim(env, staker, epoch_id) > 0 {
            ids.push_back(epoch_id);
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StakerEpochRecord;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn with_contract(env: &Env, f: impl FnOnce(Address)) {
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id.clone(), || f(contract_id));
    }

    #[test]
    fn test_compute_claim_proportional_to_weight() {
        let env = Env::default();
        with_contract(&env, |_| {
            let token = Address::generate(&env);
            let staker_a = Address::generate(&env);
            let staker_b = Address::generate(&env);
            let mut epoch = empty_epoch(0, &token);
            epoch.total_revenue = 300;
            epoch.total_stake_weight = 300;
            epoch.finalized = true;
            Storage::set_revenue_epoch(&env, &epoch);
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker_a.clone(),
                    epoch_id: 0,
                    stake_weight: 100,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker_b.clone(),
                    epoch_id: 0,
                    stake_weight: 200,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );

            assert_eq!(compute_claim(&env, &staker_a, 0), 100);
            assert_eq!(compute_claim(&env, &staker_b, 0), 200);
        });
    }

    #[test]
    fn test_unfinalized_epoch_has_no_claim() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let token = Address::generate(&env);
        let staker = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let mut epoch = empty_epoch(0, &token);
            epoch.total_revenue = 100;
            epoch.total_stake_weight = 100;
            Storage::set_revenue_epoch(&env, &epoch);
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker.clone(),
                    epoch_id: 0,
                    stake_weight: 100,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );

            assert_eq!(compute_claim(&env, &staker, 0), 0);
        });

        assert!(client.try_claim_revenue_share(&staker, &0).is_err());
    }

    #[test]
    fn test_unclaimed_epochs_returns_finalized_records() {
        let env = Env::default();
        env.ledger().set_timestamp(EPOCH_DURATION_SECONDS);
        with_contract(&env, |_| {
            let token = Address::generate(&env);
            let staker = Address::generate(&env);
            let mut epoch = empty_epoch(0, &token);
            epoch.total_revenue = 100;
            epoch.total_stake_weight = 100;
            epoch.finalized = true;
            Storage::set_revenue_epoch(&env, &epoch);
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker.clone(),
                    epoch_id: 0,
                    stake_weight: 100,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );

            let ids = unclaimed_epochs(&env, &staker);
            assert_eq!(ids.len(), 1);
            assert_eq!(ids.get(0), Some(0));
        });
    }

    #[test]
    fn test_claim_rejects_reentrant_call_when_guard_is_held() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let token = Address::generate(&env);
        let staker = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let mut epoch = empty_epoch(0, &token);
            epoch.finalized = true;
            epoch.total_revenue = 100;
            epoch.total_stake_weight = 100;
            Storage::set_revenue_epoch(&env, &epoch);
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker.clone(),
                    epoch_id: 0,
                    stake_weight: 100,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );

            env.storage()
                .temporary()
                .set(&crate::reentrancy::ReentrancyKey::ExternalCallGuard, &());

            let err = claim(&env, &staker, 0).unwrap_err();
            assert_eq!(err, ContractError::Reentrancy);
        });
    }

    #[test]
    fn test_claim_after_epoch_finalized_and_rejects_double_claim() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let token_admin_addr = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(token_admin_addr.clone())
            .address();
        let token_admin = token::StellarAssetClient::new(&env, &token);
        let staker = Address::generate(&env);

        token_admin.mint(&contract_id, &100);

        env.as_contract(&contract_id, || {
            let mut epoch = empty_epoch(0, &token);
            epoch.total_revenue = 100;
            epoch.total_stake_weight = 100;
            epoch.finalized = true;
            Storage::set_revenue_epoch(&env, &epoch);
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker.clone(),
                    epoch_id: 0,
                    stake_weight: 100,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );
        });

        assert_eq!(client.claim_revenue_share(&staker, &0), 100);
        assert!(client.try_claim_revenue_share(&staker, &0).is_err());
    }

    #[test]
    fn test_deposit_revenue_after_finalized() {
        let env = Env::default();
        let token = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let epoch_id = epoch_id_for(&env);

            // finalize epoch
            let mut epoch = empty_epoch(epoch_id, &token);
            epoch.finalized = true;
            Storage::set_revenue_epoch(&env, &epoch);

            // try to deposit, it should route to next epoch
            deposit_revenue(&env, &token, 500);

            // current epoch shouldn't change
            let curr = Storage::get_revenue_epoch(&env, epoch_id).unwrap();
            assert_eq!(curr.total_revenue, 0);

            // next epoch should receive funds
            let next_epoch_id = epoch_id + 1;
            let next = Storage::get_revenue_epoch(&env, next_epoch_id).unwrap();
            assert_eq!(next.total_revenue, 500);
        });
    }

    // ── Overflow / checked-math tests ────────────────────────────────────────

    /// epoch_bounds: when epoch_id is so large that start_at would overflow u64,
    /// the helper clamps to u64::MAX rather than wrapping.
    #[test]
    fn test_epoch_bounds_overflow_clamps_to_max() {
        // u32::MAX * EPOCH_DURATION_SECONDS overflows u64
        let (start, end) = epoch_bounds(u32::MAX);
        assert_eq!(start, u64::MAX);
        assert_eq!(end, u64::MAX);
    }

    /// epoch_bounds: a normal epoch_id produces the expected timestamps.
    #[test]
    fn test_epoch_bounds_normal() {
        let (start, end) = epoch_bounds(1);
        assert_eq!(start, EPOCH_DURATION_SECONDS);
        assert_eq!(end, EPOCH_DURATION_SECONDS * 2);
    }

    /// deposit_revenue: accumulating near i128::MAX must not silently wrap;
    /// it saturates at i128::MAX via safe_add.
    #[test]
    fn test_deposit_revenue_overflow_saturates() {
        let env = Env::default();
        let token = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let epoch_id = epoch_id_for(&env);
            let mut epoch = empty_epoch(epoch_id, &token);
            epoch.total_revenue = i128::MAX;
            Storage::set_revenue_epoch(&env, &epoch);

            // Adding 1 to i128::MAX: safe_add returns Err, so we fall back to i128::MAX.
            deposit_revenue(&env, &token, 1);

            let stored = Storage::get_revenue_epoch(&env, epoch_id).unwrap();
            assert_eq!(stored.total_revenue, i128::MAX);
        });
    }

    /// deposit_revenue into next epoch when current is finalized must also
    /// saturate rather than wrap.
    #[test]
    fn test_deposit_revenue_next_epoch_overflow_saturates() {
        let env = Env::default();
        let token = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let epoch_id = epoch_id_for(&env);
            let next_epoch_id = epoch_id.saturating_add(1);

            let mut curr = empty_epoch(epoch_id, &token);
            curr.finalized = true;
            Storage::set_revenue_epoch(&env, &curr);

            let mut next = empty_epoch(next_epoch_id, &token);
            next.total_revenue = i128::MAX;
            Storage::set_revenue_epoch(&env, &next);

            deposit_revenue(&env, &token, 1);

            let stored = Storage::get_revenue_epoch(&env, next_epoch_id).unwrap();
            assert_eq!(stored.total_revenue, i128::MAX);
        });
    }

    /// compute_claim: when stake_weight * total_revenue would overflow i128,
    /// checked_mul returns None and compute_claim returns 0 (safe fallback),
    /// rather than producing a silently-wrong value from saturating_mul.
    #[test]
    fn test_compute_claim_overflow_returns_zero() {
        let env = Env::default();
        with_contract(&env, |_| {
            let token = Address::generate(&env);
            let staker = Address::generate(&env);

            // stake_weight * total_revenue overflows i128
            let mut epoch = empty_epoch(0, &token);
            epoch.total_revenue = i128::MAX;
            epoch.total_stake_weight = i128::MAX;
            epoch.finalized = true;
            Storage::set_revenue_epoch(&env, &epoch);
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker.clone(),
                    epoch_id: 0,
                    stake_weight: i128::MAX,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );

            // With saturating_mul this would return i128::MAX (wrong).
            // With checked_mul this correctly returns 0 (overflow detected).
            assert_eq!(compute_claim(&env, &staker, 0), 0);
        });
    }

    /// compute_claim: large but non-overflowing values produce a correct result.
    #[test]
    fn test_compute_claim_large_values_correct() {
        let env = Env::default();
        with_contract(&env, |_| {
            let token = Address::generate(&env);
            let staker = Address::generate(&env);

            // 1_000_000 weight out of 2_000_000 total, revenue = 1_000_000
            // expected: 1_000_000 * 1_000_000 / 2_000_000 = 500_000
            let mut epoch = empty_epoch(0, &token);
            epoch.total_revenue = 1_000_000;
            epoch.total_stake_weight = 2_000_000;
            epoch.finalized = true;
            Storage::set_revenue_epoch(&env, &epoch);
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker.clone(),
                    epoch_id: 0,
                    stake_weight: 1_000_000,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );

            assert_eq!(compute_claim(&env, &staker, 0), 500_000);
        });
    }

    /// stake: accumulating weights that would overflow i128 is rejected with
    /// InvalidInput, ensuring the per-staker record stays consistent.
    #[test]
    fn test_stake_weight_overflow_returns_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let staker = Address::generate(&env);

            // Pre-load a record whose stake_weight is already i128::MAX.
            Storage::set_staker_epoch_record(
                &env,
                &StakerEpochRecord {
                    staker: staker.clone(),
                    epoch_id: 0,
                    stake_weight: i128::MAX,
                    claimable: 0,
                    claimed: false,
                    claimed_at: None,
                },
            );

            // Adding any positive weight must overflow and return InvalidInput.
            let err = stake(&env, &staker, 0, 1).unwrap_err();
            assert_eq!(err, ContractError::InvalidInput);
        });
    }
}
