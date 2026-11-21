use soroban_sdk::{token, Address, Env, String};

use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
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
    crate::reentrancy::protect(env)?;

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
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(contributor, &env.current_contract_address(), &req.amount);
        Ok(())
    })?;

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
pub fn release(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    contributor: &Address,
) -> Result<i128, ContractError> {
    caller.require_auth();
    let admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if *caller != admin && *caller != grant.owner {
        return Err(ContractError::Unauthorized);
    }

    use crate::types::GrantStatus;
    if grant.status != GrantStatus::Completed {
        return Err(ContractError::InvalidState);
    }

    release_authorized(env, grant_id, contributor)
}

/// Release without checking a caller or the grant's status. For internal
/// paths that have already authorized the operation, like grant cancellation
/// returning whatever the abandonment forfeiture left behind.
pub(crate) fn release_authorized(
    env: &Env,
    grant_id: u64,
    contributor: &Address,
) -> Result<i128, ContractError> {
    crate::reentrancy::protect(env)?;
    let mut deposit = Storage::get_collateral_deposit(env, grant_id, contributor)
        .ok_or(ContractError::InvalidState)?;

    // A partial forfeiture leaves the remainder owed to the contributor.
    if deposit.status != CollateralStatus::Deposited
        && deposit.status != CollateralStatus::PartiallyForfeited
    {
        return Err(ContractError::InvalidState);
    }

    let net_amount = deposit
        .amount
        .checked_sub(deposit.forfeited_amount)
        .ok_or(ContractError::InvalidInput)?;

    if net_amount > 0 {
        let token_client = token::Client::new(env, &deposit.token);
        crate::reentrancy::protect_external_call(env, || {
            token_client.transfer(&env.current_contract_address(), contributor, &net_amount);
            Ok(())
        })?;
    }

    deposit.status = CollateralStatus::Released;
    Storage::set_collateral_deposit(env, grant_id, contributor, &deposit);

    Events::emit_collateral_released(env, grant_id, contributor.clone(), net_amount);

    Ok(net_amount)
}

/// Forfeit a portion of collateral (called by dispute or abandon logic).
pub fn forfeit(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    contributor: &Address,
    forfeit_bps: u32,
    reason: String,
) -> Result<i128, ContractError> {
    caller.require_auth();
    let admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if *caller != admin && *caller != grant.owner {
        return Err(ContractError::Unauthorized);
    }

    forfeit_authorized(env, grant_id, contributor, forfeit_bps, reason)
}

/// Forfeit without checking a caller. For internal paths that have already
/// authorized the operation (or are permissionless, like timer-driven
/// cancellation); calling `forfeit` there would re-`require_auth` an address
/// the invocation either already authorized or never had signed.
pub(crate) fn forfeit_authorized(
    env: &Env,
    grant_id: u64,
    contributor: &Address,
    forfeit_bps: u32,
    reason: String,
) -> Result<i128, ContractError> {
    crate::reentrancy::protect(env)?;
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

    // Nothing to take (e.g. a 0-bps forfeiture): leave the deposit untouched
    // so it stays releasable.
    if actual_forfeit == 0 {
        return Ok(0);
    }

    // Resolve the destination before persisting anything, so a missing
    // treasury can't record a forfeiture whose tokens never left the contract.
    let treasury_addr = Storage::get_treasury(env).ok_or(ContractError::TreasuryNotConfigured)?;

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

    // Persist state before external transfer (checks-effects-interactions pattern).
    Storage::set_collateral_deposit(env, grant_id, contributor, &deposit);

    // Send forfeited amount to treasury, protected against reentrancy.
    let token_client = token::Client::new(env, &deposit.token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(
            &env.current_contract_address(),
            &treasury_addr,
            &actual_forfeit,
        );
        Ok(())
    })?;

    Events::emit_collateral_forfeited(env, grant_id, contributor.clone(), actual_forfeit, reason);

    Ok(actual_forfeit)
}

/// Check that a contributor has deposited required collateral.
pub fn require_deposited(
    env: &Env,
    grant_id: u64,
    contributor: &Address,
) -> Result<(), ContractError> {
    // If no requirement is set, pass through — unless the grant's archetype
    // (issue #912) mandates staking, in which case a requirement must have
    // been configured before work can proceed.
    let req = match Storage::get_collateral_requirement(env, grant_id) {
        Some(r) => r,
        None => {
            if Storage::get_grant_safety_flags(env, grant_id).requires_staking {
                return Err(ContractError::CollateralNotDeposited);
            }
            return Ok(());
        }
    };

    let deposit = Storage::get_collateral_deposit(env, grant_id, contributor)
        .ok_or(ContractError::CollateralNotDeposited)?;

    if deposit.status != CollateralStatus::Deposited {
        return Err(ContractError::CollateralNotDeposited);
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

    macro_rules! collateral_test {
        ($name:ident, |$env:ident, $contract_id:ident| $body:block) => {
            #[test]
            fn $name() {
                let $env = Env::default();
                $env.mock_all_auths();
                let $contract_id = $env.register(crate::StellarGrantsContract, ());
                $env.as_contract(&$contract_id, || $body);
            }
        };
    }

    collateral_test!(test_set_and_get_requirement, |env, contract_id| {
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
    });

    collateral_test!(
        test_require_deposited_no_requirement_passes,
        |env, contract_id| {
            let contributor = Address::generate(&env);
            // No requirement set -> should pass
            assert_eq!(require_deposited(&env, 1, &contributor), Ok(()));
        }
    );

    collateral_test!(
        test_require_deposited_blocks_archetype_requires_staking,
        |env, contract_id| {
            use crate::types::GrantSafetyFlags;

            let contributor = Address::generate(&env);

            // No requirement configured, and the grant's archetype (issue #912)
            // mandates staking -> must not silently pass through.
            Storage::set_grant_safety_flags(
                &env,
                1,
                &GrantSafetyFlags {
                    requires_staking: true,
                    multisig_required: false,
                    insurance_opt_in: false,
                },
            );
            assert_eq!(
                require_deposited(&env, 1, &contributor),
                Err(ContractError::CollateralNotDeposited)
            );
        }
    );

    collateral_test!(
        test_require_deposited_no_deposit_fails,
        |env, contract_id| {
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
                Err(ContractError::CollateralNotDeposited)
            );
        }
    );

    collateral_test!(
        test_unauthorized_cannot_set_requirement,
        |env, contract_id| {
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
    );

    #[test]
    fn test_forfeit_calculates_correct_amount() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let contributor = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(owner.clone())
            .address();

        // Manually set up a grant
        let mut grant = make_grant(&env, &owner);
        grant.token = token.clone();

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
        // Setup treasury for forfeit destination
        let treasury = Address::generate(&env);

        let reason = soroban_sdk::String::from_str(&env, "dispute lost");
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &grant);
            Storage::set_global_admin(&env, &owner);
            Storage::set_collateral_deposit(&env, 1, &contributor, &deposit);
            Storage::set_treasury(&env, &treasury);
        });
        soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&contract_id, &1000);

        // 2000 bps = 20% of 1000 = 200
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let result = client.collateral_forfeit(&owner, &1, &contributor, &2000, &reason);
        assert_eq!(result, 200);

        let updated = client.collateral_get_deposit(&1, &contributor).unwrap();
        assert_eq!(updated.forfeited_amount, 200);
        assert_eq!(updated.status, CollateralStatus::PartiallyForfeited);
    }

    #[test]
    fn test_full_forfeit_marks_forfeited() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let contributor = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(owner.clone())
            .address();

        // Manually set up a grant
        let mut grant = make_grant(&env, &owner);
        grant.token = token.clone();

        let deposit = CollateralDeposit {
            grant_id: 1,
            contributor: contributor.clone(),
            token: token.clone(),
            amount: 1000,
            status: CollateralStatus::Deposited,
            deposited_at: 0,
            forfeited_amount: 0,
        };

        // Setup treasury
        let treasury = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &grant);
            Storage::set_global_admin(&env, &owner);
            Storage::set_collateral_deposit(&env, 1, &contributor, &deposit);
            Storage::set_treasury(&env, &treasury);
        });
        soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&contract_id, &1000);

        let reason = soroban_sdk::String::from_str(&env, "abandoned");
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let result = client.collateral_forfeit(&owner, &1, &contributor, &10000, &reason);
        assert_eq!(result, 1000);

        let updated = client.collateral_get_deposit(&1, &contributor).unwrap();
        assert_eq!(updated.status, CollateralStatus::Forfeited);
        assert_eq!(updated.forfeited_amount, 1000);
    }

    /// Registers the contract with grant 1 owned by `owner`, a 1000-token
    /// collateral deposit from `owner` held by the contract, and a treasury.
    /// Returns the contract id, token and treasury.
    fn setup_owner_deposit(env: &Env, owner: &Address) -> (Address, Address, Address) {
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let token = env
            .register_stellar_asset_contract_v2(Address::generate(env))
            .address();
        let treasury = Address::generate(env);

        let mut grant = make_grant(env, owner);
        grant.token = token.clone();
        let req = CollateralRequirement {
            grant_id: 1,
            token: token.clone(),
            amount: 1000,
            forfeit_on_abandon_bps: 0,
            forfeit_on_dispute_loss_bps: 0,
        };
        let deposit = CollateralDeposit {
            grant_id: 1,
            contributor: owner.clone(),
            token: token.clone(),
            amount: 1000,
            status: CollateralStatus::Deposited,
            deposited_at: 0,
            forfeited_amount: 0,
        };
        env.as_contract(&contract_id, || {
            Storage::set_grant(env, 1, &grant);
            Storage::set_global_admin(env, owner);
            Storage::set_collateral_requirement(env, 1, &req);
            Storage::set_collateral_deposit(env, 1, owner, &deposit);
            Storage::set_treasury(env, &treasury);
        });
        soroban_sdk::token::StellarAssetClient::new(env, &token).mint(&contract_id, &1000);
        (contract_id, token, treasury)
    }

    fn complete_grant(env: &Env, contract_id: &Address) {
        env.as_contract(contract_id, || {
            let mut grant = Storage::get_grant(env, 1).unwrap();
            grant.status = GrantStatus::Completed;
            Storage::set_grant(env, 1, &grant);
        });
    }

    #[test]
    fn test_zero_bps_forfeit_leaves_deposit_releasable() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let (contract_id, token, _treasury) = setup_owner_deposit(&env, &owner);
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);

        let reason = soroban_sdk::String::from_str(&env, "abandoned");
        assert_eq!(
            client.collateral_forfeit(&owner, &1, &owner, &0, &reason),
            0
        );

        let deposit = client.collateral_get_deposit(&1, &owner).unwrap();
        assert_eq!(deposit.status, CollateralStatus::Deposited);
        assert_eq!(deposit.forfeited_amount, 0);

        complete_grant(&env, &contract_id);
        assert_eq!(client.collateral_release(&owner, &1, &owner), 1000);
        let token_client = soroban_sdk::token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&owner), 1000);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    fn test_partially_forfeited_remainder_releasable() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let (contract_id, token, treasury) = setup_owner_deposit(&env, &owner);
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);

        let reason = soroban_sdk::String::from_str(&env, "dispute lost");
        assert_eq!(
            client.collateral_forfeit(&owner, &1, &owner, &2000, &reason),
            200
        );
        let deposit = client.collateral_get_deposit(&1, &owner).unwrap();
        assert_eq!(deposit.status, CollateralStatus::PartiallyForfeited);

        complete_grant(&env, &contract_id);
        assert_eq!(client.collateral_release(&owner, &1, &owner), 800);

        let token_client = soroban_sdk::token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&owner), 800);
        assert_eq!(token_client.balance(&treasury), 200);
        assert_eq!(token_client.balance(&contract_id), 0);
        let deposit = client.collateral_get_deposit(&1, &owner).unwrap();
        assert_eq!(deposit.status, CollateralStatus::Released);
    }

    #[test]
    fn test_forfeit_without_treasury_leaves_deposit_untouched() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let (contract_id, _token, _treasury) = setup_owner_deposit(&env, &owner);

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .remove(&crate::storage::DataKey::Treasury);
            let reason = soroban_sdk::String::from_str(&env, "abandoned");
            assert_eq!(
                forfeit_authorized(&env, 1, &owner, 2000, reason),
                Err(ContractError::TreasuryNotConfigured)
            );
            let deposit = get_deposit(&env, 1, &owner).unwrap();
            assert_eq!(deposit.status, CollateralStatus::Deposited);
            assert_eq!(deposit.forfeited_amount, 0);
        });
    }

    #[test]
    fn test_cancel_with_zero_abandon_bps_returns_full_deposit() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let (contract_id, token, _treasury) = setup_owner_deposit(&env, &owner);
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);

        client.cancel_grant(&1, &owner, &soroban_sdk::String::from_str(&env, "done"));

        let token_client = soroban_sdk::token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&owner), 1000);
        assert_eq!(token_client.balance(&contract_id), 0);
        let deposit = client.collateral_get_deposit(&1, &owner).unwrap();
        assert_eq!(deposit.status, CollateralStatus::Released);
    }

    #[test]
    fn test_cancel_returns_remainder_after_abandon_forfeit() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let (contract_id, token, treasury) = setup_owner_deposit(&env, &owner);
        env.as_contract(&contract_id, || {
            let mut req = Storage::get_collateral_requirement(&env, 1).unwrap();
            req.forfeit_on_abandon_bps = 2500;
            Storage::set_collateral_requirement(&env, 1, &req);
        });
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);

        client.cancel_grant(&1, &owner, &soroban_sdk::String::from_str(&env, "done"));

        let token_client = soroban_sdk::token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&treasury), 250);
        assert_eq!(token_client.balance(&owner), 750);
        assert_eq!(token_client.balance(&contract_id), 0);
    }
}
