use soroban_sdk::{token, Address, Env, String};

use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::treasury;
use crate::types::{CollateralDeposit, CollateralRequirement, CollateralStatus};

/// Set collateral requirement for a grant. Owner only, before work starts.
pub fn set_requirement(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    req: &CollateralRequirement,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    // Only allow setting before any work has started (no milestones submitted).
    // Must be done before contributor deposits.
    if Storage::get_collateral_deposit(env, grant_id, owner).is_some() {
        return Err(ContractError::InvalidState);
    }

    Storage::set_collateral_requirement(env, grant_id, req);
    Ok(())
}

/// Contributor deposits required collateral to begin work.
pub fn deposit(env: &Env, contributor: &Address, grant_id: u64) -> Result<(), ContractError> {
    contributor.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }

    let req =
        Storage::get_collateral_requirement(env, grant_id).ok_or(ContractError::InvalidState)?;

    // Ensure not already deposited.
    if Storage::get_collateral_deposit(env, grant_id, contributor).is_some() {
        return Err(ContractError::InvalidState);
    }

    // Transfer tokens from contributor to contract.
    let token_client = token::Client::new(env, &req.token);
    token_client.transfer(contributor, &env.current_contract_address(), &req.amount);

    let now = env.ledger().timestamp();
    let deposit_record = CollateralDeposit {
        grant_id,
        contributor: contributor.clone(),
        token: req.token.clone(),
        amount: req.amount,
        status: CollateralStatus::Deposited,
        deposited_at: now,
        forfeited_amount: 0,
    };

    Storage::set_collateral_deposit(env, grant_id, contributor, &deposit_record);

    Events::emit_collateral_deposited(env, grant_id, contributor.clone(), req.amount);

    Ok(())
}

/// Release collateral back to contributor on grant completion.
pub fn release(env: &Env, grant_id: u64, contributor: &Address) -> Result<i128, ContractError> {
    let mut deposit = Storage::get_collateral_deposit(env, grant_id, contributor)
        .ok_or(ContractError::InvalidState)?;

    if deposit.status != CollateralStatus::Deposited {
        return Err(ContractError::InvalidState);
    }

    let net_amount = deposit
        .amount
        .checked_sub(deposit.forfeited_amount)
        .ok_or(ContractError::InvalidInput)?;

    if net_amount > 0 {
        let token_client = token::Client::new(env, &deposit.token);
        token_client.transfer(&env.current_contract_address(), contributor, &net_amount);
    }

    deposit.status = CollateralStatus::Released;
    Storage::set_collateral_deposit(env, grant_id, contributor, &deposit);

    Events::emit_collateral_released(env, grant_id, contributor.clone(), net_amount);

    Ok(net_amount)
}

/// Forfeit a portion of collateral (called by dispute or abandon logic).
pub fn forfeit(
    env: &Env,
    grant_id: u64,
    contributor: &Address,
    forfeit_bps: u32,
    reason: String,
) -> Result<i128, ContractError> {
    if forfeit_bps > 10_000 {
        return Err(ContractError::InvalidInput);
    }

    let mut deposit = Storage::get_collateral_deposit(env, grant_id, contributor)
        .ok_or(ContractError::InvalidState)?;

    if deposit.status != CollateralStatus::Deposited {
        return Err(ContractError::InvalidState);
    }

    let forfeited_amount = deposit
        .amount
        .checked_mul(forfeit_bps as i128)
        .ok_or(ContractError::InvalidInput)?
        .checked_div(10_000)
        .ok_or(ContractError::InvalidInput)?;

    let new_total_forfeited = deposit
        .forfeited_amount
        .checked_add(forfeited_amount)
        .ok_or(ContractError::InvalidInput)?;

    // Cap at total deposited amount.
    let actual_forfeit = if new_total_forfeited > deposit.amount {
        deposit
            .amount
            .checked_sub(deposit.forfeited_amount)
            .ok_or(ContractError::InvalidInput)?
    } else {
        forfeited_amount
    };

    if actual_forfeit > 0 {
        // Send forfeited amount to treasury, updating both token transfer and bookkeeping.
        let treasury_addr =
            Storage::get_treasury(env).ok_or(ContractError::TreasuryNotConfigured)?;
        let token_client = token::Client::new(env, &deposit.token);
        token_client.transfer(
            &env.current_contract_address(),
            &treasury_addr,
            &actual_forfeit,
        );
        treasury::deposit(
            env,
            &deposit.token,
            &env.current_contract_address(),
            actual_forfeit,
        )?;
    }

    deposit.forfeited_amount = deposit
        .forfeited_amount
        .checked_add(actual_forfeit)
        .ok_or(ContractError::InvalidInput)?;

    // If fully forfeited, mark as such.
    if deposit.forfeited_amount >= deposit.amount {
        deposit.status = CollateralStatus::Forfeited;
    } else {
        deposit.status = CollateralStatus::PartiallyForfeited;
    }

    Storage::set_collateral_deposit(env, grant_id, contributor, &deposit);

    Events::emit_collateral_forfeited(env, grant_id, contributor.clone(), actual_forfeit, reason);

    Ok(actual_forfeit)
}

/// Check that a contributor has deposited required collateral.
pub fn require_deposited(
    env: &Env,
    grant_id: u64,
    contributor: &Address,
) -> Result<(), ContractError> {
    // If no requirement is set, pass through.
    let req = match Storage::get_collateral_requirement(env, grant_id) {
        Some(r) => r,
        None => return Ok(()),
    };

    let deposit = Storage::get_collateral_deposit(env, grant_id, contributor)
        .ok_or(ContractError::CollateralNotDeposited)?;

    if deposit.status != CollateralStatus::Deposited {
        return Err(ContractError::BondNotPosted);
    }

    // Verify that the deposited token and amount match the requirement.
    if deposit.token != req.token {
        return Err(ContractError::InvalidInput);
    }
    if deposit.amount < req.amount {
        return Err(ContractError::InvalidInput);
    }

    // Deposit matches requirement.

    Ok(())
}

/// Return collateral deposit for a contributor.
pub fn get_deposit(env: &Env, grant_id: u64, contributor: &Address) -> Option<CollateralDeposit> {
    Storage::get_collateral_deposit(env, grant_id, contributor)
}

/// Return the collateral requirement for a grant.
pub fn get_requirement(env: &Env, grant_id: u64) -> Option<CollateralRequirement> {
    Storage::get_collateral_requirement(env, grant_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantFund, GrantStatus};
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::{testutils::Address as TestAddress, Env, Vec};

    fn make_grant(env: &Env, owner: &Address) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: soroban_sdk::String::from_str(env, "Test"),
            description: soroban_sdk::String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 2,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        }
    }

    #[test]
    fn test_set_and_get_requirement() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_grant(&env, 1, &make_grant(&env, &owner));

        let req = CollateralRequirement {
            grant_id: 1,
            token: token.clone(),
            amount: 500,
            forfeit_on_abandon_bps: 1000,
            forfeit_on_dispute_loss_bps: 2000,
        };

        set_requirement(&env, &owner, 1, &req).unwrap();
        let got = get_requirement(&env, 1).unwrap();
        assert_eq!(got.amount, 500);
        assert_eq!(got.forfeit_on_abandon_bps, 1000);
    }

    #[test]
    fn test_require_deposited_no_requirement_passes() {
        let env = Env::default();
        let contributor = Address::generate(&env);
        // No requirement set -> should pass
        assert_eq!(require_deposited(&env, 1, &contributor), Ok(()));
    }

    #[test]
    fn test_require_deposited_no_deposit_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_grant(&env, 1, &make_grant(&env, &owner));
        let req = CollateralRequirement {
            grant_id: 1,
            token: token.clone(),
            amount: 500,
            forfeit_on_abandon_bps: 1000,
            forfeit_on_dispute_loss_bps: 2000,
        };
        set_requirement(&env, &owner, 1, &req).unwrap();

        // No deposit yet
        assert_eq!(
            require_deposited(&env, 1, &owner),
            Err(ContractError::BondNotPosted)
        );
    }

    #[test]
    fn test_unauthorized_cannot_set_requirement() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_grant(&env, 1, &make_grant(&env, &owner));

        let req = CollateralRequirement {
            grant_id: 1,
            token: token.clone(),
            amount: 500,
            forfeit_on_abandon_bps: 1000,
            forfeit_on_dispute_loss_bps: 2000,
        };

        assert_eq!(
            set_requirement(&env, &stranger, 1, &req),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn test_forfeit_calculates_correct_amount() {
        let env = Env::default();
        env.mock_all_auths();
        let contributor = Address::generate(&env);
        let token = Address::generate(&env);

        // Manually set up a deposit.
        let deposit = CollateralDeposit {
            grant_id: 1,
            contributor: contributor.clone(),
            token: token.clone(),
            amount: 1000,
            status: CollateralStatus::Deposited,
            deposited_at: 0,
            forfeited_amount: 0,
        };
        Storage::set_collateral_deposit(&env, 1, &contributor, &deposit);

        // Setup treasury for forfeit destination
        Storage::set_treasury(&env, &Address::generate(&env));

        let reason = soroban_sdk::String::from_str(&env, "dispute lost");
        // 2000 bps = 20% of 1000 = 200
        let result = forfeit(&env, 1, &contributor, 2000, reason.clone());
        assert_eq!(result.unwrap(), 200);

        let updated = get_deposit(&env, 1, &contributor).unwrap();
        assert_eq!(updated.forfeited_amount, 200);
        assert_eq!(updated.status, CollateralStatus::PartiallyForfeited);
    }

    #[test]
    fn test_full_forfeit_marks_forfeited() {
        let env = Env::default();
        env.mock_all_auths();
        let contributor = Address::generate(&env);
        let token = Address::generate(&env);

        let deposit = CollateralDeposit {
            grant_id: 1,
            contributor: contributor.clone(),
            token: token.clone(),
            amount: 1000,
            status: CollateralStatus::Deposited,
            deposited_at: 0,
            forfeited_amount: 0,
        };
        Storage::set_collateral_deposit(&env, 1, &contributor, &deposit);

        // Setup treasury
        Storage::set_treasury(&env, &Address::generate(&env));

        let reason = soroban_sdk::String::from_str(&env, "abandoned");
        let result = forfeit(&env, 1, &contributor, 10000, reason.clone());
        assert_eq!(result.unwrap(), 1000);

        let updated = get_deposit(&env, 1, &contributor).unwrap();
        assert_eq!(updated.status, CollateralStatus::Forfeited);
        assert_eq!(updated.forfeited_amount, 1000);
    }
}
