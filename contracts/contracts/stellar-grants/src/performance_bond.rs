use soroban_sdk::{contractevent, token, Address, Env, String};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{BondClaim, BondStatus, GrantStatus, PerformanceBond};

// ── Events ──────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BondRequired {
    pub bond_id: u32,
    pub grant_id: u64,
    pub bond_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BondPosted {
    pub bond_id: u32,
    pub grant_id: u64,
    pub guarantor: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BondReleased {
    pub bond_id: u32,
    pub grant_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BondClaimed {
    pub bond_id: u32,
    pub grant_id: u64,
    pub payout_amount: i128,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Grant owner requires a bond for their grant. Sets the bond requirement.
pub fn require_bond(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    token: &Address,
    amount: i128,
    ttl_ledgers: u32,
) -> Result<u32, ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }
    if amount <= 0 {
        return Err(ContractError::InvalidInput);
    }
    if Storage::get_performance_bond(env, grant_id).is_some() {
        return Err(ContractError::AlreadyRegistered);
    }

    let bond_id = Storage::next_bond_id(env);
    let now = env.ledger().timestamp();
    let bond = PerformanceBond {
        id: bond_id,
        grant_id,
        principal: grant.owner.clone(),
        // Guarantor is finalised when the bond is posted; defaults to the principal.
        guarantor: grant.owner.clone(),
        bond_amount: amount,
        token: token.clone(),
        status: BondStatus::Pending,
        posted_at: None,
        expires_at: now + ttl_ledgers as u64,
    };
    Storage::set_performance_bond(env, &bond);
    Storage::set_bond_grant(env, bond_id, grant_id);

    BondRequired {
        bond_id,
        grant_id,
        bond_amount: amount,
    }
    .publish(env);

    Ok(bond_id)
}

/// Guarantor posts the bond by depositing the bond amount.
pub fn post_bond(env: &Env, guarantor: &Address, bond_id: u32) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;
    guarantor.require_auth();

    let grant_id = Storage::get_bond_grant(env, bond_id).ok_or(ContractError::BondNotFound)?;
    let mut bond =
        Storage::get_performance_bond(env, grant_id).ok_or(ContractError::BondNotFound)?;

    if bond.status != BondStatus::Pending {
        return Err(ContractError::BondAlreadyPosted);
    }
    if env.ledger().timestamp() > bond.expires_at {
        return Err(ContractError::BondExpired);
    }

    crate::reentrancy::protect_external_call(env, || {
        token::Client::new(env, &bond.token).transfer(
            guarantor,
            &env.current_contract_address(),
            &bond.bond_amount,
        );
        Ok(())
    })?;

    bond.guarantor = guarantor.clone();
    bond.status = BondStatus::Active;
    bond.posted_at = Some(env.ledger().timestamp());
    Storage::set_performance_bond(env, &bond);

    BondPosted {
        bond_id,
        grant_id,
        guarantor: guarantor.clone(),
    }
    .publish(env);

    Ok(())
}

/// Release bond back to the guarantor after successful grant completion.
pub fn release_bond(env: &Env, grant_id: u64) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;

    let mut bond = match Storage::get_performance_bond(env, grant_id) {
        Some(b) => b,
        None => return Ok(()), // no bond on this grant — nothing to release
    };

    if bond.status != BondStatus::Active {
        return Err(ContractError::BondNotActive);
    }

    crate::reentrancy::protect_external_call(env, || {
        token::Client::new(env, &bond.token).transfer(
            &env.current_contract_address(),
            &bond.guarantor,
            &bond.bond_amount,
        );
        Ok(())
    })?;

    bond.status = BondStatus::Released;
    Storage::set_performance_bond(env, &bond);

    BondReleased {
        bond_id: bond.id,
        grant_id,
    }
    .publish(env);

    Ok(())
}

/// Proportional share of `total` for a contribution of `part` out of `whole`.
/// Mirrors `escrow::proportional_share` so multi-funder bond claims stay consistent
/// with escrow refund math.
fn proportional_share(part: i128, whole: i128, total: i128) -> i128 {
    if whole == 0 || total == 0 || part <= 0 {
        return 0;
    }
    part.saturating_mul(total).checked_div(whole).unwrap_or(0)
}

/// Funder claims the bond payout after a contributor default.
///
/// Distributes the full bond proportionally across **all** `grant.funders` by
/// contribution amount (same convention as `escrow::refund_all`). Any authorized
/// funder may trigger the claim; the bond is then marked `Claimed` so a second
/// call is rejected (no winner-take-all).
pub fn claim_bond(
    env: &Env,
    funder: &Address,
    grant_id: u64,
    reason: String,
) -> Result<BondClaim, ContractError> {
    crate::reentrancy::protect(env)?;
    funder.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let is_funder = grant.funders.iter().any(|f| f.funder == *funder);
    if !is_funder {
        return Err(ContractError::Unauthorized);
    }

    let mut bond =
        Storage::get_performance_bond(env, grant_id).ok_or(ContractError::BondNotFound)?;
    if bond.status != BondStatus::Active {
        return Err(ContractError::BondNotActive);
    }

    // Claimable only when the grant was cancelled/abandoned or the bond expired.
    let expired = env.ledger().timestamp() > bond.expires_at;
    let cancelled = grant.status == GrantStatus::Cancelled;
    if !expired && !cancelled {
        return Err(ContractError::InvalidState);
    }

    let funders = &grant.funders;
    if funders.is_empty() {
        return Err(ContractError::InvalidState);
    }

    let mut total_contrib: i128 = 0;
    for gf in funders.iter() {
        total_contrib = total_contrib.saturating_add(gf.amount.max(0));
    }
    if total_contrib <= 0 {
        return Err(ContractError::InvalidState);
    }

    let token_client = token::Client::new(env, &bond.token);
    let bond_amount = bond.bond_amount;
    let funders_len = funders.len();
    let mut distributed: i128 = 0;
    let mut caller_payout: i128 = 0;

    for (i, gf) in funders.iter().enumerate() {
        let is_last = (i as u32) + 1 == funders_len;
        let share = if is_last {
            // Last funder absorbs dust from integer division.
            bond_amount.saturating_sub(distributed)
        } else {
            proportional_share(gf.amount, total_contrib, bond_amount)
        };

        if share > 0 {
            crate::reentrancy::protect_external_call(env, || {
                token_client.transfer(&env.current_contract_address(), &gf.funder, &share);
                Ok(())
            })?;
            distributed = distributed.saturating_add(share);
        }
        if gf.funder == *funder {
            caller_payout = share;
        }
    }

    bond.status = BondStatus::Claimed;
    Storage::set_performance_bond(env, &bond);

    let claim = BondClaim {
        bond_id: bond.id,
        claimed_by: funder.clone(),
        claim_reason: reason,
        payout_amount: caller_payout,
        claimed_at: env.ledger().timestamp(),
    };
    Storage::set_bond_claim(env, &claim);

    BondClaimed {
        bond_id: bond.id,
        grant_id,
        payout_amount: bond_amount,
    }
    .publish(env);

    Ok(claim)
}

/// Return the bond for a grant.
pub fn get_bond(env: &Env, grant_id: u64) -> Option<PerformanceBond> {
    Storage::get_performance_bond(env, grant_id)
}

/// Check if a grant has an active (posted) bond.
pub fn has_active_bond(env: &Env, grant_id: u64) -> bool {
    matches!(
        Storage::get_performance_bond(env, grant_id),
        Some(b) if b.status == BondStatus::Active
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantFund};
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token, token::StellarAssetClient, Address, Env, String, Vec};

    const BOND_AMOUNT: i128 = 1_000;
    const TTL_LEDGERS: u32 = 100;

    struct Fixture {
        env: Env,
        contract_id: Address,
        owner: Address,
        guarantor: Address,
        funder_a: Address,
        funder_b: Address,
        token_id: Address,
        grant_id: u64,
    }

    fn setup(funder_a_amt: i128, funder_b_amt: i128) -> Fixture {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(StellarGrantsContract, ());
        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = asset.address();
        let stellar_asset = StellarAssetClient::new(&env, &token_id);

        let owner = Address::generate(&env);
        let guarantor = Address::generate(&env);
        let funder_a = Address::generate(&env);
        let funder_b = Address::generate(&env);
        let grant_id = 1u64;

        stellar_asset.mint(&guarantor, &(BOND_AMOUNT * 2));

        let mut funders = Vec::new(&env);
        funders.push_back(GrantFund {
            funder: funder_a.clone(),
            amount: funder_a_amt,
        });
        if funder_b_amt > 0 {
            funders.push_back(GrantFund {
                funder: funder_b.clone(),
                amount: funder_b_amt,
            });
        }

        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(&env, "Bonded Grant"),
            description: String::from_str(&env, "Desc"),
            token: token_id.clone(),
            status: GrantStatus::Active,
            total_amount: funder_a_amt + funder_b_amt,
            milestone_amount: funder_a_amt + funder_b_amt,
            reviewers: Vec::new(&env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders,
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };

        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, grant_id, &grant);
        });

        Fixture {
            env,
            contract_id,
            owner,
            guarantor,
            funder_a,
            funder_b,
            token_id,
            grant_id,
        }
    }

    fn require_and_post(f: &Fixture) -> u32 {
        f.env.as_contract(&f.contract_id, || {
            let bond_id = require_bond(
                &f.env,
                &f.owner,
                f.grant_id,
                &f.token_id,
                BOND_AMOUNT,
                TTL_LEDGERS,
            )
            .unwrap();
            post_bond(&f.env, &f.guarantor, bond_id).unwrap();
            bond_id
        })
    }

    fn cancel_grant(f: &Fixture) {
        f.env.as_contract(&f.contract_id, || {
            let mut grant = Storage::get_grant(&f.env, f.grant_id).unwrap();
            grant.status = GrantStatus::Cancelled;
            Storage::set_grant(&f.env, f.grant_id, &grant);
        });
    }

    #[test]
    fn test_post_bond_activates() {
        let f = setup(600, 400);
        let bond_id = require_and_post(&f);

        f.env.as_contract(&f.contract_id, || {
            let bond = get_bond(&f.env, f.grant_id).unwrap();
            assert_eq!(bond.id, bond_id);
            assert_eq!(bond.status, BondStatus::Active);
            assert_eq!(bond.guarantor, f.guarantor);
            assert!(bond.posted_at.is_some());
            assert!(has_active_bond(&f.env, f.grant_id));

            let token_client = token::Client::new(&f.env, &f.token_id);
            assert_eq!(token_client.balance(&f.contract_id), BOND_AMOUNT);
            assert_eq!(token_client.balance(&f.guarantor), BOND_AMOUNT);
        });
    }

    #[test]
    fn test_claim_bond_single_funder_happy_path() {
        let f = setup(1_000, 0);
        require_and_post(&f);
        cancel_grant(&f);

        let claim = f.env.as_contract(&f.contract_id, || {
            claim_bond(
                &f.env,
                &f.funder_a,
                f.grant_id,
                String::from_str(&f.env, "default"),
            )
            .unwrap()
        });

        assert_eq!(claim.payout_amount, BOND_AMOUNT);
        assert_eq!(claim.claimed_by, f.funder_a);

        let token_client = token::Client::new(&f.env, &f.token_id);
        assert_eq!(token_client.balance(&f.funder_a), BOND_AMOUNT);
        assert_eq!(token_client.balance(&f.contract_id), 0);

        f.env.as_contract(&f.contract_id, || {
            assert_eq!(
                get_bond(&f.env, f.grant_id).unwrap().status,
                BondStatus::Claimed
            );
            assert!(!has_active_bond(&f.env, f.grant_id));
        });
    }

    #[test]
    fn test_claim_bond_proportional_across_funders() {
        // 60/40 split of a 1000 bond → 600 / 400
        let f = setup(600, 400);
        require_and_post(&f);
        cancel_grant(&f);

        let claim = f.env.as_contract(&f.contract_id, || {
            claim_bond(
                &f.env,
                &f.funder_a,
                f.grant_id,
                String::from_str(&f.env, "contributor defaulted"),
            )
            .unwrap()
        });

        assert_eq!(claim.payout_amount, 600);
        assert_eq!(claim.claimed_by, f.funder_a);

        let token_client = token::Client::new(&f.env, &f.token_id);
        assert_eq!(token_client.balance(&f.funder_a), 600);
        assert_eq!(token_client.balance(&f.funder_b), 400);
        assert_eq!(token_client.balance(&f.contract_id), 0);
    }

    #[test]
    fn test_claim_bond_rejects_double_claim() {
        let f = setup(700, 300);
        require_and_post(&f);
        cancel_grant(&f);

        f.env.as_contract(&f.contract_id, || {
            claim_bond(
                &f.env,
                &f.funder_a,
                f.grant_id,
                String::from_str(&f.env, "first"),
            )
            .unwrap();

            let err = claim_bond(
                &f.env,
                &f.funder_b,
                f.grant_id,
                String::from_str(&f.env, "second"),
            )
            .unwrap_err();
            assert_eq!(err, ContractError::BondNotActive);
        });
    }

    #[test]
    fn test_claim_bond_rejects_unauthorized_caller() {
        let f = setup(600, 400);
        require_and_post(&f);
        cancel_grant(&f);

        let stranger = Address::generate(&f.env);
        f.env.as_contract(&f.contract_id, || {
            let err = claim_bond(
                &f.env,
                &stranger,
                f.grant_id,
                String::from_str(&f.env, "not a funder"),
            )
            .unwrap_err();
            assert_eq!(err, ContractError::Unauthorized);
        });
    }

    #[test]
    fn test_claim_bond_rejects_before_expiry_or_cancel() {
        let f = setup(1_000, 0);
        require_and_post(&f);

        f.env.as_contract(&f.contract_id, || {
            let err = claim_bond(
                &f.env,
                &f.funder_a,
                f.grant_id,
                String::from_str(&f.env, "too early"),
            )
            .unwrap_err();
            assert_eq!(err, ContractError::InvalidState);
        });
    }

    #[test]
    fn test_claim_bond_rejects_reentrant_call_when_guard_is_held() {
        let f = setup(1_000, 0);
        require_and_post(&f);
        cancel_grant(&f);

        f.env.as_contract(&f.contract_id, || {
            f.env
                .storage()
                .temporary()
                .set(&crate::reentrancy::ReentrancyKey::ExternalCallGuard, &());

            let err = claim_bond(
                &f.env,
                &f.funder_a,
                f.grant_id,
                String::from_str(&f.env, "reentrant"),
            )
            .unwrap_err();
            assert_eq!(err, ContractError::Reentrancy);
        });
    }

    #[test]
    fn test_post_bond_rejects_after_expiry() {
        let f = setup(1_000, 0);

        let bond_id = f.env.as_contract(&f.contract_id, || {
            require_bond(
                &f.env,
                &f.owner,
                f.grant_id,
                &f.token_id,
                BOND_AMOUNT,
                TTL_LEDGERS,
            )
            .unwrap()
        });

        // Advance past expires_at (require_bond set expires_at = now + ttl).
        let now = f.env.ledger().timestamp();
        f.env
            .ledger()
            .with_mut(|l| l.timestamp = now + TTL_LEDGERS as u64 + 1);

        f.env.as_contract(&f.contract_id, || {
            let err = post_bond(&f.env, &f.guarantor, bond_id).unwrap_err();
            assert_eq!(err, ContractError::BondExpired);
        });
    }

    #[test]
    fn test_claim_bond_allowed_after_expiry() {
        let f = setup(550, 450);
        require_and_post(&f);

        let now = f.env.ledger().timestamp();
        f.env
            .ledger()
            .with_mut(|l| l.timestamp = now + TTL_LEDGERS as u64 + 1);

        // Grant still Active — claim is allowed solely because the bond expired.
        let claim = f.env.as_contract(&f.contract_id, || {
            claim_bond(
                &f.env,
                &f.funder_b,
                f.grant_id,
                String::from_str(&f.env, "bond expired"),
            )
            .unwrap()
        });

        assert_eq!(claim.payout_amount, 450);
        let token_client = token::Client::new(&f.env, &f.token_id);
        assert_eq!(token_client.balance(&f.funder_a), 550);
        assert_eq!(token_client.balance(&f.funder_b), 450);
    }

    #[test]
    fn test_release_bond_returns_funds_to_guarantor() {
        let f = setup(1_000, 0);
        require_and_post(&f);

        f.env.as_contract(&f.contract_id, || {
            release_bond(&f.env, f.grant_id).unwrap();
            assert_eq!(
                get_bond(&f.env, f.grant_id).unwrap().status,
                BondStatus::Released
            );
        });

        let token_client = token::Client::new(&f.env, &f.token_id);
        assert_eq!(token_client.balance(&f.guarantor), BOND_AMOUNT * 2);
        assert_eq!(token_client.balance(&f.contract_id), 0);
    }

    #[test]
    fn test_require_bond_rejects_duplicate() {
        let f = setup(1_000, 0);
        f.env.as_contract(&f.contract_id, || {
            require_bond(
                &f.env,
                &f.owner,
                f.grant_id,
                &f.token_id,
                BOND_AMOUNT,
                TTL_LEDGERS,
            )
            .unwrap();
        });
        // Separate auth frame — calling require_auth twice in one frame panics.
        f.env.as_contract(&f.contract_id, || {
            let err = require_bond(
                &f.env,
                &f.owner,
                f.grant_id,
                &f.token_id,
                BOND_AMOUNT,
                TTL_LEDGERS,
            )
            .unwrap_err();
            assert_eq!(err, ContractError::AlreadyRegistered);
        });
    }

    #[test]
    fn test_proportional_share_helper() {
        assert_eq!(proportional_share(600, 1_000, 1_000), 600);
        assert_eq!(proportional_share(400, 1_000, 1_000), 400);
        assert_eq!(proportional_share(0, 1_000, 1_000), 0);
        assert_eq!(proportional_share(100, 0, 1_000), 0);
    }
}
