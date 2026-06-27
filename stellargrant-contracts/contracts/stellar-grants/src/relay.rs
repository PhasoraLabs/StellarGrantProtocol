use crate::constants;
use crate::storage::Storage;
use crate::types::{ContractError, RelayAllowance, RelayConfig, RelayableAction};
use soroban_sdk::{Address, Bytes, Env};

pub fn set_relay_config(
    env: &Env,
    admin: &Address,
    config: RelayConfig,
) -> Result<(), ContractError> {
    admin.require_auth();

    if let Some(current_admin) = crate::Storage::get_global_admin(env) {
        if current_admin != *admin {
            return Err(ContractError::Unauthorized);
        }
    }

    Storage::set_relay_config(env, &config);
    Ok(())
}

pub fn execute_relayed(
    env: &Env,
    relayer: &Address,
    sender: &Address,
    action: RelayableAction,
    nonce: u32,
    _payload: Bytes,
) -> Result<(), ContractError> {
    relayer.require_auth();

    let config = Storage::get_relay_config(env).ok_or(ContractError::InvalidState)?;

    if !config.enabled {
        return Err(ContractError::InvalidState);
    }

    if config.relayer_address != *relayer {
        return Err(ContractError::Unauthorized);
    }

    let action_allowed = config.allowed_actions.iter().any(|a| {
        matches!(
            (&a, &action),
            (
                RelayableAction::ContributorRegister,
                RelayableAction::ContributorRegister
            ) | (
                RelayableAction::MilestoneSubmit,
                RelayableAction::MilestoneSubmit
            ) | (RelayableAction::ClaimVested, RelayableAction::ClaimVested)
                | (
                    RelayableAction::WithdrawStream,
                    RelayableAction::WithdrawStream
                )
        )
    });

    if !action_allowed {
        return Err(ContractError::InvalidInput);
    }

    let expected_nonce = Storage::get_relay_nonce(env, sender) + 1;
    if nonce != expected_nonce {
        return Err(ContractError::InvalidInput);
    }

    Storage::set_relay_nonce(env, sender, nonce);

    let mut allowance =
        Storage::get_relay_allowance(env, sender).unwrap_or_else(|| RelayAllowance {
            address: sender.clone(),
            daily_relays_used: 0,
            window_start: env.ledger().timestamp(),
            total_relayed: 0,
        });

    let current_time = env.ledger().timestamp();

    if current_time > allowance.window_start + constants::SECONDS_PER_DAY {
        allowance.daily_relays_used = 0;
        allowance.window_start = current_time;
    }

    if allowance.daily_relays_used >= config.max_relays_per_address_per_day {
        return Err(ContractError::InvalidState);
    }

    allowance.daily_relays_used += 1;
    allowance.total_relayed += 1;

    Storage::set_relay_allowance(env, &allowance);

    Ok(())
}

pub fn can_relay(env: &Env, sender: &Address, action: &RelayableAction) -> bool {
    if let Some(config) = Storage::get_relay_config(env) {
        if !config.enabled {
            return false;
        }

        let allowance = if let Some(a) = Storage::get_relay_allowance(env, sender) {
            a
        } else {
            return true;
        };

        let current_time = env.ledger().timestamp();

        let daily_relays_used = if current_time > allowance.window_start + constants::SECONDS_PER_DAY {
            0
        } else {
            allowance.daily_relays_used
        };

        if daily_relays_used >= config.max_relays_per_address_per_day {
            return false;
        }

        config.allowed_actions.iter().any(|a| {
            matches!(
                (&a, action),
                (
                    RelayableAction::ContributorRegister,
                    RelayableAction::ContributorRegister
                ) | (
                    RelayableAction::MilestoneSubmit,
                    RelayableAction::MilestoneSubmit
                ) | (RelayableAction::ClaimVested, RelayableAction::ClaimVested)
                    | (
                        RelayableAction::WithdrawStream,
                        RelayableAction::WithdrawStream
                    )
            )
        })
    } else {
        false
    }
}

pub fn reimburse_relayer(_env: &Env, _relayer: &Address) -> Result<(), ContractError> {
    Ok(())
}

pub fn get_allowance(env: &Env, address: &Address) -> RelayAllowance {
    Storage::get_relay_allowance(env, address).unwrap_or_else(|| RelayAllowance {
        address: address.clone(),
        daily_relays_used: 0,
        window_start: env.ledger().timestamp(),
        total_relayed: 0,
    })
}

pub fn get_relay_config(env: &Env) -> Option<RelayConfig> {
    Storage::get_relay_config(env)
}
