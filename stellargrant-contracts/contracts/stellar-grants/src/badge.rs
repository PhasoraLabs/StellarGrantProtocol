use soroban_sdk::{contractevent, contracttype, Address, Env, Vec};

use crate::types::{BadgeCriteria, BadgeRecord, BadgeType, ContractError};
use crate::Storage;

#[contracttype]
pub enum BadgeKey {
    Badge(Address, BadgeType),
    BadgeList(Address),
    BadgeRegistry,
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
    env.storage()
        .persistent()
        .get(&BadgeKey::BadgeList(contributor.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

fn has_badge_raw(env: &Env, contributor: &Address, badge_type: &BadgeType) -> bool {
    env.storage()
        .persistent()
        .has(&BadgeKey::Badge(contributor.clone(), badge_type.clone()))
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

    let mut badges = get_badges_raw(env, contributor);
    badges.push_back(record.clone());
    env.storage()
        .persistent()
        .set(&BadgeKey::BadgeList(contributor.clone()), &badges);

    let count: u32 = env
        .storage()
        .persistent()
        .get(&BadgeKey::BadgeAwardCount(badge_type.clone()))
        .unwrap_or(0);
    env.storage().persistent().set(
        &BadgeKey::BadgeAwardCount(badge_type.clone()),
        &count.saturating_add(1),
    );

    let mut registry: Vec<BadgeRecord> = env
        .storage()
        .persistent()
        .get(&BadgeKey::BadgeRegistry)
        .unwrap_or_else(|| Vec::new(env));
    registry.push_back(record.clone());
    env.storage()
        .persistent()
        .set(&BadgeKey::BadgeRegistry, &registry);

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
    env.storage()
        .persistent()
        .get(&BadgeKey::BadgeAwardCount(badge_type))
        .unwrap_or(0)
}

pub fn manual_award(
    env: &Env,
    admin: &Address,
    contributor: &Address,
    badge_type: BadgeType,
) -> Result<(), ContractError> {
    admin.require_auth();
    if Storage::get_treasury(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    write_award(env, contributor, badge_type, None, None);
    Ok(())
}
