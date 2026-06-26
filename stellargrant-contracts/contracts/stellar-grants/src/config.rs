use soroban_sdk::{Address, Env};

use crate::storage::Storage;
use crate::types::{ContractError, DecayConfig, DecayType, ProtocolConfig};

pub fn default_config() -> ProtocolConfig {
    ProtocolConfig {
        quorum_threshold_bps: 5001,
        max_reviewers: 10,
        min_stake_amount: 1_000_000,
        protocol_fee_bps: 100,
        max_milestones_per_grant: 20,
        dispute_window_ledgers: 17280,
        max_grant_title_len: 128,
        max_grant_desc_len: 1024,
        multisig_threshold: 0,
        rate_limit_multiplier: 1,
        referral_fee_bps: 1000,
        decay_config: DecayConfig {
            enabled: false,
            decay_type: DecayType::None,
            half_life_ledgers: 0,
            linear_decay_per_day: 0,
            decay_floor: 50,
            inactivity_threshold_ledgers: 0,
        },
    }
}

pub fn get_config(env: &Env) -> ProtocolConfig {
    Storage::get_protocol_config(env).unwrap_or_else(default_config)
}

pub fn validate_config(config: &ProtocolConfig) -> Result<(), ContractError> {
    if config.quorum_threshold_bps < 5001 || config.quorum_threshold_bps > 10_000 {
        return Err(ContractError::InvalidInput);
    }
    if config.protocol_fee_bps > 1000 {
        return Err(ContractError::InvalidInput);
    }
    if config.referral_fee_bps > 10_000 {
        return Err(ContractError::InvalidInput);
    }
    if config.max_reviewers < 1 {
        return Err(ContractError::InvalidInput);
    }
    if config.max_milestones_per_grant < 1 {
        return Err(ContractError::InvalidInput);
    }
    if config.max_grant_title_len < 1 || config.max_grant_desc_len < 1 {
        return Err(ContractError::InvalidInput);
    }
    Ok(())
}

pub fn set_config(
    env: &Env,
    admin: &Address,
    new_config: ProtocolConfig,
) -> Result<(), ContractError> {
    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    validate_config(&new_config)?;
    Storage::set_protocol_config(env, &new_config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    #[test]
    fn test_default_config_values() {
        let cfg = default_config();
        assert_eq!(cfg.quorum_threshold_bps, 5001);
        assert_eq!(cfg.max_milestones_per_grant, 20);
        assert_eq!(cfg.protocol_fee_bps, 100);
    }

    #[test]
    fn test_invalid_quorum_threshold_rejected() {
        let mut cfg = default_config();
        cfg.quorum_threshold_bps = 4999;
        assert_eq!(validate_config(&cfg), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_invalid_fee_bps_rejected() {
        let mut cfg = default_config();
        cfg.protocol_fee_bps = 1001;
        assert_eq!(validate_config(&cfg), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_valid_config_passes_validation() {
        let cfg = default_config();
        assert!(validate_config(&cfg).is_ok());
    }

    #[test]
    fn test_zero_max_reviewers_rejected() {
        let mut cfg = default_config();
        cfg.max_reviewers = 0;
        assert_eq!(validate_config(&cfg), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_quorum_too_high_rejected() {
        let mut cfg = default_config();
        cfg.quorum_threshold_bps = 10_001;
        assert_eq!(validate_config(&cfg), Err(ContractError::InvalidInput));
    }
}
