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

    // Iterate through tag index to find grants in this category
    let grant_ids = Storage::get_tag_index(env, category_id);

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

    let success_rate_bps = if total_grants > 0 {
        (completed_grants * constants::BASIS_POINTS_SCALE) / total_grants
    } else {
        0
    };

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
    };

    Storage::set_analytics_snapshot(env, &snapshot);
    snapshot
}

/// Return the latest cached snapshot.
pub fn get_snapshot(env: &Env) -> Option<AnalyticsSnapshot> {
    let snapshot = Storage::get_analytics_snapshot(env)?;

    // Check staleness
    let current_ledger = env.ledger().sequence();
    let snapshot_ledger = env.ledger().sequence(); // Simplified - in real impl track ledger

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
