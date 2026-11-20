use soroban_sdk::Env;

use crate::storage::Storage;
use crate::types::{ContributorProfile, DecayConfig, DecayType};

pub fn days_inactive(env: &Env, last_action_at: u64) -> u32 {
    let now = env.ledger().timestamp();
    if now <= last_action_at {
        return 0;
    }
    let secs_inactive = now - last_action_at;
    (secs_inactive / 86400) as u32
}

pub fn ledgers_inactive(env: &Env, last_action_at: u64) -> u32 {
    let now = env.ledger().timestamp();
    if now <= last_action_at {
        return 0;
    }
    let secs_inactive = now - last_action_at;
    (secs_inactive / 5) as u32
}

pub fn linear_decay(raw: u32, inactive_days: u32, config: &DecayConfig) -> u32 {
    let loss = inactive_days.saturating_mul(config.linear_decay_per_day);
    let decayed = (raw as u64).saturating_sub(loss as u64);
    decayed.max(config.decay_floor as u64) as u32
}

pub fn exponential_decay(raw: u32, inactive_ledgers: u32, config: &DecayConfig) -> u32 {
    if config.half_life_ledgers == 0 || inactive_ledgers == 0 {
        return raw;
    }
    let shifts = inactive_ledgers / config.half_life_ledgers;
    let decayed = if shifts >= 32 {
        0u64
    } else {
        (raw as u64) >> shifts
    };
    decayed.max(config.decay_floor as u64) as u32
}

pub fn apply_decay(env: &Env, raw_score: u32, last_action_at: u64, config: &DecayConfig) -> u32 {
    if !config.enabled {
        return raw_score;
    }

    let ledgers_idle = ledgers_inactive(env, last_action_at);
    if ledgers_idle < config.inactivity_threshold_ledgers {
        return raw_score;
    }

    match config.decay_type {
        DecayType::None => raw_score,
        DecayType::Linear => {
            let days_idle = days_inactive(env, last_action_at);
            linear_decay(raw_score, days_idle, config)
        }
        DecayType::Exponential => exponential_decay(raw_score, ledgers_idle, config),
    }
}

pub fn effective_score(env: &Env, profile: &ContributorProfile, config: &DecayConfig) -> u32 {
    apply_decay(
        env,
        profile.reputation_score as u32,
        profile.last_action_at,
        config,
    )
}

pub fn record_activity(env: &Env, contributor: &soroban_sdk::Address) {
    let mut profile = match Storage::get_contributor(env, contributor.clone()) {
        Some(p) => p,
        None => return,
    };
    profile.last_action_at = env.ledger().timestamp();
    Storage::set_contributor(env, contributor.clone(), &profile);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContributorProfile;
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::{Address, String, Vec};

    fn cfg(decay_type: DecayType) -> DecayConfig {
        DecayConfig {
            enabled: true,
            decay_type,
            half_life_ledgers: 1,
            linear_decay_per_day: 10,
            decay_floor: 0,
            inactivity_threshold_ledgers: 0,
        }
    }

    #[test]
    fn exponential_decay_shift_boundaries() {
        let mut c = cfg(DecayType::Exponential);
        c.half_life_ledgers = 1;
        let raw: u32 = 1 << 31; // 2_147_483_648

        // shift 31: still a valid `>>`
        assert_eq!(exponential_decay(raw, 31, &c), 1);
        // shift 32: special-cased to 0 to avoid an invalid shift
        assert_eq!(exponential_decay(raw, 32, &c), 0);
        // shift 33: also caught by the `shifts >= 32` guard
        assert_eq!(exponential_decay(raw, 33, &c), 0);
    }

    #[test]
    fn exponential_decay_respects_floor_and_zero_inputs() {
        let mut c = cfg(DecayType::Exponential);
        c.decay_floor = 100;
        // shifted to 0, then clamped up to the floor
        assert_eq!(exponential_decay(1_000, 32, &c), 100);

        // zero half-life or zero inactivity: passthrough
        c.half_life_ledgers = 0;
        assert_eq!(exponential_decay(1_000, 50, &c), 1_000);
        c.half_life_ledgers = 5;
        assert_eq!(exponential_decay(1_000, 0, &c), 1_000);
    }

    #[test]
    fn linear_decay_clamps_to_floor() {
        let mut c = cfg(DecayType::Linear);
        c.linear_decay_per_day = 10;
        c.decay_floor = 25;

        // 100 - 50*10 would saturate to 0; clamp brings it up to the floor
        assert_eq!(linear_decay(100, 50, &c), 25);
        // no floor interaction when the loss is small
        assert_eq!(linear_decay(100, 2, &c), 80);
    }

    #[test]
    fn apply_decay_passthrough_when_disabled() {
        let env = Env::default();
        let mut c = cfg(DecayType::Linear);
        c.enabled = false;
        assert_eq!(apply_decay(&env, 777, 0, &c), 777);
    }

    #[test]
    fn apply_decay_passthrough_below_inactivity_threshold() {
        let env = Env::default();
        env.ledger().set_timestamp(10);
        let mut c = cfg(DecayType::Linear);
        c.inactivity_threshold_ledgers = 1_000;
        assert_eq!(apply_decay(&env, 500, 0, &c), 500);
    }

    #[test]
    fn apply_decay_routes_by_decay_type() {
        let env = Env::default();
        env.ledger().set_timestamp(2 * 86_400); // 2 days idle since last_action_at = 0

        let linear = cfg(DecayType::Linear); // per_day 10 -> loss 20
        assert_eq!(apply_decay(&env, 100, 0, &linear), 80);

        let mut exp = cfg(DecayType::Exponential);
        exp.half_life_ledgers = 10; // ledgers idle = 172_800/5 = 34_560 -> shifts huge -> 0
        assert_eq!(apply_decay(&env, 400, 0, &exp), 0);

        let none = cfg(DecayType::None);
        assert_eq!(apply_decay(&env, 400, 0, &none), 400);
    }

    #[test]
    fn inactivity_helpers_return_zero_when_now_not_after_last_action() {
        let env = Env::default();
        // default ledger timestamp is 0
        assert_eq!(days_inactive(&env, 100), 0);
        assert_eq!(ledgers_inactive(&env, 100), 0);

        env.ledger().set_timestamp(100);
        assert_eq!(days_inactive(&env, 100), 0);
        assert_eq!(ledgers_inactive(&env, 100), 0);

        env.ledger().set_timestamp(100 + 3 * 86_400 + 5);
        assert_eq!(days_inactive(&env, 100), 3);
        assert_eq!(ledgers_inactive(&env, 100), (3 * 86_400 + 5) / 5);
    }

    fn profile(env: &Env, contributor: &Address, reputation_score: u64) -> ContributorProfile {
        ContributorProfile {
            contributor: contributor.clone(),
            name: String::from_str(env, "c"),
            bio: String::from_str(env, ""),
            skills: Vec::new(env),
            github_url: String::from_str(env, ""),
            registration_timestamp: 0,
            reputation_score,
            grants_count: 0,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            last_action_at: 0,
        }
    }

    #[test]
    fn effective_score_uses_profile_fields() {
        let env = Env::default();
        let contributor = Address::generate(&env);
        let mut c = cfg(DecayType::Linear);
        c.enabled = false;
        let p = profile(&env, &contributor, 500);
        assert_eq!(effective_score(&env, &p, &c), 500);
    }

    #[test]
    fn record_activity_updates_last_action_at() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let contributor = Address::generate(&env);

        env.as_contract(&contract_id, || {
            Storage::set_contributor(&env, contributor.clone(), &profile(&env, &contributor, 10));
            env.ledger().set_timestamp(5_000);
            record_activity(&env, &contributor);

            let reloaded = Storage::get_contributor(&env, contributor.clone()).unwrap();
            assert_eq!(reloaded.last_action_at, 5_000);

            // Unknown contributor: no-op, no panic.
            record_activity(&env, &Address::generate(&env));
        });
    }
}
