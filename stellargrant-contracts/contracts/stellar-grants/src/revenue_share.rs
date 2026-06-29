use soroban_sdk::{contractevent, token, Address, Env, Vec};

use crate::constants::EPOCH_DURATION_SECONDS;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::RevenueEpoch;

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

fn epoch_bounds(epoch_id: u32) -> (u64, u64) {
    let start_at = (epoch_id as u64).saturating_mul(EPOCH_DURATION_SECONDS);
    let end_at = start_at.saturating_add(EPOCH_DURATION_SECONDS);
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

/// Deposit revenue into the current epoch pool. Called by fees.rs.
pub fn deposit_revenue(env: &Env, token: &Address, amount: i128) {
    if amount <= 0 {
        return;
    }

    let epoch_id = epoch_id_for(env);
    let mut epoch =
        Storage::get_revenue_epoch(env, epoch_id).unwrap_or_else(|| empty_epoch(epoch_id, token));
    epoch.total_revenue = epoch.total_revenue.saturating_add(amount);
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
        .saturating_mul(epoch.total_revenue)
        .checked_div(epoch.total_stake_weight)
        .unwrap_or(0)
}

/// Staker claims their revenue share for a finalized epoch.
pub fn claim(env: &Env, staker: &Address, epoch_id: u32) -> Result<i128, ContractError> {
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

    epoch.claimed_count = epoch.claimed_count.saturating_add(1);
    Storage::set_revenue_epoch(env, &epoch);

    let token_client = token::Client::new(env, &epoch.token);
    token_client.transfer(&env.current_contract_address(), staker, &claimable);

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
}
