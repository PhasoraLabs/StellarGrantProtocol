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
