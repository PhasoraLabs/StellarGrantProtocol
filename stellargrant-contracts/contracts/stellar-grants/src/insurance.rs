use soroban_sdk::{contractevent, token, Address, Env, String};

use crate::constants::{
    BASIS_POINTS_SCALE, DEFAULT_INSURANCE_DURATION_LEDGERS, DEFAULT_INSURANCE_PREMIUM_RATE_BPS,
};
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{InsuranceClaim, InsuranceClaimStatus, InsurancePolicy};

// ── Events ────────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyPurchased {
    pub grant_id: u64,
    pub policyholder: Address,
    pub coverage_amount: i128,
    pub premium_paid: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimFiled {
    pub claim_id: u32,
    pub grant_id: u64,
    pub claimant: Address,
    pub claimed_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimApproved {
    pub claim_id: u32,
    pub payout_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimRejected {
    pub claim_id: u32,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Purchase insurance for a grant. Premium = coverage * premium_rate_bps / BASIS_POINTS_SCALE.
pub fn purchase_policy(
    env: &Env,
    policyholder: &Address,
    grant_id: u64,
    token: &Address,
    coverage_amount: i128,
) -> Result<InsurancePolicy, ContractError> {
    crate::reentrancy::protect(env)?;
    policyholder.require_auth();

    if coverage_amount <= 0 {
        return Err(ContractError::InvalidInput);
    }

    if Storage::get_insurance_policy(env, grant_id).is_some() {
        return Err(ContractError::AlreadyRegistered);
    }

    let premium = coverage_amount
        .checked_mul(DEFAULT_INSURANCE_PREMIUM_RATE_BPS as i128)
        .ok_or(ContractError::InvalidInput)?
        .checked_div(BASIS_POINTS_SCALE as i128)
        .ok_or(ContractError::InvalidInput)?;

    if premium > 0 {
        let token_client = token::Client::new(env, token);
        crate::reentrancy::protect_external_call(env, || {
            token_client.transfer(policyholder, env.current_contract_address(), &premium);
            Ok(())
        })?;
    }

    // Add premium to pool
    let pool_balance = Storage::get_insurance_pool(env, token);
    Storage::set_insurance_pool(
        env,
        token,
        pool_balance
            .checked_add(premium)
            .ok_or(ContractError::InvalidInput)?,
    );

    let now = env.ledger().timestamp();
    let expires_at = now
        .checked_add(DEFAULT_INSURANCE_DURATION_LEDGERS as u64)
        .ok_or(ContractError::InvalidInput)?;

    let policy = InsurancePolicy {
        grant_id,
        policyholder: policyholder.clone(),
        token: token.clone(),
        coverage_amount,
        premium_paid: premium,
        issued_at: now,
        expires_at,
        active: true,
    };
    Storage::set_insurance_policy(env, &policy);

    PolicyPurchased {
        grant_id,
        policyholder: policyholder.clone(),
        coverage_amount,
        premium_paid: premium,
    }
    .publish(env);

    Ok(policy)
}

/// File an insurance claim for a grant.
pub fn file_claim(
    env: &Env,
    claimant: &Address,
    grant_id: u64,
    claimed_amount: i128,
    reason: String,
) -> Result<u32, ContractError> {
    claimant.require_auth();

    if claimed_amount <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let policy =
        Storage::get_insurance_policy(env, grant_id).ok_or(ContractError::PolicyNotFound)?;

    if !policy.active {
        return Err(ContractError::PolicyInactive);
    }
    if env.ledger().timestamp() > policy.expires_at {
        return Err(ContractError::PolicyExpired);
    }

    let claim_id = Storage::next_claim_id(env);
    let claim = InsuranceClaim {
        id: claim_id,
        policy_grant_id: grant_id,
        claimant: claimant.clone(),
        claimed_amount,
        reason: reason.clone(),
        status: InsuranceClaimStatus::Submitted,
        submitted_at: env.ledger().timestamp(),
        resolved_at: None,
        payout_amount: None,
    };
    Storage::set_insurance_claim(env, &claim);

    ClaimFiled {
        claim_id,
        grant_id,
        claimant: claimant.clone(),
        claimed_amount,
    }
    .publish(env);

    Ok(claim_id)
}

/// Approve and pay out a claim. Admin or DAO only.
pub fn approve_claim(
    env: &Env,
    admin: &Address,
    claim_id: u32,
    payout_amount: i128,
) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;
    admin.require_auth();

    if payout_amount <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let mut claim =
        Storage::get_insurance_claim(env, claim_id).ok_or(ContractError::ClaimNotFound)?;

    if claim.status != InsuranceClaimStatus::Submitted
        && claim.status != InsuranceClaimStatus::UnderReview
    {
        return Err(ContractError::ClaimAlreadyResolved);
    }

    let policy = Storage::get_insurance_policy(env, claim.policy_grant_id)
        .ok_or(ContractError::PolicyNotFound)?;

    let pool_balance = Storage::get_insurance_pool(env, &policy.token);

    // Payout capped at min(claimed_amount, coverage_amount, pool_balance)
    let actual_payout = payout_amount
        .min(claim.claimed_amount)
        .min(policy.coverage_amount)
        .min(pool_balance);

    if actual_payout <= 0 {
        return Err(ContractError::InsufficientPoolBalance);
    }

    Storage::set_insurance_pool(
        env,
        &policy.token,
        pool_balance
            .checked_sub(actual_payout)
            .ok_or(ContractError::InvalidInput)?,
    );

    claim.status = InsuranceClaimStatus::Paid;
    claim.resolved_at = Some(env.ledger().timestamp());
    claim.payout_amount = Some(actual_payout);
    Storage::set_insurance_claim(env, &claim);

    let token_client = token::Client::new(env, &policy.token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(
            &env.current_contract_address(),
            &claim.claimant,
            &actual_payout,
        );
        Ok(())
    })?;

    ClaimApproved {
        claim_id,
        payout_amount: actual_payout,
    }
    .publish(env);

    Ok(())
}

/// Reject a claim. Admin or DAO only.
pub fn reject_claim(env: &Env, admin: &Address, claim_id: u32) -> Result<(), ContractError> {
    admin.require_auth();

    let mut claim =
        Storage::get_insurance_claim(env, claim_id).ok_or(ContractError::ClaimNotFound)?;

    if claim.status != InsuranceClaimStatus::Submitted
        && claim.status != InsuranceClaimStatus::UnderReview
    {
        return Err(ContractError::ClaimAlreadyResolved);
    }

    claim.status = InsuranceClaimStatus::Rejected;
    claim.resolved_at = Some(env.ledger().timestamp());
    Storage::set_insurance_claim(env, &claim);

    ClaimRejected { claim_id }.publish(env);

    Ok(())
}

/// Return total funds in the insurance pool for a given token.
pub fn pool_balance(env: &Env, token: &Address) -> i128 {
    Storage::get_insurance_pool(env, token)
}

/// Return the insurance policy for a grant.
pub fn get_policy(env: &Env, grant_id: u64) -> Option<InsurancePolicy> {
    Storage::get_insurance_policy(env, grant_id)
}

/// Return a claim by id.
pub fn get_claim(env: &Env, claim_id: u32) -> Result<InsuranceClaim, ContractError> {
    Storage::get_insurance_claim(env, claim_id).ok_or(ContractError::ClaimNotFound)
}

// ── Unit Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token::StellarAssetClient, Address, Env, String};

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let policyholder = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        let stellar_asset = StellarAssetClient::new(&env, &token_contract);
        stellar_asset.mint(&policyholder, &10_000_000);
        (env, admin, policyholder, token_contract)
    }

    #[test]
    fn test_premium_calculated_correctly() {
        let (env, _admin, policyholder, token) = setup();
        let coverage = 1_000_000i128;
        // Expected premium = 1_000_000 * 50 / 10_000 = 5_000
        let policy = purchase_policy(&env, &policyholder, 1, &token, coverage).unwrap();
        assert_eq!(policy.premium_paid, 5_000);
        assert_eq!(pool_balance(&env, &token), 5_000);
    }

    #[test]
    fn test_claim_exceeding_coverage_is_capped() {
        let (env, admin, policyholder, token) = setup();
        let coverage = 1_000_000i128;
        purchase_policy(&env, &policyholder, 1, &token, coverage).unwrap();

        // Mint extra into pool so pool isn't the bottleneck
        let stellar_asset = StellarAssetClient::new(&env, &token);
        stellar_asset.mint(&env.current_contract_address(), &2_000_000);
        Storage::set_insurance_pool(&env, &token, 2_000_000);

        let reason = String::from_str(&env, "bug");
        let claim_id = file_claim(&env, &policyholder, 1, 2_000_000, reason).unwrap();
        approve_claim(&env, &admin, claim_id, 2_000_000).unwrap();

        let claim = get_claim(&env, claim_id).unwrap();
        // Capped at coverage_amount = 1_000_000
        assert_eq!(claim.payout_amount, Some(1_000_000));
    }

    #[test]
    fn test_pool_insufficient_payout_is_capped() {
        let (env, admin, policyholder, token) = setup();
        purchase_policy(&env, &policyholder, 1, &token, 1_000_000).unwrap();
        // Pool currently equals the premium (5_000). Claim for full coverage.
        let reason = String::from_str(&env, "issue");
        let claim_id = file_claim(&env, &policyholder, 1, 1_000_000, reason).unwrap();
        approve_claim(&env, &admin, claim_id, 1_000_000).unwrap();
        let claim = get_claim(&env, claim_id).unwrap();
        // Capped at pool balance = 5_000
        assert_eq!(claim.payout_amount, Some(5_000));
    }

    #[test]
    fn test_expired_policy_claim_rejected() {
        let (env, _admin, policyholder, token) = setup();
        purchase_policy(&env, &policyholder, 1, &token, 1_000_000).unwrap();
        // Advance time past policy expiry
        env.ledger().with_mut(|li| {
            li.timestamp = li.timestamp + DEFAULT_INSURANCE_DURATION_LEDGERS as u64 + 1;
        });
        let reason = String::from_str(&env, "late");
        let err = file_claim(&env, &policyholder, 1, 500_000, reason).unwrap_err();
        assert_eq!(err, ContractError::PolicyExpired);
    }
}
