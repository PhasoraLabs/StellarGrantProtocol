use soroban_sdk::{Env, Vec};

use crate::constants;
use crate::storage::{DataKey, GrantKey, Storage};
use crate::types::{ExportGrant, ExportGrantPage, ExportMilestone, ExportMilestonePage};

const MAX_GRANTS_PER_SCAN: u32 = 1_000;

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

    let start = core::cmp::min(offset, all_ids.len());
    let scan_limit = core::cmp::min(MAX_GRANTS_PER_SCAN, all_ids.len().saturating_sub(start));
    let scan_end = start.saturating_add(scan_limit);
    let mut filtered = soroban_sdk::Vec::new(env);
    for i in start..scan_end {
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
    let mut i = 0;
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

    let has_more = scan_end < all_ids.len() || total_filtered > capped;

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
            let approved_at = if milestone.state == crate::types::MilestoneState::Approved
                || milestone.state == crate::types::MilestoneState::Paid
            {
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
    let start = core::cmp::min(offset, all_ids.len());
    let scan_limit = core::cmp::min(MAX_GRANTS_PER_SCAN, all_ids.len().saturating_sub(start));
    let scan_end = start.saturating_add(scan_limit);

    for i in start..scan_end {
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
    let mut i = 0;
    while i < all_milestones.len() && count < capped {
        page_items.push_back(all_milestones.get(i).unwrap());
        count += 1;
        i += 1;
    }

    let has_more = scan_end < all_ids.len() || total > capped;

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
    let total_grants: u64 = env.storage().persistent().get(&counter_key).unwrap_or(0u64);

    let last_update = last_global_update(env);

    let mut data = soroban_sdk::Bytes::new(env);
    data.extend_from_slice(&total_grants.to_be_bytes());
    data.extend_from_slice(&last_update.to_be_bytes());

    env.crypto().sha256(&data).into()
}

pub fn set_last_updated(env: &Env, grant_id: u64, timestamp: u64) {
    env.storage()
        .persistent()
        .set(&DataKey::GrantLastUpdated(grant_id), &timestamp);
    env.storage()
        .persistent()
        .set(&DataKey::GlobalLastUpdated, &timestamp);
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

    macro_rules! export_test {
        ($name:ident, |$env:ident| $body:block) => {
            #[test]
            fn $name() {
                let ($env, _) = setup();
                let contract_id = $env.register(crate::StellarGrantsContract, ());
                $env.as_contract(&contract_id, || $body);
            }
        };
    }

    export_test!(test_export_grants_empty, |env| {
        let result = export_grants(&env, 0, 10, None);
        assert_eq!(result.total, 0);
        assert!(!result.has_more);
        assert!(result.items.is_empty());
    });

    export_test!(test_last_global_update_default, |env| {
        assert_eq!(last_global_update(&env), 0);
    });

    export_test!(test_set_and_get_last_updated, |env| {
        set_last_updated(&env, 1, 1000);
        assert_eq!(get_last_updated(&env, 1), 1000);
        assert_eq!(last_global_update(&env), 1000);
    });

    export_test!(test_export_milestones_empty_grant, |env| {
        let result = export_milestones(&env, 999);
        assert!(result.is_empty());
    });

    export_test!(test_state_fingerprint_is_stable, |env| {
        let fp1 = state_fingerprint(&env);
        let fp2 = state_fingerprint(&env);
        assert_eq!(fp1, fp2);
    });

    #[test]
    fn test_cap_limit() {
        assert_eq!(cap_limit(0), 0);
        assert_eq!(cap_limit(25), 25);
        assert_eq!(cap_limit(100), constants::MAX_EXPORT_PAGE_SIZE);
    }

    export_test!(test_export_grants_overflow, |env| {
        let result = export_grants(&env, u32::MAX - 5, 10, None);
        assert!(!result.has_more);
    });

    export_test!(test_export_milestones_since_overflow, |env| {
        let result = export_milestones_since(&env, 0, u32::MAX - 5, 10);
        assert!(!result.has_more);
    });

    fn make_grant(env: &Env, owner: &Address) -> crate::types::Grant {
        crate::types::Grant {
            id: 1,
            owner: owner.clone(),
            title: soroban_sdk::String::from_str(env, "Test Grant"),
            description: soroban_sdk::String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: crate::types::GrantStatus::Active,
            total_amount: 1_000,
            milestone_amount: 1_000,
            reviewers: soroban_sdk::Vec::new(env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: soroban_sdk::Vec::new(env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        }
    }

    // Regression test for issue #886: `export_grants` compared a `usize`
    // loop index against `all_ids.len()` (`u32`) without casting, which
    // failed to compile with E0308. Confirms the fixed scan loop still
    // bounds itself to `MAX_GRANTS_PER_SCAN` even when `GlobalOrder` holds
    // more ids than that (issue #855's cost-bounding behavior).
    export_test!(test_export_grants_stops_at_max_grants_per_scan, |env| {
        env.cost_estimate().disable_resource_limits();
        env.cost_estimate().budget().reset_unlimited();
        let overflow_count = MAX_GRANTS_PER_SCAN + 5;
        let mut ids = soroban_sdk::Vec::new(&env);
        for i in 1..=overflow_count as u64 {
            ids.push_back(i);
        }
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::GlobalOrder), &ids);

        let result = export_grants(&env, 0, 10, None);
        // Only the first MAX_GRANTS_PER_SCAN ids are ever scanned, regardless
        // of how many more are queued in GlobalOrder.
        assert_eq!(result.total, MAX_GRANTS_PER_SCAN);
        assert!(result.has_more);
    });

    #[test]
    fn test_export_grants_scans_from_offset_past_scan_limit() {
        let (env, owner) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            let grant_count = MAX_GRANTS_PER_SCAN + 5;
            let mut ids = soroban_sdk::Vec::new(&env);
            for id in 1..=grant_count as u64 {
                ids.push_back(id);
            }
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::GlobalOrder), &ids);

            let grant_id = MAX_GRANTS_PER_SCAN as u64 + 1;
            let mut grant = make_grant(&env, &owner);
            grant.id = grant_id;
            Storage::set_grant(&env, grant_id, &grant);

            let result = export_grants(&env, MAX_GRANTS_PER_SCAN, 10, None);

            assert_eq!(result.items.len(), 1);
            assert_eq!(result.items.get(0).unwrap().id, grant_id);
            assert!(!result.has_more);
        });
    }

    // Regression test for issue #886, `export_milestones_since` half.
    // A real grant/milestone placed right at the scan boundary
    // (id == MAX_GRANTS_PER_SCAN) must be counted; one placed just past it
    // must not be, proving the scan actually stops at the limit rather than
    // merely not panicking.
    #[test]
    fn test_export_milestones_since_stops_at_max_grants_per_scan() {
        let (env, owner) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            env.cost_estimate().disable_resource_limits();
            env.cost_estimate().budget().reset_unlimited();
            let overflow_count = MAX_GRANTS_PER_SCAN + 5;
            let mut ids = soroban_sdk::Vec::new(&env);
            for i in 1..=overflow_count as u64 {
                ids.push_back(i);
            }
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::GlobalOrder), &ids);

            let included_id = MAX_GRANTS_PER_SCAN as u64;
            let excluded_id = included_id + 1;
            for gid in [included_id, excluded_id] {
                Storage::set_grant(&env, gid, &make_grant(&env, &owner));
                Storage::set_milestone(
                    &env,
                    gid,
                    0,
                    &crate::types::Milestone {
                        idx: 0,
                        description: soroban_sdk::String::from_str(&env, "m"),
                        amount: 100,
                        state: crate::types::MilestoneState::Submitted,
                        votes: soroban_sdk::Map::new(&env),
                        approvals: 0,
                        rejections: 0,
                        reasons: soroban_sdk::Map::new(&env),
                        status_updated_at: 1,
                        proof_url: None,
                        submission_timestamp: 1,
                        deadline: None,
                        reviewer_count_snapshot: 0,
                        reviewer_list_snapshot: soroban_sdk::Vec::new(&env),
                    },
                );
            }

            let result = export_milestones_since(&env, 0, 0, 50);
            assert_eq!(result.total, 1);
        });
    }

    #[test]
    fn test_export_milestones_since_scans_from_offset_past_scan_limit() {
        let (env, owner) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            let grant_count = MAX_GRANTS_PER_SCAN + 5;
            let mut ids = soroban_sdk::Vec::new(&env);
            for id in 1..=grant_count as u64 {
                ids.push_back(id);
            }
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::GlobalOrder), &ids);

            let grant_id = MAX_GRANTS_PER_SCAN as u64 + 1;
            let mut grant = make_grant(&env, &owner);
            grant.id = grant_id;
            Storage::set_grant(&env, grant_id, &grant);
            Storage::set_milestone(
                &env,
                grant_id,
                0,
                &crate::types::Milestone {
                    idx: 0,
                    description: soroban_sdk::String::from_str(&env, "m"),
                    amount: 100,
                    state: crate::types::MilestoneState::Submitted,
                    votes: soroban_sdk::Map::new(&env),
                    approvals: 0,
                    rejections: 0,
                    reasons: soroban_sdk::Map::new(&env),
                    status_updated_at: 1,
                    proof_url: None,
                    submission_timestamp: 1,
                    deadline: None,
                    reviewer_count_snapshot: 0,
                    reviewer_list_snapshot: soroban_sdk::Vec::new(&env),
                },
            );

            let result = export_milestones_since(&env, 0, MAX_GRANTS_PER_SCAN, 10);

            assert_eq!(result.items.len(), 1);
            assert_eq!(result.items.get(0).unwrap().grant_id, grant_id);
            assert!(!result.has_more);
        });
    }
}
