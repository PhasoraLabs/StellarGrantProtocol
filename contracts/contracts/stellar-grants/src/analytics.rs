use crate::constants;
use crate::storage::Storage;
use crate::types::{AnalyticsSnapshot, CategoryStats, RollingWindow};
use soroban_sdk::{Env, Symbol, Vec};

const MAX_WINDOW_SIZE: u32 = 50;
const STALENESS_THRESHOLD: u32 = 1000; // ledgers

/// Record a data point in a rolling window (max 50 points, evicts oldest).
pub fn record(env: &Env, metric: Symbol, value: i128) {
    let mut window = get_window(env, metric.clone()).unwrap_or_else(|| RollingWindow {
        metric_key: metric.clone(),
        window_size: 0,
        values: Vec::new(env),
        timestamps: Vec::new(env),
        sum: 0,
        count: 0,
    });

    // Evict oldest if window is full
    if window.window_size >= constants::MAX_ROLLING_WINDOW_SIZE {
        let oldest_val = window.values.get(0).unwrap();
        window.sum = window.sum.saturating_sub(oldest_val);
        window.values.remove(0);
        window.timestamps.remove(0);
        window.count -= 1;
        window.window_size -= 1;
    }

    // Add new value
    window.values.push_back(value);
    window.timestamps.push_back(env.ledger().timestamp());
    window.sum = window.sum.saturating_add(value);
    window.count += 1;
    window.window_size += 1;

    Storage::set_rolling_window(env, &metric, &window);
}

/// Compute the rolling average for a metric.
pub fn rolling_average(env: &Env, metric: Symbol) -> Option<i128> {
    let window = get_window(env, metric)?;
    if window.count == 0 {
        return None;
    }
    Some(window.sum / (window.count as i128))
}

/// Compute stats for a grant category.
pub fn category_stats(env: &Env, category_id: u32) -> CategoryStats {
    let tags = Storage::get_category_list(env);
    let mut total_grants = 0u32;
    let mut completed_grants = 0u32;
    let mut total_funded = 0i128;
    let mut total_completion_ledgers = 0u64;
    let mut completion_count = 0u32;

    // Iterate through category index to find grants in this category
    let grant_ids = Storage::get_category_index(env, category_id);

    for grant_id in grant_ids.iter() {
        if let Some(grant) = Storage::get_grant(env, grant_id) {
            total_grants += 1;
            total_funded = total_funded.saturating_add(grant.escrow_balance);

            if grant.status as u32 == 3 {
                // Completed
                completed_grants += 1;
                // Estimate completion time (simplified)
                completion_count += 1;
            }
        }
    }

    let avg_completion_ledgers = if completion_count > 0 {
        (total_completion_ledgers / (completion_count as u64)) as u32
    } else {
        0
    };

    let success_rate_bps = (completed_grants * constants::BASIS_POINTS_SCALE)
        .checked_div(total_grants)
        .unwrap_or(0);

    CategoryStats {
        category_id,
        total_grants,
        completed_grants,
        total_funded,
        avg_completion_ledgers,
        success_rate_bps,
    }
}

/// Build and cache the full analytics snapshot.
pub fn build_snapshot(env: &Env) -> AnalyticsSnapshot {
    let milestone_avg =
        rolling_average(env, Symbol::new(env, "milestone_completion_time")).unwrap_or(0);
    let reviewer_avg = rolling_average(env, Symbol::new(env, "reviewer_turnaround")).unwrap_or(0);
    let success_window = get_window(env, Symbol::new(env, "grant_success"));

    let overall_success_rate_bps = if let Some(window) = success_window {
        if window.count > 0 {
            ((window.sum * constants::BASIS_POINTS_SCALE as i128) / (window.count as i128)) as u32
        } else {
            0
        }
    } else {
        0
    };

    // Find top category by total funded
    let categories = Storage::get_category_list(env);
    let mut top_category_id = None;
    let mut max_funded = 0i128;

    for cat in categories.iter() {
        let stats = category_stats(env, cat.id);
        if stats.total_funded > max_funded {
            max_funded = stats.total_funded;
            top_category_id = Some(cat.id);
        }
    }

    // Calculate TVL 7-day growth
    let tvl_window = get_window(env, Symbol::new(env, "tvl"));
    let tvl_7day_growth_bps = if let Some(window) = tvl_window {
        if window.window_size >= 7 {
            let current_tvl = window.values.get(window.window_size - 1).unwrap();
            let tvl_7days_ago = window
                .values
                .get(window.window_size.saturating_sub(7))
                .unwrap();
            if tvl_7days_ago > 0 {
                ((current_tvl - tvl_7days_ago) * constants::BASIS_POINTS_SCALE as i128)
                    / tvl_7days_ago
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    let snapshot = AnalyticsSnapshot {
        avg_milestone_comp_ledgers: milestone_avg as u32,
        avg_reviewer_turn_ledgers: reviewer_avg as u32,
        overall_success_rate_bps,
        top_category_id,
        tvl_7day_growth_bps,
        snapshot_at: env.ledger().timestamp(),
        captured_at_ledger: env.ledger().sequence(),
    };

    Storage::set_analytics_snapshot(env, &snapshot);
    snapshot
}

/// Return the latest cached snapshot.
pub fn get_snapshot(env: &Env) -> Option<AnalyticsSnapshot> {
    let snapshot = Storage::get_analytics_snapshot(env)?;

    // Check staleness (#695). Previously this compared `current_ledger`
    // against a *fresh* `env.ledger().sequence()` call, so the difference
    // was always 0 and the cached snapshot was effectively immortal.
    // The captured ledger is now stored on the snapshot itself.
    let current_ledger = env.ledger().sequence();
    let snapshot_ledger = snapshot.captured_at_ledger;

    if current_ledger.saturating_sub(snapshot_ledger)
        >= constants::ANALYTICS_SNAPSHOT_STALENESS_LEDGERS
    {
        // Stale, rebuild
        return Some(build_snapshot(env));
    }

    Some(snapshot)
}

/// Return the raw rolling window for a metric.
pub fn get_window(env: &Env, metric: Symbol) -> Option<RollingWindow> {
    Storage::get_rolling_window(env, &metric)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantStatus};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, String};

    fn setup_grant(env: &Env, id: u64, owner: &Address, status: GrantStatus, escrow_balance: i128) {
        let grant = Grant {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "Grant"),
            description: String::from_str(env, "Desc"),
            token: Address::generate(env),
            status,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 2,
            milestones_paid_out: 0,
            escrow_balance,
            funders: Vec::new(env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        };
        Storage::set_grant(env, id, &grant);
    }

    /// #876: category_stats must read the per-category index populated by
    /// grant_tags::tag_grant, not the freeform-tag hash index (whose keys
    /// are 32-bit hashes that a small sequential category_id will never
    /// match, silently leaving every category's stats at zero).
    #[test]
    fn test_category_stats_reads_tagged_grants() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let owner = Address::generate(&env);

        let cat_id = crate::grant_tags::create_category(
            &env,
            &admin,
            String::from_str(&env, "Infrastructure"),
            Vec::new(&env),
        )
        .unwrap();

        setup_grant(&env, 1, &owner, GrantStatus::Active, 1000);
        setup_grant(&env, 2, &owner, GrantStatus::Completed, 2000);

        let no_tags = Vec::new(&env);
        crate::grant_tags::tag_grant(&env, &owner, 1, Some(cat_id), None, no_tags.clone()).unwrap();
        crate::grant_tags::tag_grant(&env, &owner, 2, Some(cat_id), None, no_tags).unwrap();

        let stats = category_stats(&env, cat_id);
        assert_eq!(stats.total_grants, 2);
        assert_eq!(stats.completed_grants, 1);
        assert_eq!(stats.total_funded, 3000);
        assert_eq!(stats.success_rate_bps, constants::BASIS_POINTS_SCALE / 2);
    }

    /// #695: an old cached snapshot must trigger a rebuild once the ledger
    /// advances past the staleness threshold. The previous implementation
    /// compared against a self-referential `env.ledger().sequence()` call
    /// (always 0), so the cached snapshot was effectively immortal.
    #[test]
    fn test_stale_snapshot_triggers_rebuild() {
        let env = Env::default();

        // Seed a window so build_snapshot has something to compute against.
        record(&env, Symbol::new(&env, "milestone_completion_time"), 100);
        record(&env, Symbol::new(&env, "reviewer_turnaround"), 50);
        record(&env, Symbol::new(&env, "grant_success"), 1);

        // Build at ledger 10.
        let original_ledger = env.ledger().sequence();
        let snap1 = build_snapshot(&env);
        assert_eq!(snap1.captured_at_ledger, original_ledger);

        // Advance the ledger past ANALYTICS_SNAPSHOT_STALENESS_LEDGERS (1000).
        env.ledger().with_mut(|li| {
            li.sequence_number += constants::ANALYTICS_SNAPSHOT_STALENESS_LEDGERS + 1
        });

        // Re-build — captured_at_ledger must follow the new ledger.
        let snap2 = build_snapshot(&env);
        assert_eq!(
            snap2.captured_at_ledger,
            original_ledger + constants::ANALYTICS_SNAPSHOT_STALENESS_LEDGERS + 1
        );
        assert!(
            snap2.captured_at_ledger > snap1.captured_at_ledger,
            "rebuilt snapshot must reflect the new ledger sequence"
        );

        // And the get_snapshot path must notice the staleness.
        let fetched = get_snapshot(&env).expect("snapshot is cached");
        assert_eq!(fetched.captured_at_ledger, snap2.captured_at_ledger);
        assert!(
            fetched.snapshot_at >= snap1.snapshot_at,
            "snapshot_at timestamp monotonic"
        );
    }
}
