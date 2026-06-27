use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::escrow;
use crate::storage::Storage;
use crate::types::{FunderGrantSummary, FunderReport, FunderTokenSummary, GrantStatus};

/// Build a comprehensive financial report for a funder. Read-only.
pub fn get_report(env: &Env, funder: &Address) -> Result<FunderReport, ContractError> {
    let grants = grant_summaries(env, funder, 0, 50)?;
    let token_sums = build_token_summaries(env, funder, &grants);

    let mut active: u32 = 0;
    let mut completed: u32 = 0;
    let mut matching_total: i128 = 0;
    let mut premiums_total: i128 = 0;

    for g in grants.iter() {
        match g.grant_status {
            GrantStatus::Active => active += 1,
            GrantStatus::Completed => completed += 1,
            _ => {}
        }
    }

    // matching_contributions: try to read the matching module if present.
    matching_total = Storage::get_matching_contribution(env, funder).unwrap_or(0);

    // insurance_premiums_paid: iterate grants and sum from policy storage.
    for g in grants.iter() {
        if let Some(policy) = Storage::get_insurance_policy(env, g.grant_id) {
            if policy.policyholder == *funder {
                premiums_total = premiums_total
                    .checked_add(policy.premium_paid)
                    .unwrap_or(premiums_total);
            }
        }
    }

    Ok(FunderReport {
        funder: funder.clone(),
        report_at: env.ledger().timestamp(),
        total_grants_funded: grants.len() as u32,
        active_grants: active,
        completed_grants: completed,
        token_summaries: token_sums,
        grant_summaries: grants,
        matching_contributions: matching_total,
        insurance_premiums_paid: premiums_total,
    })
}

/// Return per-token financial summary for a funder.
pub fn token_summary(env: &Env, funder: &Address, token: &Address) -> FunderTokenSummary {
    let grants = grant_summaries(env, funder, 0, 50).unwrap_or_else(|_| Vec::new(env));
    let mut summary = FunderTokenSummary {
        token: token.clone(),
        total_committed: 0,
        total_paid_out: 0,
        total_refunded: 0,
        total_in_escrow: 0,
        total_yield_earned: 0,
        net_deployed: 0,
    };

    for g in grants.iter() {
        if g.token == *token {
            summary.total_committed = summary
                .total_committed
                .checked_add(g.funded_amount)
                .unwrap_or(summary.total_committed);
            summary.total_paid_out = summary
                .total_paid_out
                .checked_add(g.paid_out_amount)
                .unwrap_or(summary.total_paid_out);
            summary.total_refunded = summary
                .total_refunded
                .checked_add(g.refunded_amount)
                .unwrap_or(summary.total_refunded);
            summary.total_in_escrow = summary
                .total_in_escrow
                .checked_add(g.in_escrow)
                .unwrap_or(summary.total_in_escrow);
            if let Some(y) = g.yield_earned {
                summary.total_yield_earned = summary
                    .total_yield_earned
                    .checked_add(y)
                    .unwrap_or(summary.total_yield_earned);
            }
        }
    }

    summary.net_deployed = summary
        .total_committed
        .checked_sub(summary.total_refunded)
        .unwrap_or(0);

    summary
}

/// Return summaries for all grants funded by an address.
pub fn grant_summaries(
    env: &Env,
    funder: &Address,
    offset: u32,
    limit: u32,
) -> Result<Vec<FunderGrantSummary>, ContractError> {
    // First collect all escrow funders lists to find grants this funder participated in.
    // We iterate the funder's contributed grants by looking at escrow funder ledgers.
    let mut summaries: Vec<FunderGrantSummary> = Vec::new(env);

    // Strategy: iterate through known grants by checking escrow funder lists.
    // We need to be efficient; use the escrow module's funder ledger lookup.
    // For each grant_id where funder has a contribution, build a summary.

    // Since we don't have a direct reverse index, we use a pragmatic approach:
    // Look at each grant the funder contributed to by checking funder ledger.
    // This is a read-only query, so we accept O(num_grants) iteration.

    // Get the grant ids from the contributor's grant index as a starting point
    // if the funder is also a contributor, or iterate through all escrow accounts.

    // Simplest approach: iterate through escrow funders list for each grant.
    // But we don't have a global grant count easily. Let's use a reasonable range.

    // Better approach: use the FunderGrantIndex if available in storage,
    // or just scan through grant ids from 1 to a reasonable counter.
    let grant_count = Storage::get_grant_counter(env);
    let mut collected: u32 = 0;

    for id in 1..=grant_count {
        if collected >= offset + limit {
            break;
        }

        // Check if this funder has a ledger entry for this grant.
        let ledger = escrow::get_funder_ledger(env, id, funder);
        if ledger.is_none() {
            continue;
        }

        if collected < offset {
            collected += 1;
            continue;
        }

        let grant = match Storage::get_grant(env, id) {
            Some(g) => g,
            None => continue,
        };

        let ld = ledger.unwrap();
        let escrow_account = escrow::get_account(env, id).ok();

        let in_escrow = escrow_account
            .as_ref()
            .map(|a| {
                // Funder's proportional share of remaining escrow balance.
                if a.total_deposited > 0 {
                    let net = ld.contributed.saturating_sub(ld.refunded);
                    if net > 0 {
                        a.balance
                            .checked_mul(net)
                            .unwrap_or(0)
                            .checked_div(a.total_deposited)
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            })
            .unwrap_or(0);

        let paid_out = escrow_account
            .as_ref()
            .map(|a| {
                if a.total_deposited > 0 {
                    let net = ld.contributed.saturating_sub(ld.refunded);
                    if net > 0 {
                        a.total_released
                            .checked_mul(net)
                            .unwrap_or(0)
                            .checked_div(a.total_deposited)
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            })
            .unwrap_or(0);

        let summary = FunderGrantSummary {
            grant_id: id,
            grant_title: grant.title.clone(),
            token: grant.token.clone(),
            funded_amount: ld.contributed,
            paid_out_amount: paid_out,
            refunded_amount: ld.refunded,
            in_escrow,
            yield_earned: None, // No yield module integration yet
            funded_at: ld.last_contribution_at,
            grant_status: grant.status,
        };

        summaries.push_back(summary);
        collected += 1;
    }

    Ok(summaries)
}

/// Return total amount currently in escrow across all grants for a funder (per token).
pub fn total_in_escrow(env: &Env, funder: &Address, token: &Address) -> i128 {
    let ts = token_summary(env, funder, token);
    ts.total_in_escrow
}

/// Return a lightweight report suitable for a dashboard widget.
/// Returns: (grants_count, total_committed, total_in_escrow, total_paid_out)
pub fn dashboard_summary(env: &Env, funder: &Address) -> (u32, i128, i128, i128) {
    let grants = grant_summaries(env, funder, 0, 50).unwrap_or_else(|_| Vec::new(env));
    let count = grants.len() as u32;
    let mut committed: i128 = 0;
    let mut escrowed: i128 = 0;
    let mut paid: i128 = 0;

    for g in grants.iter() {
        committed = committed.saturating_add(g.funded_amount);
        escrowed = escrowed.saturating_add(g.in_escrow);
        paid = paid.saturating_add(g.paid_out_amount);
    }

    (count, committed, escrowed, paid)
}

// ── Private helpers ──────────────────────────────────────────────────────────

fn build_token_summaries(
    env: &Env,
    funder: &Address,
    grants: &Vec<FunderGrantSummary>,
) -> Vec<FunderTokenSummary> {
    use soroban_sdk::Map;

    let mut map: Map<Address, FunderTokenSummary> = Map::new(env);

    for g in grants.iter() {
        let token = g.token.clone();

        let summary = map.get(token.clone()).unwrap_or(FunderTokenSummary {
            token: token.clone(),
            total_committed: 0,
            total_paid_out: 0,
            total_refunded: 0,
            total_in_escrow: 0,
            total_yield_earned: 0,
            net_deployed: 0,
        });

        let mut s = summary;
        s.total_committed = s.total_committed.saturating_add(g.funded_amount);
        s.total_paid_out = s.total_paid_out.saturating_add(g.paid_out_amount);
        s.total_refunded = s.total_refunded.saturating_add(g.refunded_amount);
        s.total_in_escrow = s.total_in_escrow.saturating_add(g.in_escrow);
        if let Some(y) = g.yield_earned {
            s.total_yield_earned = s.total_yield_earned.saturating_add(y);
        }
        s.net_deployed = s.total_committed.saturating_sub(s.total_refunded);

        map.set(token, s);
    }

    let mut result: Vec<FunderTokenSummary> = Vec::new(env);
    for token in map.keys() {
        if let Some(s) = map.get(token) {
            result.push_back(s);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EscrowAccount, FunderLedger, Grant, GrantFund, GrantStatus};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Vec;

    #[test]
    fn test_unknown_funder_returns_empty_report() {
        let env = Env::default();
        let funder = Address::generate(&env);
        let report = get_report(&env, &funder).unwrap();
        assert_eq!(report.total_grants_funded, 0);
        assert_eq!(report.active_grants, 0);
        assert_eq!(report.completed_grants, 0);
        assert_eq!(report.grant_summaries.len(), 0);
    }

    #[test]
    fn test_dashboard_summary_empty() {
        let env = Env::default();
        let funder = Address::generate(&env);
        let (count, committed, escrowed, paid) = dashboard_summary(&env, &funder);
        assert_eq!(count, 0);
        assert_eq!(committed, 0);
        assert_eq!(escrowed, 0);
        assert_eq!(paid, 0);
    }

    #[test]
    fn test_token_summary_empty() {
        let env = Env::default();
        let funder = Address::generate(&env);
        let token = Address::generate(&env);
        let ts = token_summary(&env, &funder, &token);
        assert_eq!(ts.total_committed, 0);
        assert_eq!(ts.total_paid_out, 0);
    }
}
