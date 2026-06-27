use soroban_sdk::{Env, Vec};

use crate::constants;
use crate::storage::{DataKey, GrantKey, Storage};
use crate::types::{ExportGrant, ExportGrantPage, ExportMilestone, ExportMilestonePage};

fn cap_limit(limit: u32) -> u32 {
    if limit > constants::MAX_EXPORT_PAGE_SIZE {
        constants::MAX_EXPORT_PAGE_SIZE
    } else {
        limit
    }
}

pub fn export_grants(
    env: &Env,
    offset: u32,
    limit: u32,
    last_updated_after: Option<u64>,
) -> ExportGrantPage {
    let capped = cap_limit(limit);

    let global_order_key = DataKey::Grant(GrantKey::GlobalOrder);
    let all_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&global_order_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut filtered = soroban_sdk::Vec::new(env);
    for i in 0..all_ids.len() {
        let gid = all_ids.get(i).unwrap();
        let last_updated = get_last_updated(env, gid);
        if let Some(after) = last_updated_after {
            if last_updated <= after {
                continue;
            }
        }
        filtered.push_back(gid);
    }

    let total_filtered: u32 = filtered.len();
    let mut page_items = soroban_sdk::Vec::new(env);
    let mut count = 0u32;
    let mut i = offset;
    while i < filtered.len() && count < capped {
        let gid = filtered.get(i).unwrap();
        if let Some(grant) = Storage::get_grant(env, gid) {
            let paid_out = total_paid_out_for_grant(env, gid, grant.total_milestones);
            let contributor = grant.funders.first().map(|f| f.funder.clone());
            let export = ExportGrant {
                id: grant.id,
                owner: grant.owner,
                contributor,
                token: grant.token,
                total_amount: grant.total_amount,
                paid_out,
                status: grant.status,
                milestone_count: grant.total_milestones,
                created_at: grant.timestamp,
                last_updated_at: get_last_updated(env, gid),
            };
            page_items.push_back(export);
        }
        count += 1;
        i += 1;
    }

    let has_more = (offset + capped) < total_filtered;

    ExportGrantPage {
        items: page_items,
        total: total_filtered,
        offset,
        has_more,
    }
}

pub fn export_milestones(env: &Env, grant_id: u64) -> Vec<ExportMilestone> {
    let grant = match Storage::get_grant(env, grant_id) {
        Some(g) => g,
        None => return soroban_sdk::Vec::new(env),
    };

    let mut result = soroban_sdk::Vec::new(env);
    for idx in 0..grant.total_milestones {
        if let Some(milestone) = Storage::get_milestone(env, grant_id, idx) {
            let submitted_at = if milestone.submission_timestamp > 0 {
                Some(milestone.submission_timestamp)
            } else {
                None
            };
            let approved_at = if milestone.state == crate::types::MilestoneState::Approved {
                Some(milestone.status_updated_at)
            } else {
                None
            };
            let export = ExportMilestone {
                grant_id,
                milestone_idx: idx,
                state: milestone.state,
                amount: milestone.amount,
                submitted_at,
                approved_at,
                proof_url: milestone.proof_url,
            };
            result.push_back(export);
        }
    }

    result
}

pub fn export_milestones_since(
    env: &Env,
    since: u64,
    offset: u32,
    limit: u32,
) -> ExportMilestonePage {
    let capped = cap_limit(limit);

    let global_order_key = DataKey::Grant(GrantKey::GlobalOrder);
    let all_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&global_order_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut all_milestones = soroban_sdk::Vec::new(env);

    for i in 0..all_ids.len() {
        let gid = all_ids.get(i).unwrap();
        if let Some(grant) = Storage::get_grant(env, gid) {
            for idx in 0..grant.total_milestones {
                if let Some(milestone) = Storage::get_milestone(env, gid, idx) {
                    let ts = if milestone.submission_timestamp > 0 {
                        milestone.submission_timestamp
                    } else {
                        milestone.status_updated_at
                    };
                    if ts > since {
                        let submitted_at = if milestone.submission_timestamp > 0 {
                            Some(milestone.submission_timestamp)
                        } else {
                            None
                        };
                        let approved_at =
                            if milestone.state == crate::types::MilestoneState::Approved {
                                Some(milestone.status_updated_at)
                            } else {
                                None
                            };
                        let export = ExportMilestone {
                            grant_id: gid,
                            milestone_idx: idx,
                            state: milestone.state,
                            amount: milestone.amount,
                            submitted_at,
                            approved_at,
                            proof_url: milestone.proof_url,
                        };
                        all_milestones.push_back(export);
                    }
                }
            }
        }
    }

    let total = all_milestones.len();
    let mut page_items = soroban_sdk::Vec::new(env);
    let mut count = 0u32;
    let mut i = offset;
    while i < all_milestones.len() && count < capped {
        page_items.push_back(all_milestones.get(i).unwrap());
        count += 1;
        i += 1;
    }

    let has_more = (offset + capped) < total;

    ExportMilestonePage {
        items: page_items,
        total,
        offset,
        has_more,
    }
}

pub fn last_global_update(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::GlobalLastUpdated)
        .unwrap_or(0u64)
}

pub fn state_fingerprint(env: &Env) -> soroban_sdk::BytesN<32> {
    let counter_key = DataKey::Grant(GrantKey::Counter);
    let total_grants: u64 = env
        .storage()
        .persistent()
        .get(&counter_key)
        .unwrap_or(0u64);

    let last_update = last_global_update(env);

    let mut data = soroban_sdk::Bytes::new(env);
    data.extend_from_slice(&total_grants.to_be_bytes());
    data.extend_from_slice(&last_update.to_be_bytes());

    env.crypto().sha256(&data).into()
}

pub fn set_last_updated(env: &Env, grant_id: u64, timestamp: u64) {
    env.storage().persistent().set(
        &DataKey::GrantLastUpdated(grant_id),
        &timestamp,
    );
    env.storage().persistent().set(
        &DataKey::GlobalLastUpdated,
        &timestamp,
    );
}

fn get_last_updated(env: &Env, grant_id: u64) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::GrantLastUpdated(grant_id))
        .unwrap_or(0u64)
}

fn total_paid_out_for_grant(env: &Env, grant_id: u64, total_milestones: u32) -> i128 {
    let mut total = 0i128;
    for idx in 0..total_milestones {
        if let Some(milestone) = Storage::get_milestone(env, grant_id, idx) {
            if milestone.state == crate::types::MilestoneState::Paid {
                total += milestone.amount;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env};

    fn setup() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        (env, admin)
    }

    #[test]
    fn test_export_grants_empty() {
        let (env, _admin) = setup();
        let result = export_grants(&env, 0, 10, None);
        assert_eq!(result.total, 0);
        assert!(!result.has_more);
        assert!(result.items.is_empty());
    }

    #[test]
    fn test_last_global_update_default() {
        let (env, _admin) = setup();
        assert_eq!(last_global_update(&env), 0);
    }

    #[test]
    fn test_set_and_get_last_updated() {
        let (env, _admin) = setup();
        set_last_updated(&env, 1, 1000);
        assert_eq!(get_last_updated(&env, 1), 1000);
        assert_eq!(last_global_update(&env), 1000);
    }

    #[test]
    fn test_export_milestones_empty_grant() {
        let (env, _admin) = setup();
        let result = export_milestones(&env, 999);
        assert!(result.is_empty());
    }

    #[test]
    fn test_state_fingerprint_is_stable() {
        let (env, _admin) = setup();
        let fp1 = state_fingerprint(&env);
        let fp2 = state_fingerprint(&env);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_cap_limit() {
        assert_eq!(cap_limit(0), 0);
        assert_eq!(cap_limit(25), 25);
        assert_eq!(cap_limit(100), constants::MAX_EXPORT_PAGE_SIZE);
    }
}
