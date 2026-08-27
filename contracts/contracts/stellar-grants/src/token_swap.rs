use soroban_sdk::{contractevent, token, Address, Env};

use crate::constants::BASIS_POINTS_SCALE;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{DexConfig, SwapResult, SwapRoute};

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapExecuted {
    pub from_token: Address,
    pub to_token: Address,
    pub amount_in: i128,
    pub amount_out: i128,
    pub slippage_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapAndFundExecuted {
    pub grant_id: u64,
    pub funder: Address,
    pub input_token: Address,
    pub input_amount: i128,
    pub swapped_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapAndPayExecuted {
    pub grant_id: u64,
    pub recipient: Address,
    pub grant_token: Address,
    pub preferred_token: Address,
    pub amount_out: i128,
}

fn require_global_admin(env: &Env, admin: &Address) -> Result<(), ContractError> {
    let global_admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if global_admin != *admin {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

pub fn set_dex_config(env: &Env, admin: &Address, config: DexConfig) -> Result<(), ContractError> {
    admin.require_auth();
    require_global_admin(env, admin)?;
    Storage::set_dex_config(env, &config);
    Ok(())
}

pub fn get_dex_config(env: &Env) -> Result<DexConfig, ContractError> {
    Storage::get_dex_config(env).ok_or(ContractError::DexNotConfigured)
}

pub fn quote(env: &Env, route: &SwapRoute, amount_in: i128) -> Result<i128, ContractError> {
    let config = get_dex_config(env)?;
    if !config.is_active {
        return Err(ContractError::DexNotConfigured);
    }
    if amount_in <= 0 {
        return Err(ContractError::InvalidInput);
    }

    // No real DEX/AMM is integrated yet. Refuse rather than fake a 1:1 rate
    // and silently confiscate the caller's input token (see #683).
    Err(ContractError::SwapNotImplemented)
}

/// Performs the swap. Callers that have not already authenticated `caller`
/// elsewhere in the same call chain must call `caller.require_auth()`
/// themselves before invoking this (see the `swap_tokens` entry point) —
/// this function does not re-check it, since a repeated `require_auth()` for
/// the same address within one invocation is rejected as redundant.
pub fn swap(
    env: &Env,
    caller: &Address,
    route: SwapRoute,
    amount_in: i128,
) -> Result<SwapResult, ContractError> {
    crate::reentrancy::protect(env)?;

    let config = get_dex_config(env)?;
    if !config.is_active {
        return Err(ContractError::DexNotConfigured);
    }
    if amount_in <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let expected_out = quote(env, &route, amount_in)?;

    let from_token_client = token::Client::new(env, &route.from_token);
    crate::reentrancy::protect_external_call(env, || {
        from_token_client.transfer(caller, env.current_contract_address(), &amount_in);
        Ok(())
    })?;

    let actual_out = expected_out;
    let slippage_bps = if expected_out > 0 {
        let diff = expected_out.saturating_sub(actual_out);
        (diff
            .saturating_mul(BASIS_POINTS_SCALE as i128)
            .checked_div(expected_out)
            .unwrap_or(0)) as u32
    } else {
        0
    };

    if slippage_bps > config.max_slippage_bps {
        return Err(ContractError::SwapExceedsSlippage);
    }

    let result = SwapResult {
        amount_in,
        amount_out: actual_out,
        slippage_actual_bps: slippage_bps,
        dex_contract: config.dex_contract.clone(),
        swapped_at: env.ledger().timestamp(),
    };

    SwapExecuted {
        from_token: route.from_token.clone(),
        to_token: route.to_token.clone(),
        amount_in,
        amount_out: actual_out,
        slippage_bps,
    }
    .publish(env);

    Ok(result)
}

pub fn swap_and_fund(
    env: &Env,
    funder: &Address,
    grant_id: u64,
    input_token: &Address,
    input_amount: i128,
) -> Result<SwapResult, ContractError> {
    crate::reentrancy::protect(env)?;
    funder.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if input_token == &grant.token {
        crate::escrow::deposit(env, grant_id, funder, input_amount)?;
        let result = SwapResult {
            amount_in: input_amount,
            amount_out: input_amount,
            slippage_actual_bps: 0,
            dex_contract: env.current_contract_address(),
            swapped_at: env.ledger().timestamp(),
        };
        SwapAndFundExecuted {
            grant_id,
            funder: funder.clone(),
            input_token: input_token.clone(),
            input_amount,
            swapped_amount: input_amount,
        }
        .publish(env);
        return Ok(result);
    }

    let route = SwapRoute {
        from_token: input_token.clone(),
        to_token: grant.token.clone(),
        intermediary: None,
        min_out: 0,
    };

    let swap_result = swap(env, funder, route, input_amount)?;
    crate::escrow::deposit(env, grant_id, funder, swap_result.amount_out)?;

    SwapAndFundExecuted {
        grant_id,
        funder: funder.clone(),
        input_token: input_token.clone(),
        input_amount,
        swapped_amount: swap_result.amount_out,
    }
    .publish(env);

    Ok(swap_result)
}

pub fn swap_and_pay(
    env: &Env,
    payer: &Address,
    grant_id: u64,
    recipient: &Address,
    grant_token: &Address,
    preferred_token: &Address,
    amount: i128,
) -> Result<SwapResult, ContractError> {
    crate::reentrancy::protect(env)?;
    payer.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *payer {
        return Err(ContractError::Unauthorized);
    }
    if grant.token != *grant_token {
        return Err(ContractError::InvalidInput);
    }

    if grant_token == preferred_token {
        crate::escrow::release(env, grant_id, recipient, amount)?;
        let result = SwapResult {
            amount_in: amount,
            amount_out: amount,
            slippage_actual_bps: 0,
            dex_contract: env.current_contract_address(),
            swapped_at: env.ledger().timestamp(),
        };
        SwapAndPayExecuted {
            grant_id,
            recipient: recipient.clone(),
            grant_token: grant_token.clone(),
            preferred_token: preferred_token.clone(),
            amount_out: amount,
        }
        .publish(env);
        return Ok(result);
    }

    let route = SwapRoute {
        from_token: grant_token.clone(),
        to_token: preferred_token.clone(),
        intermediary: None,
        min_out: 0,
    };

    let swap_result = swap(env, &env.current_contract_address(), route, amount)?;
    let token_client = token::Client::new(env, preferred_token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(
            &env.current_contract_address(),
            recipient,
            &swap_result.amount_out,
        );
        Ok(())
    })?;

    SwapAndPayExecuted {
        grant_id,
        recipient: recipient.clone(),
        grant_token: grant_token.clone(),
        preferred_token: preferred_token.clone(),
        amount_out: swap_result.amount_out,
    }
    .publish(env);

    Ok(swap_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StellarGrantsContract;
    use crate::StellarGrantsContractClient;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{token::StellarAssetClient, Address, Env};

    fn setup_env() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        (env, contract_id)
    }

    #[test]
    fn test_set_dex_config() {
        let (env, contract_id) = setup_env();
        let contract_id_clone = contract_id.clone();
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let dex_contract = Address::generate(&env);

        client.set_global_admin(&admin, &admin);

        let new_config = DexConfig {
            dex_contract: dex_contract.clone(),
            max_slippage_bps: 50,
            is_active: false,
        };
        env.as_contract(&contract_id_clone, || {
            set_dex_config(&env, &admin, new_config.clone()).unwrap();
        });
        let stored = env
            .as_contract(&contract_id_clone, || get_dex_config(&env))
            .unwrap();
        assert_eq!(stored.max_slippage_bps, 50);
        assert!(!stored.is_active);
    }

    #[test]
    fn test_get_dex_config_not_configured() {
        let (env, contract_id) = setup_env();
        let admin = Address::generate(&env);
        let client = StellarGrantsContractClient::new(&env, &contract_id);
        client.set_global_admin(&admin, &admin);

        let result = env.as_contract(&contract_id, || get_dex_config(&env));
        assert_eq!(result, Err(ContractError::DexNotConfigured));
    }

    #[test]
    fn test_swap_zero_amount() {
        let (env, contract_id) = setup_env();
        let admin = Address::generate(&env);
        let dex_contract = Address::generate(&env);
        let token = Address::generate(&env);
        let caller = Address::generate(&env);
        let client = StellarGrantsContractClient::new(&env, &contract_id);
        client.set_global_admin(&admin, &admin);

        env.as_contract(&contract_id, || {
            let config = DexConfig {
                dex_contract,
                max_slippage_bps: 100,
                is_active: true,
            };
            Storage::set_dex_config(&env, &config);
        });

        let route = SwapRoute {
            from_token: token.clone(),
            to_token: token.clone(),
            intermediary: None,
            min_out: 0,
        };
        let result = env.as_contract(&contract_id, || swap(&env, &caller, route, 0));
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_swap_and_fund_same_token() {
        let (env, contract_id) = setup_env();
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let funder = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin.clone());
        let stellar_asset = StellarAssetClient::new(&env, &asset.address());
        stellar_asset.mint(&funder, &1_000_000);

        client.set_global_admin(&admin, &admin);

        env.as_contract(&contract_id, || {
            let token = asset.address();
            crate::escrow::open(&env, 1, &admin, &token).unwrap();
            let grant = crate::types::Grant {
                id: 1,
                owner: admin.clone(),
                title: soroban_sdk::String::from_str(&env, "test"),
                description: soroban_sdk::String::from_str(&env, "desc"),
                token: token.clone(),
                status: crate::types::GrantStatus::Active,
                total_amount: 100_000,
                milestone_amount: 10_000,
                reviewers: soroban_sdk::Vec::new(&env),
                total_milestones: 1,
                milestones_paid_out: 0,
                escrow_balance: 0,
                funders: soroban_sdk::Vec::new(&env),
                reason: None,
                timestamp: env.ledger().timestamp(),
                require_compliance: None,
            };
            Storage::set_grant(&env, 1, &grant);
        });

        let token = asset.address();
        let result = env.as_contract(&contract_id, || {
            swap_and_fund(&env, &funder, 1, &token, 1000).unwrap()
        });
        assert_eq!(result.amount_in, 1000);
        assert_eq!(result.amount_out, 1000);

        env.as_contract(&contract_id, || {
            let grant = Storage::get_grant(&env, 1).unwrap();
            assert!(grant.escrow_balance > 0);
        });
    }

    fn open_funded_grant(
        env: &Env,
        contract_id: &Address,
        owner: &Address,
        token: &Address,
        funder: &Address,
        amount: i128,
    ) {
        env.as_contract(contract_id, || {
            crate::escrow::open(env, 1, owner, token).unwrap();
            let grant = crate::types::Grant {
                id: 1,
                owner: owner.clone(),
                title: soroban_sdk::String::from_str(env, "test"),
                description: soroban_sdk::String::from_str(env, "desc"),
                token: token.clone(),
                status: crate::types::GrantStatus::Active,
                total_amount: 100_000,
                milestone_amount: 10_000,
                reviewers: soroban_sdk::Vec::new(env),
                total_milestones: 1,
                milestones_paid_out: 0,
                escrow_balance: 0,
                funders: soroban_sdk::Vec::new(env),
                reason: None,
                timestamp: env.ledger().timestamp(),
                require_compliance: None,
            };
            Storage::set_grant(env, 1, &grant);
        });
        env.as_contract(contract_id, || {
            // escrow::deposit trusts its caller to have already authorized
            // the funder (as swap_and_fund/grant_fund do in production).
            funder.require_auth();
            crate::escrow::deposit(env, 1, funder, amount).unwrap();
        });
    }

    // ── Issue #682: swap_and_pay must not release escrow to an unauthorized caller ──

    #[test]
    fn test_swap_and_pay_unauthorized_rejected() {
        let (env, contract_id) = setup_env();
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        let recipient = Address::generate(&env);
        let funder = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin.clone());
        StellarAssetClient::new(&env, &asset.address()).mint(&funder, &1_000_000);

        client.set_global_admin(&owner, &owner);

        let token = asset.address();
        open_funded_grant(&env, &contract_id, &owner, &token, &funder, 50_000);

        // A non-owner cannot drain the escrow via swap_and_pay.
        let result = env.as_contract(&contract_id, || {
            swap_and_pay(&env, &stranger, 1, &recipient, &token, &token, 10_000)
        });
        assert_eq!(result, Err(ContractError::Unauthorized));

        env.as_contract(&contract_id, || {
            let grant = Storage::get_grant(&env, 1).unwrap();
            assert_eq!(grant.escrow_balance, 50_000);
        });

        // The real grant owner can.
        let result = env.as_contract(&contract_id, || {
            swap_and_pay(&env, &owner, 1, &recipient, &token, &token, 10_000)
        });
        assert!(result.is_ok());

        let token_client = token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&recipient), 10_000);
    }

    #[test]
    fn test_swap_and_pay_grant_token_mismatch_rejected() {
        let (env, contract_id) = setup_env();
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let recipient = Address::generate(&env);
        let funder = Address::generate(&env);
        let other_token = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin.clone());
        StellarAssetClient::new(&env, &asset.address()).mint(&funder, &1_000_000);

        client.set_global_admin(&owner, &owner);

        let token = asset.address();
        open_funded_grant(&env, &contract_id, &owner, &token, &funder, 50_000);

        let result = env.as_contract(&contract_id, || {
            swap_and_pay(
                &env,
                &owner,
                1,
                &recipient,
                &other_token,
                &other_token,
                10_000,
            )
        });
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    // ── Issue #683: quote/swap must refuse rather than confiscate funds ──────

    #[test]
    fn test_quote_returns_not_implemented() {
        let (env, contract_id) = setup_env();
        let admin = Address::generate(&env);
        let dex_contract = Address::generate(&env);
        let client = StellarGrantsContractClient::new(&env, &contract_id);
        client.set_global_admin(&admin, &admin);

        env.as_contract(&contract_id, || {
            let config = DexConfig {
                dex_contract,
                max_slippage_bps: 100,
                is_active: true,
            };
            Storage::set_dex_config(&env, &config);
        });

        let route = SwapRoute {
            from_token: Address::generate(&env),
            to_token: Address::generate(&env),
            intermediary: None,
            min_out: 0,
        };
        let result = env.as_contract(&contract_id, || quote(&env, &route, 1000));
        assert_eq!(result, Err(ContractError::SwapNotImplemented));
    }

    #[test]
    fn test_swap_returns_not_implemented_and_touches_no_funds() {
        let (env, contract_id) = setup_env();
        let admin = Address::generate(&env);
        let dex_contract = Address::generate(&env);
        let caller = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let from_asset = env.register_stellar_asset_contract_v2(token_admin.clone());
        StellarAssetClient::new(&env, &from_asset.address()).mint(&caller, &1_000_000);

        let client = StellarGrantsContractClient::new(&env, &contract_id);
        client.set_global_admin(&admin, &admin);

        env.as_contract(&contract_id, || {
            let config = DexConfig {
                dex_contract,
                max_slippage_bps: 100,
                is_active: true,
            };
            Storage::set_dex_config(&env, &config);
        });

        let route = SwapRoute {
            from_token: from_asset.address(),
            to_token: Address::generate(&env),
            intermediary: None,
            min_out: 0,
        };
        let result = env.as_contract(&contract_id, || swap(&env, &caller, route, 1000));
        assert_eq!(result, Err(ContractError::SwapNotImplemented));

        let token_client = token::Client::new(&env, &from_asset.address());
        assert_eq!(token_client.balance(&caller), 1_000_000);
    }

    // ── Issue #684: swap_and_fund cannot double-charge once swap() is disabled ──

    #[test]
    fn test_swap_and_fund_cross_token_rejected_cleanly() {
        let (env, contract_id) = setup_env();
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let funder = Address::generate(&env);

        let grant_token_admin = Address::generate(&env);
        let grant_asset = env.register_stellar_asset_contract_v2(grant_token_admin.clone());

        let input_token_admin = Address::generate(&env);
        let input_asset = env.register_stellar_asset_contract_v2(input_token_admin.clone());
        StellarAssetClient::new(&env, &input_asset.address()).mint(&funder, &1_000_000);

        client.set_global_admin(&admin, &admin);

        let grant_token = grant_asset.address();
        env.as_contract(&contract_id, || {
            let config = DexConfig {
                dex_contract: Address::generate(&env),
                max_slippage_bps: 100,
                is_active: true,
            };
            Storage::set_dex_config(&env, &config);
            crate::escrow::open(&env, 1, &admin, &grant_token).unwrap();
            let grant = crate::types::Grant {
                id: 1,
                owner: admin.clone(),
                title: soroban_sdk::String::from_str(&env, "test"),
                description: soroban_sdk::String::from_str(&env, "desc"),
                token: grant_token.clone(),
                status: crate::types::GrantStatus::Active,
                total_amount: 100_000,
                milestone_amount: 10_000,
                reviewers: soroban_sdk::Vec::new(&env),
                total_milestones: 1,
                milestones_paid_out: 0,
                escrow_balance: 0,
                funders: soroban_sdk::Vec::new(&env),
                reason: None,
                timestamp: env.ledger().timestamp(),
                require_compliance: None,
            };
            Storage::set_grant(&env, 1, &grant);
        });

        let input_token = input_asset.address();
        let result = env.as_contract(&contract_id, || {
            swap_and_fund(&env, &funder, 1, &input_token, 1000)
        });
        assert_eq!(result, Err(ContractError::SwapNotImplemented));

        // The funder was charged exactly zero times, not twice.
        let input_token_client = token::Client::new(&env, &input_token);
        assert_eq!(input_token_client.balance(&funder), 1_000_000);

        env.as_contract(&contract_id, || {
            let grant = Storage::get_grant(&env, 1).unwrap();
            assert_eq!(grant.escrow_balance, 0);
        });
    }
}
