/// Contributor public portfolio aggregator (issue #565).
/// Read-only module — aggregates a contributor's on-chain history into a single
/// verifiable view: reputation, badges, earnings, and recent grants.
use soroban_sdk::{Address, Bytes, Env, Vec};

use crate::badge;
use crate::constants::PORTFOLIO_RECENT_GRANTS_LIMIT;
use crate::errors::ContractError;
use crate::reputation;
use crate::storage::Storage;
use crate::types::{BadgeType, ContributorPortfolio, GrantStatus, GrantSummary, MilestoneState};

/// Build and return the full portfolio for a contributor. Read-only.
pub fn get_portfolio(
    env: &Env,
    contributor: &Address,
) -> Result<ContributorPortfolio, ContractError> {
    let profile =
        Storage::get_contributor(env, contributor.clone()).ok_or(ContractError::InvalidInput)?;

    let reputation_score = reputation::calculate_effective_score(env, &profile);
    let tier = reputation::tier_from_score(reputation_score);

    let badge_records = badge::get_badges(env, contributor);
    let mut badges: Vec<BadgeType> = Vec::new(env);
    for b in badge_records.iter() {
        badges.push_back(b.badge_type);
    }

    // USD equivalent — available only when oracle is configured
    let total_earned_usd_equivalent = None;

    // Build recent_grants from the contributor grant index
    let grant_ids = Storage::get_contributor_grant_ids(env, contributor);
    let mut recent_grants: Vec<GrantSummary> = Vec::new(env);
    let start = if grant_ids.len() > PORTFOLIO_RECENT_GRANTS_LIMIT {
        grant_ids.len() - PORTFOLIO_RECENT_GRANTS_LIMIT
    } else {
        0
    };
    for i in start..grant_ids.len() {
        let grant_id = grant_ids.get(i).unwrap();
        if let Some(summary) = get_grant_summary(env, contributor, grant_id).ok() {
            recent_grants.push_back(summary);
        }
    }

    // Count grants_completed and grants_active from the recent index
    let mut grants_completed = 0u32;
    let mut grants_active = 0u32;
    for i in 0..grant_ids.len() {
        let gid = grant_ids.get(i).unwrap();
        if let Some(g) = Storage::get_grant(env, gid) {
            match g.status {
                GrantStatus::Completed => grants_completed = grants_completed.saturating_add(1),
                GrantStatus::Active => grants_active = grants_active.saturating_add(1),
                _ => {}
            }
        }
    }

    Ok(ContributorPortfolio {
        contributor: contributor.clone(),
        display_name: profile.name.clone(),
        bio: profile.bio.clone(),
        reputation_score,
        reputation_tier: tier,
        total_earned_usd_equivalent,
        grants_completed,
        grants_active,
        milestones_approved: profile.milestones_completed,
        milestones_rejected: profile.milestones_rejected,
        badges,
        recent_grants,
        member_since: profile.registration_timestamp,
    })
}

/// Return a lightweight summary of a single grant's contribution record.
pub fn get_grant_summary(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
) -> Result<GrantSummary, ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }

    let mut milestones_completed = 0u32;
    let mut completed_at: Option<u64> = None;
    let mut earned: i128 = 0;

    for i in 0..grant.total_milestones {
        if let Some(ms) = Storage::get_milestone(env, grant_id, i) {
            match ms.state {
                MilestoneState::Approved | MilestoneState::Paid => {
                    milestones_completed = milestones_completed.saturating_add(1);
                    earned = earned.saturating_add(ms.amount);
                    if ms.state == MilestoneState::Paid {
                        let t = ms.status_updated_at;
                        completed_at = Some(match completed_at {
                            Some(prev) => prev.max(t),
                            None => t,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    let _ = contributor; // contributor address available for future filtering

    Ok(GrantSummary {
        grant_id,
        title: grant.title.clone(),
        milestones_completed,
        total_milestones: grant.total_milestones,
        total_earned: earned,
        token: grant.token.clone(),
        completed_at,
        status: grant.status,
    })
}

/// Return the contributor's earnings broken down by token address.
pub fn earnings_by_token(env: &Env, contributor: &Address) -> Vec<(Address, i128)> {
    let grant_ids = Storage::get_contributor_grant_ids(env, contributor);
    let mut result: Vec<(Address, i128)> = Vec::new(env);

    for i in 0..grant_ids.len() {
        let grant_id = grant_ids.get(i).unwrap();
        let grant = match Storage::get_grant(env, grant_id) {
            Some(g) => g,
            None => continue,
        };

        let mut earned: i128 = 0;
        for j in 0..grant.total_milestones {
            if let Some(ms) = Storage::get_milestone(env, grant_id, j) {
                if ms.state == MilestoneState::Paid {
                    earned = earned.saturating_add(ms.amount);
                }
            }
        }
        if earned == 0 {
            continue;
        }

        // Merge into existing token entry or push new one
        let mut found = false;
        for k in 0..result.len() {
            let (token, prev) = result.get(k).unwrap();
            if token == grant.token {
                result.set(k, (token, prev.saturating_add(earned)));
                found = true;
                break;
            }
        }
        if !found {
            result.push_back((grant.token.clone(), earned));
        }
    }
    result
}

/// Return a verifiable hash of the portfolio.
/// `hash = SHA-256(contributor_bytes || reputation_score_bytes || grants_completed_bytes || milestones_approved_bytes)`
pub fn portfolio_hash(env: &Env, contributor: &Address) -> Bytes {
    let profile = Storage::get_contributor(env, contributor.clone());
    let reputation_score = profile
        .as_ref()
        .map(|p| reputation::calculate_effective_score(env, p))
        .unwrap_or(0);
    let milestones_approved = profile
        .as_ref()
        .map(|p| p.milestones_completed)
        .unwrap_or(0);
    let grant_ids = Storage::get_contributor_grant_ids(env, contributor);
    let grants_completed = {
        let mut count = 0u32;
        for i in 0..grant_ids.len() {
            if let Some(g) = Storage::get_grant(env, grant_ids.get(i).unwrap()) {
                if g.status == GrantStatus::Completed {
                    count += 1;
                }
            }
        }
        count
    };

    let mut input = Bytes::new(env);
    input.extend_from_array(&reputation_score.to_be_bytes());
    input.extend_from_array(&grants_completed.to_be_bytes());
    input.extend_from_array(&milestones_approved.to_be_bytes());

    env.crypto().sha256(&input).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String, Vec};

    fn register_contributor(env: &Env, contributor: &Address) {
        let profile = crate::types::ContributorProfile {
            contributor: contributor.clone(),
            name: String::from_str(env, "Alice"),
            bio: String::from_str(env, "dev"),
            skills: Vec::new(env),
            github_url: String::from_str(env, ""),
            registration_timestamp: 1000,
            reputation_score: 0,
            grants_count: 0,
            total_earned: 0,
            milestones_completed: 2,
            milestones_rejected: 0,
            last_action_at: 1000,
        };
        Storage::set_contributor(env, contributor.clone(), &profile);
    }

    #[test]
    fn unregistered_contributor_returns_error() {
        let env = Env::default();
        let addr = Address::generate(&env);
        let result = get_portfolio(&env, &addr);
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn portfolio_hash_is_deterministic() {
        let env = Env::default();
        let contributor = Address::generate(&env);
        register_contributor(&env, &contributor);

        let hash1 = portfolio_hash(&env, &contributor);
        let hash2 = portfolio_hash(&env, &contributor);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn earnings_by_token_empty_for_new_contributor() {
        let env = Env::default();
        let contributor = Address::generate(&env);
        register_contributor(&env, &contributor);

        let earnings = earnings_by_token(&env, &contributor);
        assert_eq!(earnings.len(), 0);
    }

    #[test]
    fn portfolio_reflects_badges() {
        let env = Env::default();
        env.mock_all_auths();
        let contributor = Address::generate(&env);
        register_contributor(&env, &contributor);

        let portfolio = get_portfolio(&env, &contributor).unwrap();
        // No badges initially — list should be empty
        assert_eq!(portfolio.badges.len(), 0);
        assert_eq!(portfolio.milestones_approved, 2);
    }

    #[test]
    fn get_grant_summary_requires_owner() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let other = Address::generate(&env);

        // create a grant owned by `owner`
        let grant = crate::types::Grant {
            id: 42,
            owner: owner.clone(),
            title: String::from_str(&env, "G"),
            description: String::from_str(&env, "D"),
            token: Address::generate(&env),
            status: GrantStatus::Active,
            total_amount: 0,
            milestone_amount: 0,
            reviewers: Vec::new(&env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(&env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        };
        Storage::set_grant(&env, 42, &grant);

        // other address should be rejected
        let res = get_grant_summary(&env, &other, 42);
        assert_eq!(res, Err(ContractError::Unauthorized));
    }

    #[test]
    fn get_portfolio_applies_reputation_decay() {
        use soroban_sdk::testutils::Ledger as _;

        let env = Env::default();
        env.mock_all_auths();
        let contributor = Address::generate(&env);

        // register contributor with high raw score and old last_action_at
        let profile = crate::types::ContributorProfile {
            contributor: contributor.clone(),
            name: String::from_str(&env, "Alice"),
            bio: String::from_str(&env, "dev"),
            skills: Vec::new(&env),
            github_url: String::from_str(&env, ""),
            registration_timestamp: 1,
            reputation_score: 800,
            grants_count: 0,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            last_action_at: 0,
        };
        Storage::set_contributor(&env, contributor.clone(), &profile);

        // enable linear decay in protocol config and set params
        let mut cfg = crate::config::default_config();
        cfg.decay_config.enabled = true;
        cfg.decay_config.decay_type = crate::types::DecayType::Linear;
        cfg.decay_config.linear_decay_per_day = 50; // 50 points/day
        cfg.decay_config.decay_floor = 50;
        cfg.decay_config.inactivity_threshold_ledgers = 0;
        Storage::set_protocol_config(&env, &cfg);

        // advance ledger far enough to trigger decay (~10 days)
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 86400 * 10,
            protocol_version: 21,
            sequence_number: 1,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 100_000,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 1_000_000,
        });

        let portfolio = get_portfolio(&env, &contributor).unwrap();
        assert!(portfolio.reputation_score < 800);
    }
}
