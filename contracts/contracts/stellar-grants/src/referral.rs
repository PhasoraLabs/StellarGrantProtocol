use soroban_sdk::{contractevent, token, Address, Bytes, Env};

use crate::config;
use crate::constants::BASIS_POINTS_SCALE;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ReferralCode, ReferralRecord};

// ── Events ──────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralCodeCreated {
    pub referrer: Address,
    pub code_hash: Bytes,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralApplied {
    pub referred: Address,
    pub referrer: Address,
    pub code_hash: Bytes,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralRewardEarned {
    pub referrer: Address,
    pub referred: Address,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralRewardsClaimed {
    pub referrer: Address,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralCodeDeactivated {
    pub referrer: Address,
    pub code_hash: Bytes,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn is_registered(env: &Env, addr: &Address) -> bool {
    Storage::get_contributor(env, addr.clone()).is_some()
        || Storage::get_reviewer_profile(env, addr).is_some()
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Create a referral code. Any registered contributor or reviewer.
/// Returns the SHA-256 code hash that the referrer shares with new joiners.
pub fn create_code(
    env: &Env,
    referrer: &Address,
    expires_at: Option<u64>,
    max_uses: Option<u32>,
) -> Result<Bytes, ContractError> {
    referrer.require_auth();

    if !is_registered(env, referrer) {
        return Err(ContractError::Unauthorized);
    }

    let now = env.ledger().timestamp();
    if let Some(exp) = expires_at {
        if exp <= now {
            return Err(ContractError::InvalidInput);
        }
    }
    if let Some(mu) = max_uses {
        if mu == 0 {
            return Err(ContractError::InvalidInput);
        }
    }

    // Derive a unique, unguessable code hash from PRNG salt + ledger context.
    let salt: u64 = env.prng().gen();
    let mut plain = Bytes::new(env);
    plain.extend_from_array(&salt.to_be_bytes());
    plain.extend_from_array(&now.to_be_bytes());
    plain.extend_from_array(&(env.ledger().sequence() as u64).to_be_bytes());
    let code_hash: Bytes = env.crypto().sha256(&plain).into();

    let code = ReferralCode {
        code_hash: code_hash.clone(),
        referrer: referrer.clone(),
        created_at: now,
        expires_at,
        max_uses,
        uses: 0,
        is_active: true,
    };
    Storage::set_referral_code(env, &code);

    ReferralCodeCreated {
        referrer: referrer.clone(),
        code_hash: code_hash.clone(),
    }
    .publish(env);

    Ok(code_hash)
}

/// Apply a referral code on registration. Called once per referred address.
pub fn apply_code(env: &Env, referred: &Address, code_hash: &Bytes) -> Result<(), ContractError> {
    referred.require_auth();

    let mut code =
        Storage::get_referral_code(env, code_hash).ok_or(ContractError::ReferralCodeNotFound)?;

    if !code.is_active {
        return Err(ContractError::ReferralCodeInactive);
    }

    let now = env.ledger().timestamp();
    if let Some(exp) = code.expires_at {
        if now > exp {
            return Err(ContractError::ReferralCodeExpired);
        }
    }
    if let Some(mu) = code.max_uses {
        if code.uses >= mu {
            return Err(ContractError::ReferralCodeExhausted);
        }
    }

    // Self-referral and double-referral are rejected.
    if code.referrer == *referred {
        return Err(ContractError::InvalidInput);
    }
    if Storage::get_referral_record(env, referred).is_some() {
        return Err(ContractError::AlreadyReferred);
    }

    code.uses = code.uses.saturating_add(1);
    Storage::set_referral_code(env, &code);

    let record = ReferralRecord {
        referred: referred.clone(),
        referrer: code.referrer.clone(),
        code_hash: code_hash.clone(),
        referred_at: now,
        first_action_at: None,
        reward_paid: false,
    };
    Storage::set_referral_record(env, &record);

    ReferralApplied {
        referred: referred.clone(),
        referrer: code.referrer.clone(),
        code_hash: code_hash.clone(),
    }
    .publish(env);

    Ok(())
}

/// Trigger reward after referred address completes its first qualifying action.
/// No-op (Ok) if there is no referral record or the reward was already paid.
pub fn trigger_reward(
    env: &Env,
    referred: &Address,
    token: &Address,
    fee_amount: i128,
) -> Result<(), ContractError> {
    let mut record = match Storage::get_referral_record(env, referred) {
        Some(r) => r,
        None => return Ok(()),
    };

    if record.reward_paid {
        return Ok(());
    }

    let referral_bps = config::get_config(env).referral_fee_bps;
    let reward = if fee_amount > 0 && referral_bps > 0 {
        fee_amount
            .checked_mul(referral_bps as i128)
            .ok_or(ContractError::InvalidInput)?
            .checked_div(BASIS_POINTS_SCALE as i128)
            .ok_or(ContractError::InvalidInput)?
    } else {
        0
    };

    let now = env.ledger().timestamp();
    record.first_action_at = Some(now);
    record.reward_paid = true;
    Storage::set_referral_record(env, &record);

    if reward > 0 {
        let pending = Storage::get_referral_rewards(env, &record.referrer, token);
        Storage::set_referral_rewards(
            env,
            &record.referrer,
            token,
            pending
                .checked_add(reward)
                .ok_or(ContractError::InvalidInput)?,
        );

        ReferralRewardEarned {
            referrer: record.referrer.clone(),
            referred: referred.clone(),
            token: token.clone(),
            amount: reward,
        }
        .publish(env);
    }

    Ok(())
}

/// Referrer claims accumulated rewards for a token.
pub fn claim_rewards(
    env: &Env,
    referrer: &Address,
    token: &Address,
) -> Result<i128, ContractError> {
    crate::reentrancy::protect(env)?;
    referrer.require_auth();

    let pending = Storage::get_referral_rewards(env, referrer, token);
    if pending <= 0 {
        return Err(ContractError::NoRewardsToClaim);
    }

    Storage::set_referral_rewards(env, referrer, token, 0);

    crate::reentrancy::protect_external_call(env, || {
        token::Client::new(env, token).transfer(
            &env.current_contract_address(),
            referrer,
            &pending,
        );
        Ok(())
    })?;

    ReferralRewardsClaimed {
        referrer: referrer.clone(),
        token: token.clone(),
        amount: pending,
    }
    .publish(env);

    Ok(pending)
}

/// Return total unclaimed rewards for a referrer and token.
pub fn pending_rewards(env: &Env, referrer: &Address, token: &Address) -> i128 {
    Storage::get_referral_rewards(env, referrer, token)
}

/// Return the referral record for a referred address.
pub fn get_record(env: &Env, referred: &Address) -> Option<ReferralRecord> {
    Storage::get_referral_record(env, referred)
}

/// Deactivate a referral code. Creator or admin only.
pub fn deactivate_code(
    env: &Env,
    caller: &Address,
    code_hash: &Bytes,
) -> Result<(), ContractError> {
    caller.require_auth();

    let mut code =
        Storage::get_referral_code(env, code_hash).ok_or(ContractError::ReferralCodeNotFound)?;

    let is_creator = code.referrer == *caller;
    let is_admin = Storage::get_global_admin(env) == Some(caller.clone());
    if !is_creator && !is_admin {
        return Err(ContractError::Unauthorized);
    }

    code.is_active = false;
    Storage::set_referral_code(env, &code);

    ReferralCodeDeactivated {
        referrer: code.referrer.clone(),
        code_hash: code_hash.clone(),
    }
    .publish(env);

    Ok(())
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::types::AcceptanceCriteria;
    use crate::{StellarGrantsContract, StellarGrantsContractClient};
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::{vec, String, Vec};

    struct Fixture<'a> {
        env: Env,
        client: StellarGrantsContractClient<'a>,
        token: Address,
        owner: Address,
        referrer: Address,
        reviewer: Address,
        funder: Address,
    }

    fn setup<'a>() -> Fixture<'a> {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();

        // The payout path reads the persisted protocol config, so publish the
        // defaults (1% protocol fee, 10% of that to the referrer).
        client.set_global_admin(&admin, &admin);
        client.update_config(&admin, &config::default_config());

        let funder = Address::generate(&env);
        token::StellarAssetClient::new(&env, &token).mint(&funder, &100_000_000);

        Fixture {
            client,
            token,
            owner: Address::generate(&env),
            referrer: Address::generate(&env),
            reviewer: Address::generate(&env),
            funder,
            env,
        }
    }

    fn register_referrer(f: &Fixture) {
        f.client.reviewer_register(
            &f.referrer,
            &String::from_str(&f.env, "Referrer"),
            &Vec::new(&f.env),
            &None,
        );
    }

    /// Register the referrer (a code requires a registered participant) and
    /// have `owner` join under their code.
    fn apply_referral(f: &Fixture) {
        register_referrer(f);
        let code = f.client.referral_create_code(&f.referrer, &None, &None);
        f.client.referral_apply_code(&f.owner, &code);
    }

    /// Run one grant of `amount` for `owner` all the way to payout: the fee it
    /// generates is the qualifying action that triggers the referral reward.
    fn run_grant_to_payout(f: &Fixture, amount: i128) -> u64 {
        let env = &f.env;
        let grant_id = f.client.grant_create(
            &f.owner,
            &String::from_str(env, "Referred grant"),
            &String::from_str(env, "Description"),
            &f.token,
            &amount,
            &amount,
            &1,
            &vec![env, f.reviewer.clone()],
        );
        f.client.grant_fund(&grant_id, &f.funder, &amount);
        f.client.milestone_submit(
            &grant_id,
            &0,
            &f.owner,
            &String::from_str(env, "Done"),
            &String::from_str(env, "https://proof.url"),
        );

        // An approving vote requires every required criterion to be signed off.
        f.client.checklist_define_criteria(
            &f.owner,
            &grant_id,
            &0,
            &vec![
                env,
                AcceptanceCriteria {
                    idx: 0,
                    description: String::from_str(env, "Ships"),
                    is_required: true,
                },
            ],
        );
        f.client
            .checklist_submit(&f.owner, &grant_id, &0, &vec![env, None]);
        f.client
            .checklist_review_criterion(&f.reviewer, &grant_id, &0, &0, &true);

        f.client
            .milestone_vote(&grant_id, &0, &f.reviewer, &true, &None);
        f.client.grant_complete(&grant_id);
        grant_id
    }

    #[test]
    fn test_code_creation_expiry_and_max_uses() {
        let f = setup();
        register_referrer(&f);

        let expires_at = f.env.ledger().timestamp() + 10;
        let expiring = f
            .client
            .referral_create_code(&f.referrer, &Some(expires_at), &None);
        let stored = f.env.as_contract(&f.client.address, || {
            Storage::get_referral_code(&f.env, &expiring).unwrap()
        });
        assert_eq!(stored.expires_at, Some(expires_at));
        assert!(stored.is_active);

        f.env.ledger().set_timestamp(expires_at + 1);
        assert_eq!(
            f.client
                .try_referral_apply_code(&f.owner, &expiring)
                .unwrap_err(),
            Ok(ContractError::ReferralCodeExpired)
        );

        let limited = f.client.referral_create_code(&f.referrer, &None, &Some(1));
        f.client.referral_apply_code(&f.owner, &limited);
        let another = Address::generate(&f.env);
        assert_eq!(
            f.client
                .try_referral_apply_code(&another, &limited)
                .unwrap_err(),
            Ok(ContractError::ReferralCodeExhausted)
        );
    }

    #[test]
    fn test_self_referral_and_double_apply_are_rejected() {
        let f = setup();
        register_referrer(&f);
        let code = f.client.referral_create_code(&f.referrer, &None, &None);

        assert_eq!(
            f.client
                .try_referral_apply_code(&f.referrer, &code)
                .unwrap_err(),
            Ok(ContractError::InvalidInput)
        );
        f.client.referral_apply_code(&f.owner, &code);
        assert_eq!(
            f.client
                .try_referral_apply_code(&f.owner, &code)
                .unwrap_err(),
            Ok(ContractError::AlreadyReferred)
        );
    }

    #[test]
    fn test_referral_reward_accrues_on_first_qualifying_payout() {
        let f = setup();
        apply_referral(&f);

        assert_eq!(f.client.referral_pending_rewards(&f.referrer, &f.token), 0);

        run_grant_to_payout(&f, 1_000_000);

        // 1% protocol fee on 1_000_000 = 10_000; 10% referral share = 1_000.
        assert_eq!(
            f.client.referral_pending_rewards(&f.referrer, &f.token),
            1_000
        );

        let record = f.client.referral_get_record(&f.owner).unwrap();
        assert!(record.reward_paid);
        assert!(record.first_action_at.is_some());
    }

    #[test]
    fn test_referrer_can_claim_reward_after_referred_payout() {
        let f = setup();
        apply_referral(&f);

        // Before the qualifying action there is nothing to claim.
        assert_eq!(
            f.client
                .try_referral_claim_rewards(&f.referrer, &f.token)
                .unwrap_err(),
            Ok(ContractError::NoRewardsToClaim)
        );

        run_grant_to_payout(&f, 1_000_000);

        let claimed = f.client.referral_claim_rewards(&f.referrer, &f.token);
        assert_eq!(claimed, 1_000);
        assert_eq!(
            token::Client::new(&f.env, &f.token).balance(&f.referrer),
            1_000
        );
        assert_eq!(f.client.referral_pending_rewards(&f.referrer, &f.token), 0);
    }

    #[test]
    fn test_referral_reward_pays_only_once_per_referred_user() {
        let f = setup();
        apply_referral(&f);

        run_grant_to_payout(&f, 1_000_000);
        run_grant_to_payout(&f, 1_000_000);

        assert_eq!(
            f.client.referral_pending_rewards(&f.referrer, &f.token),
            1_000
        );
    }

    #[test]
    fn test_payout_without_referral_record_accrues_nothing() {
        let f = setup();

        run_grant_to_payout(&f, 1_000_000);

        assert_eq!(f.client.referral_pending_rewards(&f.referrer, &f.token), 0);
        assert!(f.client.referral_get_record(&f.owner).is_none());
    }

    #[test]
    fn test_claim_rewards_rejects_reentrant_call_when_guard_is_held() {
        let f = setup();

        f.env.as_contract(&f.client.address, || {
            f.env
                .storage()
                .temporary()
                .set(&crate::reentrancy::ReentrancyKey::ExternalCallGuard, &());

            let err = claim_rewards(&f.env, &f.referrer, &f.token).unwrap_err();
            assert_eq!(err, ContractError::Reentrancy);
        });
    }
}
