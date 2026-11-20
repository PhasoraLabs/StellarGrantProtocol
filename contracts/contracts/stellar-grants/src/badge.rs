use soroban_sdk::{contractevent, contracttype, Address, Env, Vec};

use crate::types::{BadgeCriteria, BadgeRecord, BadgeType, ContractError};
use crate::Storage;

const BADGE_TTL_THRESHOLD: u32 = 100_000;
const BADGE_TTL_EXTEND_TO: u32 = 1_000_000;

#[contracttype]
pub enum BadgeKey {
    Badge(Address, BadgeType),
    BadgeList(Address),
    BadgeAwardCount(BadgeType),
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadgeAwarded {
    pub contributor: Address,
    pub badge_type: BadgeType,
    pub grant_id: Option<u64>,
    pub awarded_at: u64,
}

fn get_badges_raw(env: &Env, contributor: &Address) -> Vec<BadgeRecord> {
    let key = BadgeKey::BadgeList(contributor.clone());
    let exists = env.storage().persistent().has(&key);
    if exists {
        env.storage()
            .persistent()
            .extend_ttl(&key, BADGE_TTL_THRESHOLD, BADGE_TTL_EXTEND_TO);
    }
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env))
}

fn has_badge_raw(env: &Env, contributor: &Address, badge_type: &BadgeType) -> bool {
    let key = BadgeKey::Badge(contributor.clone(), badge_type.clone());
    let exists = env.storage().persistent().has(&key);
    if exists {
        env.storage()
            .persistent()
            .extend_ttl(&key, BADGE_TTL_THRESHOLD, BADGE_TTL_EXTEND_TO);
    }
    exists
}

fn meets_criteria(env: &Env, contributor: &Address, criteria: &BadgeCriteria) -> bool {
    let profile = match Storage::get_contributor(env, contributor.clone()) {
        Some(profile) => profile,
        None => return false,
    };
    if let Some(required) = criteria.required_milestones {
        if profile.milestones_completed < required {
            return false;
        }
    }
    if let Some(required) = criteria.required_reputation {
        if profile.reputation_score < required as u64 {
            return false;
        }
    }
    if let Some(required) = criteria.required_grants {
        if profile.grants_count < required {
            return false;
        }
    }
    true
}

fn write_award(
    env: &Env,
    contributor: &Address,
    badge_type: BadgeType,
    grant_id: Option<u64>,
    milestone_idx: Option<u32>,
) -> bool {
    if has_badge_raw(env, contributor, &badge_type) {
        return false;
    }
    let record = BadgeRecord {
        badge_type: badge_type.clone(),
        recipient: contributor.clone(),
        awarded_at: env.ledger().timestamp(),
        grant_id,
        milestone_idx,
    };
    env.storage().persistent().set(
        &BadgeKey::Badge(contributor.clone(), badge_type.clone()),
        &record,
    );
    // Extend TTL for the badge entry
    env.storage().persistent().extend_ttl(
        &BadgeKey::Badge(contributor.clone(), badge_type.clone()),
        BADGE_TTL_THRESHOLD,
        BADGE_TTL_EXTEND_TO,
    );

    let mut badges = get_badges_raw(env, contributor);
    badges.push_back(record.clone());
    let list_key = BadgeKey::BadgeList(contributor.clone());
    env.storage().persistent().set(&list_key, &badges);
    env.storage()
        .persistent()
        .extend_ttl(&list_key, BADGE_TTL_THRESHOLD, BADGE_TTL_EXTEND_TO);

    let count_key = BadgeKey::BadgeAwardCount(badge_type.clone());
    let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
    env.storage()
        .persistent()
        .set(&count_key, &count.saturating_add(1));
    env.storage()
        .persistent()
        .extend_ttl(&count_key, BADGE_TTL_THRESHOLD, BADGE_TTL_EXTEND_TO);

    BadgeAwarded {
        contributor: contributor.clone(),
        badge_type,
        grant_id,
        awarded_at: record.awarded_at,
    }
    .publish(env);
    true
}

pub fn try_award(
    env: &Env,
    contributor: &Address,
    badge_type: BadgeType,
    grant_id: Option<u64>,
    milestone_idx: Option<u32>,
) -> bool {
    let criteria = get_criteria(badge_type.clone());
    if criteria.one_time && has_badge_raw(env, contributor, &badge_type) {
        return false;
    }
    if !meets_criteria(env, contributor, &criteria) {
        return false;
    }
    write_award(env, contributor, badge_type, grant_id, milestone_idx)
}

pub fn get_badges(env: &Env, contributor: &Address) -> Vec<BadgeRecord> {
    get_badges_raw(env, contributor)
}

pub fn has_badge(env: &Env, contributor: &Address, badge_type: BadgeType) -> bool {
    has_badge_raw(env, contributor, &badge_type)
}

pub fn get_criteria(badge_type: BadgeType) -> BadgeCriteria {
    match badge_type {
        BadgeType::FirstMilestone => BadgeCriteria {
            badge_type,
            required_milestones: Some(1),
            required_reputation: None,
            required_grants: None,
            one_time: true,
        },
        BadgeType::TenMilestones => BadgeCriteria {
            badge_type,
            required_milestones: Some(10),
            required_reputation: None,
            required_grants: None,
            one_time: true,
        },
        BadgeType::FiftyMilestones => BadgeCriteria {
            badge_type,
            required_milestones: Some(50),
            required_reputation: None,
            required_grants: None,
            one_time: true,
        },
        BadgeType::BronzeContributor => BadgeCriteria {
            badge_type,
            required_milestones: None,
            required_reputation: Some(100),
            required_grants: None,
            one_time: true,
        },
        BadgeType::SilverContributor => BadgeCriteria {
            badge_type,
            required_milestones: None,
            required_reputation: Some(400),
            required_grants: None,
            one_time: true,
        },
        BadgeType::GoldContributor => BadgeCriteria {
            badge_type,
            required_milestones: None,
            required_reputation: Some(700),
            required_grants: None,
            one_time: true,
        },
        BadgeType::PlatinumContributor => BadgeCriteria {
            badge_type,
            required_milestones: None,
            required_reputation: Some(900),
            required_grants: None,
            one_time: true,
        },
        BadgeType::EarlyAdopter => BadgeCriteria {
            badge_type,
            required_milestones: None,
            required_reputation: None,
            required_grants: Some(1),
            one_time: true,
        },
        _ => BadgeCriteria {
            badge_type,
            required_milestones: None,
            required_reputation: None,
            required_grants: None,
            one_time: true,
        },
    }
}

pub fn award_count(env: &Env, badge_type: BadgeType) -> u32 {
    let key = BadgeKey::BadgeAwardCount(badge_type);
    let count = env.storage().persistent().get(&key);
    if count.is_some() {
        env.storage()
            .persistent()
            .extend_ttl(&key, BADGE_TTL_THRESHOLD, BADGE_TTL_EXTEND_TO);
    }
    count.unwrap_or(0)
}

pub fn manual_award(
    env: &Env,
    admin: &Address,
    contributor: &Address,
    badge_type: BadgeType,
) -> Result<(), ContractError> {
    admin.require_auth();
    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    write_award(env, contributor, badge_type, None, None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContributorProfile;
    use soroban_sdk::testutils::{storage::Persistent as _, Address as _, Ledger as _};
    use soroban_sdk::{Env, String, Vec};

    fn run<T>(env: &Env, f: impl FnOnce() -> T) -> T {
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, f)
    }

    fn setup_ttl_test() -> (Env, Address) {
        let env = Env::default();
        env.ledger().with_mut(|ledger| {
            ledger.max_entry_ttl = BADGE_TTL_EXTEND_TO * 2;
        });
        let contract = env.register(crate::StellarGrantsContract, ());
        (env, contract)
    }

    fn decay_ttl(env: &Env) {
        env.ledger().with_mut(|ledger| {
            ledger.sequence_number += BADGE_TTL_EXTEND_TO - BADGE_TTL_THRESHOLD / 2;
        });
    }

    #[test]
    fn awards_extend_all_written_keys_and_preserve_lists_and_counts() {
        let (env, contract) = setup_ttl_test();
        let contributor = Address::generate(&env);
        let other_contributor = Address::generate(&env);
        env.as_contract(&contract, || {
            assert!(write_award(
                &env,
                &contributor,
                BadgeType::FirstMilestone,
                Some(1),
                Some(0)
            ));
            for key in [
                BadgeKey::Badge(contributor.clone(), BadgeType::FirstMilestone),
                BadgeKey::BadgeList(contributor.clone()),
                BadgeKey::BadgeAwardCount(BadgeType::FirstMilestone),
            ] {
                assert_eq!(
                    env.storage().persistent().get_ttl(&key),
                    BADGE_TTL_EXTEND_TO
                );
            }
        });

        decay_ttl(&env);
        env.as_contract(&contract, || {
            for key in [
                BadgeKey::BadgeList(contributor.clone()),
                BadgeKey::BadgeAwardCount(BadgeType::FirstMilestone),
            ] {
                assert!(env.storage().persistent().get_ttl(&key) < BADGE_TTL_THRESHOLD);
            }
            assert!(write_award(
                &env,
                &contributor,
                BadgeType::TenMilestones,
                Some(2),
                Some(0)
            ));
            assert!(write_award(
                &env,
                &other_contributor,
                BadgeType::FirstMilestone,
                Some(3),
                Some(0)
            ));
            // Check before the getters can refresh either key.
            for key in [
                BadgeKey::BadgeList(contributor.clone()),
                BadgeKey::BadgeAwardCount(BadgeType::FirstMilestone),
            ] {
                assert_eq!(
                    env.storage().persistent().get_ttl(&key),
                    BADGE_TTL_EXTEND_TO
                );
            }
            let badges = get_badges(&env, &contributor);
            assert_eq!(badges.len(), 2);
            assert_eq!(badges.get(0).unwrap().badge_type, BadgeType::FirstMilestone);
            assert_eq!(badges.get(1).unwrap().badge_type, BadgeType::TenMilestones);
            assert_eq!(award_count(&env, BadgeType::FirstMilestone), 2);
            assert_eq!(award_count(&env, BadgeType::TenMilestones), 1);
        });
    }

    #[test]
    fn badge_reads_renew_all_existing_keys() {
        let (env, contract) = setup_ttl_test();
        let contributor = Address::generate(&env);
        env.as_contract(&contract, || {
            assert!(write_award(
                &env,
                &contributor,
                BadgeType::FirstMilestone,
                Some(1),
                Some(0)
            ));
        });
        decay_ttl(&env);
        env.as_contract(&contract, || {
            for key in [
                BadgeKey::Badge(contributor.clone(), BadgeType::FirstMilestone),
                BadgeKey::BadgeList(contributor.clone()),
                BadgeKey::BadgeAwardCount(BadgeType::FirstMilestone),
            ] {
                assert!(env.storage().persistent().get_ttl(&key) < BADGE_TTL_THRESHOLD);
            }
            assert!(has_badge(&env, &contributor, BadgeType::FirstMilestone));
            assert_eq!(get_badges(&env, &contributor).len(), 1);
            assert_eq!(award_count(&env, BadgeType::FirstMilestone), 1);
            for key in [
                BadgeKey::Badge(contributor.clone(), BadgeType::FirstMilestone),
                BadgeKey::BadgeList(contributor.clone()),
                BadgeKey::BadgeAwardCount(BadgeType::FirstMilestone),
            ] {
                assert_eq!(
                    env.storage().persistent().get_ttl(&key),
                    BADGE_TTL_EXTEND_TO
                );
            }
        });
    }

    #[test]
    fn missing_badge_reads_do_not_create_entries() {
        let (env, contract) = setup_ttl_test();
        let contributor = Address::generate(&env);
        env.as_contract(&contract, || {
            assert!(!has_badge(&env, &contributor, BadgeType::FirstMilestone));
            assert!(get_badges(&env, &contributor).is_empty());
            assert_eq!(award_count(&env, BadgeType::FirstMilestone), 0);
            for key in [
                BadgeKey::Badge(contributor.clone(), BadgeType::FirstMilestone),
                BadgeKey::BadgeList(contributor.clone()),
                BadgeKey::BadgeAwardCount(BadgeType::FirstMilestone),
            ] {
                assert!(!env.storage().persistent().has(&key));
            }
        });
    }

    fn set_profile(
        env: &Env,
        contributor: &Address,
        milestones_completed: u32,
        reputation_score: u64,
        grants_count: u32,
    ) {
        let profile = ContributorProfile {
            contributor: contributor.clone(),
            name: String::from_str(env, "c"),
            bio: String::from_str(env, ""),
            skills: Vec::new(env),
            github_url: String::from_str(env, ""),
            registration_timestamp: 0,
            reputation_score,
            grants_count,
            total_earned: 0,
            milestones_completed,
            milestones_rejected: 0,
            last_action_at: 0,
        };
        Storage::set_contributor(env, contributor.clone(), &profile);
    }

    #[test]
    fn award_on_milestone_completion_path() {
        let env = Env::default();
        let contributor = Address::generate(&env);

        run(&env, || {
            // A contributor who has just completed their first milestone.
            set_profile(&env, &contributor, 1, 0, 0);

            // try_award is exactly what the milestone-approval payout path in lib.rs calls.
            assert!(try_award(
                &env,
                &contributor,
                BadgeType::FirstMilestone,
                Some(7),
                Some(0),
            ));

            assert!(has_badge(&env, &contributor, BadgeType::FirstMilestone));
            let badges = get_badges(&env, &contributor);
            assert_eq!(badges.len(), 1);
            let record = badges.get(0).unwrap();
            assert_eq!(record.badge_type, BadgeType::FirstMilestone);
            assert_eq!(record.grant_id, Some(7));
            assert_eq!(record.milestone_idx, Some(0));
            assert_eq!(award_count(&env, BadgeType::FirstMilestone), 1);
        });
    }

    #[test]
    fn one_time_badge_not_re_awarded_across_grants() {
        let env = Env::default();
        let contributor = Address::generate(&env);

        run(&env, || {
            set_profile(&env, &contributor, 5, 0, 0);

            assert!(try_award(
                &env,
                &contributor,
                BadgeType::FirstMilestone,
                Some(1),
                Some(0),
            ));
            // Second qualifying milestone on a different grant — must not re-award.
            assert!(!try_award(
                &env,
                &contributor,
                BadgeType::FirstMilestone,
                Some(2),
                Some(0),
            ));

            let badges = get_badges(&env, &contributor);
            assert_eq!(badges.len(), 1);
            assert_eq!(badges.get(0).unwrap().grant_id, Some(1));
            assert_eq!(award_count(&env, BadgeType::FirstMilestone), 1);
        });
    }

    #[test]
    fn award_denied_when_criteria_not_met() {
        let env = Env::default();
        let contributor = Address::generate(&env);

        run(&env, || {
            set_profile(&env, &contributor, 0, 0, 0);
            assert!(!try_award(
                &env,
                &contributor,
                BadgeType::FirstMilestone,
                None,
                None
            ));
            assert!(!has_badge(&env, &contributor, BadgeType::FirstMilestone));
            assert_eq!(get_badges(&env, &contributor).len(), 0);
        });
    }

    #[test]
    fn award_denied_for_unregistered_contributor() {
        let env = Env::default();
        let contributor = Address::generate(&env);

        run(&env, || {
            assert!(!try_award(
                &env,
                &contributor,
                BadgeType::FirstMilestone,
                None,
                None
            ));
            assert!(!has_badge(&env, &contributor, BadgeType::FirstMilestone));
        });
    }

    #[test]
    fn reputation_threshold_badge_gate() {
        let env = Env::default();
        let contributor = Address::generate(&env);

        run(&env, || {
            set_profile(&env, &contributor, 0, 99, 0);
            assert!(!try_award(
                &env,
                &contributor,
                BadgeType::BronzeContributor,
                None,
                None
            ));

            set_profile(&env, &contributor, 0, 100, 0);
            assert!(try_award(
                &env,
                &contributor,
                BadgeType::BronzeContributor,
                None,
                None
            ));
            assert!(has_badge(&env, &contributor, BadgeType::BronzeContributor));
        });
    }

    #[test]
    fn get_criteria_matches_badge_type() {
        let c = get_criteria(BadgeType::FirstMilestone);
        assert_eq!(c.required_milestones, Some(1));
        assert!(c.one_time);
        assert_eq!(
            get_criteria(BadgeType::EarlyAdopter).required_grants,
            Some(1)
        );
    }

    #[test]
    fn manual_award_is_admin_gated() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let stranger = Address::generate(&env);
        let contributor = Address::generate(&env);

        run(&env, || {
            Storage::set_global_admin(&env, &admin);

            assert_eq!(
                manual_award(&env, &stranger, &contributor, BadgeType::DisputeWinner),
                Err(ContractError::Unauthorized)
            );

            manual_award(&env, &admin, &contributor, BadgeType::DisputeWinner).unwrap();
            assert!(has_badge(&env, &contributor, BadgeType::DisputeWinner));
        });
    }
}
