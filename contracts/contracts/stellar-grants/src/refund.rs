use soroban_sdk::{contracttype, token, Address, Env};

use crate::types::{ContractError, RefundCalculation, RefundPolicy, RefundPolicyType};
use crate::Storage;

const BPS: i128 = 10_000;

#[contracttype]
pub enum RefundKey {
    Policy(u64),
}

pub fn set_policy(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    policy: RefundPolicy,
) -> Result<(), ContractError> {
    owner.require_auth();
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner || policy.grant_id != grant_id || grant.escrow_balance > 0 {
        return Err(ContractError::InvalidInput);
    }
    env.storage()
        .persistent()
        .set(&RefundKey::Policy(grant_id), &policy);
    Ok(())
}

/// Whether an owner has explicitly configured a refund policy for this grant,
/// as opposed to `get_policy`'s fabricated default (FullRefund) when nothing
/// has been stored.
pub fn has_policy(env: &Env, grant_id: u64) -> bool {
    env.storage().persistent().has(&RefundKey::Policy(grant_id))
}

pub fn get_policy(env: &Env, grant_id: u64) -> RefundPolicy {
    env.storage()
        .persistent()
        .get(&RefundKey::Policy(grant_id))
        .unwrap_or(RefundPolicy {
            grant_id,
            policy_type: RefundPolicyType::FullRefund,
            penalty_bps: 0,
            grace_period_ledgers: 0,
            min_refund_pct_bps: 0,
        })
}

pub fn calculate_refund(
    env: &Env,
    grant_id: u64,
    _canceller: &Address,
) -> Result<RefundCalculation, ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let policy = get_policy(env, grant_id);
    let gross = grant.escrow_balance;
    let now = env.ledger().timestamp();
    let start = grant.timestamp;
    let mut applied = policy.policy_type.clone();
    let raw = if policy.grace_period_ledgers > 0
        && now < start.saturating_add(policy.grace_period_ledgers as u64)
    {
        applied = RefundPolicyType::FullRefund;
        gross
    } else {
        match policy.policy_type {
            RefundPolicyType::FullRefund => gross,
            RefundPolicyType::ProportionalToRemaining => {
                if grant.total_milestones == 0 {
                    0
                } else {
                    gross
                        .saturating_mul(
                            grant
                                .total_milestones
                                .saturating_sub(grant.milestones_paid_out)
                                as i128,
                        )
                        .checked_div(grant.total_milestones as i128)
                        .unwrap_or(0)
                }
            }
            RefundPolicyType::TimeWeighted => {
                // Derive an end time from the grant's real timeline: use the latest
                // milestone deadline if present, otherwise fall back to a sensible
                // default duration from grant start.
                let mut max_deadline: Option<u64> = None;
                for i in 0..grant.total_milestones {
                    if let Some(ms) = Storage::get_milestone(env, grant_id, i) {
                        if let Some(d) = ms.deadline {
                            max_deadline = Some(match max_deadline {
                                Some(prev) => prev.max(d),
                                None => d,
                            });
                        }
                    }
                }
                let end_time = max_deadline.unwrap_or_else(|| {
                    start.saturating_add(crate::constants::DEFAULT_TIME_WEIGHTED_DURATION_SECONDS)
                });
                time_weighted_refund(gross, start, end_time, now)
            }
            RefundPolicyType::PenaltyOnCancel => gross.saturating_sub(
                gross
                    .saturating_mul(policy.penalty_bps as i128)
                    .checked_div(BPS)
                    .unwrap_or(0),
            ),
            _ => 0,
        }
    };
    let floor = gross
        .saturating_mul(policy.min_refund_pct_bps as i128)
        .checked_div(BPS)
        .unwrap_or(0);
    let funder_refund = raw.max(floor).min(gross);
    let contributor_compensation = gross.saturating_sub(funder_refund);
    Ok(RefundCalculation {
        gross_escrow: gross,
        funder_refund,
        contributor_compensation,
        penalty_amount: contributor_compensation,
        policy_applied: applied,
    })
}

pub fn execute_refund(
    env: &Env,
    grant_id: u64,
    canceller: &Address,
) -> Result<RefundCalculation, ContractError> {
    crate::reentrancy::protect(env)?;
    let calc = calculate_refund(env, grant_id, canceller)?;
    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let client = token::Client::new(env, &grant.token);

    grant.escrow_balance = 0;
    Storage::set_grant(env, grant_id, &grant);

    if calc.contributor_compensation > 0 {
        crate::reentrancy::protect_external_call(env, || {
            client.transfer(
                &env.current_contract_address(),
                &grant.owner,
                &calc.contributor_compensation,
            );
            Ok(())
        })?;
    }
    if calc.funder_refund > 0 {
        let mut total: i128 = 0;
        for fund in grant.funders.iter() {
            total = total.saturating_add(fund.amount);
        }
        if total > 0 {
            let len = grant.funders.len();
            let mut paid: i128 = 0;
            for i in 0..len {
                let fund = grant.funders.get(i).ok_or(ContractError::InvalidInput)?;
                let amount = if i + 1 == len {
                    calc.funder_refund.saturating_sub(paid)
                } else {
                    fund.amount
                        .saturating_mul(calc.funder_refund)
                        .checked_div(total)
                        .unwrap_or(0)
                };
                if amount > 0 {
                    let funder = fund.funder.clone();
                    crate::reentrancy::protect_external_call(env, || {
                        client.transfer(&env.current_contract_address(), &funder, &amount);
                        Ok(())
                    })?;
                    paid = paid.saturating_add(amount);
                }
            }
        }
    }
    Ok(calc)
}

pub fn time_weighted_refund(
    gross: i128,
    start_time: u64,
    end_time: u64,
    current_time: u64,
) -> i128 {
    if gross <= 0 || end_time <= start_time || current_time >= end_time {
        return 0;
    }
    if current_time <= start_time {
        return gross;
    }
    gross
        .saturating_mul(end_time.saturating_sub(current_time) as i128)
        .checked_div(end_time.saturating_sub(start_time) as i128)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Env, String, Vec};

    #[test]
    fn time_weighted_uses_milestone_deadlines() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let grant_id = 7u64;
        let start = 1_000u64;

        // create grant with 3 milestones and an escrow balance
        let grant = crate::types::Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(&env, "G"),
            description: String::from_str(&env, "D"),
            token: Address::generate(&env),
            status: crate::types::GrantStatus::Active,
            total_amount: 0,
            milestone_amount: 0,
            reviewers: Vec::new(&env),
            total_milestones: 3,
            milestones_paid_out: 0,
            escrow_balance: 1_000,
            funders: Vec::new(&env),
            reason: None,
            timestamp: start,
            require_compliance: None,
        };
        Storage::set_grant(&env, grant_id, &grant);

        // set milestone deadlines spaced by a week
        for i in 0..3u32 {
            let ms = crate::types::Milestone {
                idx: i,
                description: String::from_str(&env, "m"),
                amount: 0,
                state: crate::types::MilestoneState::Pending,
                votes: soroban_sdk::Map::new(&env),
                approvals: 0,
                rejections: 0,
                reasons: soroban_sdk::Map::new(&env),
                status_updated_at: 0,
                proof_url: None,
                submission_timestamp: 0,
                deadline: Some(
                    start.saturating_add(crate::constants::SECONDS_PER_WEEK * (i as u64 + 1)),
                ),
                reviewer_count_snapshot: 0,
            };
            Storage::set_milestone(&env, grant_id, i, &ms);
        }

        // set a TimeWeighted policy directly into storage
        let policy = RefundPolicy {
            grant_id,
            policy_type: RefundPolicyType::TimeWeighted,
            penalty_bps: 0,
            grace_period_ledgers: 0,
            min_refund_pct_bps: 0,
        };
        env.storage()
            .persistent()
            .set(&RefundKey::Policy(grant_id), &policy);

        // advance ledger to one week after start (should still be before latest deadline)
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: start + crate::constants::SECONDS_PER_WEEK,
            protocol_version: 21,
            sequence_number: 1,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 100_000,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 1_000_000,
        });

        let calc = calculate_refund(&env, grant_id, &owner).unwrap();
        // Because the latest milestone deadline is in the future, funder_refund should be > 0
        assert!(calc.funder_refund > 0);
    }
}
