use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ConditionResult, ConditionType, ReleaseCondition};

const MAX_CONDITIONS_PER_MILESTONE: u32 = 5;

/// Attach release conditions to a milestone. Owner only, before submission.
pub fn attach_conditions(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    milestone_idx: u32,
    conditions: Vec<ReleaseCondition>,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    if conditions.len() > MAX_CONDITIONS_PER_MILESTONE {
        return Err(ContractError::MaxConditionsExceeded);
    }

    Storage::set_release_conditions(env, grant_id, milestone_idx, &conditions);
    Ok(())
}

/// Check all conditions for a milestone. Returns detailed results per condition.
pub fn check_conditions(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
) -> Vec<ConditionResult> {
    let conditions = Storage::get_release_conditions(env, grant_id, milestone_idx);
    let mut results = Vec::new(env);

    for (idx, condition) in conditions.iter().enumerate() {
        let (met, current_value) = evaluate_condition(env, &condition);
        results.push_back(ConditionResult {
            condition_idx: idx as u32,
            met,
            current_value,
            threshold: condition.threshold,
            checked_at: env.ledger().timestamp(),
        });
    }

    results
}

/// Return true only if every condition is met.
pub fn all_conditions_met(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    let results = check_conditions(env, grant_id, milestone_idx);
    for result in results.iter() {
        if !result.met {
            return false;
        }
    }
    true
}

/// Return the conditions attached to a milestone.
pub fn get_conditions(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
) -> Vec<ReleaseCondition> {
    Storage::get_release_conditions(env, grant_id, milestone_idx)
}

fn evaluate_condition(env: &Env, condition: &ReleaseCondition) -> (bool, i128) {
    match condition.condition_type {
        ConditionType::LedgerSequenceAfter => {
            let current = env.ledger().sequence() as i128;
            (current >= condition.threshold, current)
        }
        ConditionType::TimestampAfter => {
            let current = env.ledger().timestamp() as i128;
            (current >= condition.threshold, current)
        }
        ConditionType::OraclePriceAbove => {
            if let Some(ref token) = condition.oracle_token {
                match crate::oracle::get_price(env, token) {
                    Ok(quote) => {
                        let met = quote.price_in_base >= condition.threshold;
                        (met, quote.price_in_base)
                    }
                    Err(_) => (false, 0),
                }
            } else {
                (false, 0)
            }
        }
        ConditionType::OraclePriceBelow => {
            if let Some(ref token) = condition.oracle_token {
                match crate::oracle::get_price(env, token) {
                    Ok(quote) => {
                        let met = quote.price_in_base <= condition.threshold;
                        (met, quote.price_in_base)
                    }
                    Err(_) => (false, 0),
                }
            } else {
                (false, 0)
            }
        }
        ConditionType::CustomContractCall => {
            if let (Some(ref contract), Some(fn_name)) =
                (&condition.custom_contract, &condition.custom_fn_name)
            {
                let args: soroban_sdk::Vec<soroban_sdk::Val> = soroban_sdk::Vec::new(env);
                let result: Option<i128> = env.invoke_contract(contract, fn_name, args);
                match result {
                    Some(val) => (val != 0, val),
                    None => (false, 0),
                }
            } else {
                (false, 0)
            }
        }
        ConditionType::AlwaysTrue => (true, 1),
    }
}
