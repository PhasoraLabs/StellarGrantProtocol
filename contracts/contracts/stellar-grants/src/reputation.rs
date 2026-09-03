use soroban_sdk::Env;

use crate::config;
use crate::events::Events;
use crate::reputation_decay;
use crate::storage::Storage;
use crate::types::{ContractError, ContributorProfile, ReputationTier};

const DELIVERY_RATE_WEIGHT_BPS: u64 = 5000;
const EARNINGS_WEIGHT_BPS: u64 = 3000;
const REJECTION_PENALTY_BPS: u64 = 200;
const MAX_SCORE: u64 = 1000;
const EARNINGS_NORMALIZATION: i128 = 10_000_000_000;

pub fn calculate_score(profile: &ContributorProfile) -> u32 {
    let total_attempted = profile
        .milestones_completed
        .saturating_add(profile.milestones_rejected) as u64;

    let delivery_rate_score = if total_attempted == 0 {
        0u64
    } else {
        (profile.milestones_completed as u64)
            .saturating_mul(DELIVERY_RATE_WEIGHT_BPS)
            .checked_div(total_attempted)
            .unwrap_or(0)
    };

    let earnings_normalized = if profile.total_earned <= 0 {
        0u64
    } else {
        let capped = profile.total_earned.min(EARNINGS_NORMALIZATION);
        ((capped as u64).saturating_mul(EARNINGS_WEIGHT_BPS))
            .checked_div(EARNINGS_NORMALIZATION as u64)
            .unwrap_or(0)
    };

    let rejection_penalty =
        (profile.milestones_rejected as u64).saturating_mul(REJECTION_PENALTY_BPS);

    let raw = delivery_rate_score
        .saturating_add(earnings_normalized)
        .saturating_sub(rejection_penalty);

    let scaled = raw
        .saturating_mul(MAX_SCORE)
        .checked_div(10_000)
        .unwrap_or(0);

    scaled.min(MAX_SCORE) as u32
}

pub fn calculate_effective_score(env: &Env, profile: &ContributorProfile) -> u32 {
    let raw = calculate_score(profile);
    let cfg = config::get_config(env);
    reputation_decay::apply_decay(env, raw, profile.last_action_at, &cfg.decay_config)
}

pub fn record_completion(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    profile: &mut ContributorProfile,
    amount_earned: i128,
) -> Result<u32, ContractError> {
    profile.milestones_completed = profile.milestones_completed.saturating_add(1);
    profile.total_earned = profile.total_earned.saturating_add(amount_earned);

    let new_score = calculate_effective_score(env, profile);
    profile.reputation_score = new_score as u64;

    Storage::set_contributor(env, profile.contributor.clone(), profile);
    reputation_decay::record_activity(env, &profile.contributor);
    Events::emit_reputation_updated(
        env,
        grant_id,
        milestone_idx,
        profile.contributor.clone(),
        profile.reputation_score,
        profile.total_earned,
    );
    Ok(new_score)
}

pub fn tier_from_score(score: u32) -> ReputationTier {
    match score {
        0..=99 => ReputationTier::Unranked,
        100..=399 => ReputationTier::Bronze,
        400..=699 => ReputationTier::Silver,
        700..=899 => ReputationTier::Gold,
        _ => ReputationTier::Platinum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContributorProfile;
    use soroban_sdk::{testutils::Address as _, Env, String, Vec};

    fn blank_profile(env: &Env) -> ContributorProfile {
        ContributorProfile {
            contributor: soroban_sdk::Address::generate(env),
            name: String::from_str(env, "Alice"),
            bio: String::from_str(env, ""),
            skills: Vec::new(env),
            github_url: String::from_str(env, ""),
            registration_timestamp: 0,
            reputation_score: 0,
            grants_count: 0,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            last_action_at: 0,
        }
    }

    #[test]
    fn test_score_from_scratch_is_zero() {
        let env = Env::default();
        let profile = blank_profile(&env);
        assert_eq!(calculate_score(&profile), 0);
    }

    #[test]
    fn test_tier_boundary_unranked_to_bronze() {
        assert_eq!(tier_from_score(99), ReputationTier::Unranked);
        assert_eq!(tier_from_score(100), ReputationTier::Bronze);
    }

    #[test]
    fn test_tier_boundary_gold_to_platinum() {
        assert_eq!(tier_from_score(899), ReputationTier::Gold);
        assert_eq!(tier_from_score(900), ReputationTier::Platinum);
    }

    #[test]
    fn test_rejection_lowers_score() {
        let env = Env::default();
        env.mock_all_auths();
        let mut profile = blank_profile(&env);
        profile.milestones_completed = 5;
        profile.total_earned = 1_000_000_000;
        let score_before = calculate_score(&profile);

        profile.milestones_rejected = 3;
        let score_after = calculate_score(&profile);
        assert!(score_after < score_before);
    }

    #[test]
    fn test_perfect_delivery_scores_high() {
        let env = Env::default();
        let mut profile = blank_profile(&env);
        profile.milestones_completed = 10;
        profile.total_earned = EARNINGS_NORMALIZATION;
        let score = calculate_score(&profile);
        assert!(
            score >= 700,
            "expected gold+ for perfect delivery, got {score}"
        );
    }

    #[test]
    fn test_score_does_not_exceed_max() {
        let env = Env::default();
        let mut profile = blank_profile(&env);
        profile.milestones_completed = 1000;
        profile.total_earned = i128::MAX / 2;
        let score = calculate_score(&profile);
        assert!(score <= 1000);
    }
}
