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
pub fn check_conditions(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<ConditionResult> {
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
pub fn get_conditions(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<ReleaseCondition> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{Grant, GrantStatus, MilestoneState, ReleaseCondition};
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env, Map, String, Vec};

    fn make_grant(env: &Env, owner: &Address) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: String::from_str(env, "T"),
            description: String::from_str(env, "D"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1_000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 2,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        }
    }

    fn seed_milestone(env: &Env, grant_id: u64, idx: u32) {
        Storage::set_milestone(
            env,
            grant_id,
            idx,
            &crate::types::Milestone {
                idx,
                description: String::from_str(env, "M"),
                amount: 500,
                state: MilestoneState::Submitted,
                votes: Map::new(env),
                approvals: 0,
                rejections: 0,
                reasons: Map::new(env),
                status_updated_at: 0,
                proof_url: None,
                submission_timestamp: 0,
                deadline: None,
                reviewer_count_snapshot: 0,
            },
        );
    }

    fn cond(env: &Env, condition_type: ConditionType, threshold: i128) -> ReleaseCondition {
        ReleaseCondition {
            condition_type,
            threshold,
            oracle_token: None,
            custom_contract: None,
            custom_fn_name: None,
            description: String::from_str(env, ""),
        }
    }

    // ── attach_conditions ──────────────────────────────────────────────────

    #[test]
    fn attach_conditions_stores_and_reads_back() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));

            let mut conditions = Vec::new(&env);
            conditions.push_back(cond(&env, ConditionType::AlwaysTrue, 0));
            conditions.push_back(cond(&env, ConditionType::TimestampAfter, 100));

            attach_conditions(&env, &owner, 1, 0, conditions).unwrap();
            assert_eq!(get_conditions(&env, 1, 0).len(), 2);
        });
    }

    #[test]
    fn attach_conditions_rejects_non_owner() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));
            assert_eq!(
                attach_conditions(&env, &stranger, 1, 0, Vec::new(&env)),
                Err(ContractError::Unauthorized)
            );
        });
    }

    #[test]
    fn attach_conditions_rejects_out_of_bounds_milestone() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));
            assert_eq!(
                attach_conditions(&env, &owner, 1, 5, Vec::new(&env)),
                Err(ContractError::MilestoneIndexOutOfBounds)
            );
        });
    }

    #[test]
    fn attach_conditions_rejects_more_than_max() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));
            let mut conditions = Vec::new(&env);
            for _ in 0..(MAX_CONDITIONS_PER_MILESTONE + 1) {
                conditions.push_back(cond(&env, ConditionType::AlwaysTrue, 0));
            }
            assert_eq!(
                attach_conditions(&env, &owner, 1, 0, conditions),
                Err(ContractError::MaxConditionsExceeded)
            );
        });
    }

    // ── check_conditions / all_conditions_met ──────────────────────────────

    #[test]
    fn no_conditions_reads_as_empty_and_met() {
        let env = Env::default();
        let contract_id = env.register(StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            assert_eq!(get_conditions(&env, 1, 0).len(), 0);
            assert_eq!(check_conditions(&env, 1, 0).len(), 0);
            // Vacuously true when nothing is attached.
            assert!(all_conditions_met(&env, 1, 0));
        });
    }

    #[test]
    fn always_true_condition_is_met() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));
            seed_milestone(&env, 1, 0);
            let mut conditions = Vec::new(&env);
            conditions.push_back(cond(&env, ConditionType::AlwaysTrue, 0));
            attach_conditions(&env, &owner, 1, 0, conditions).unwrap();

            let results = check_conditions(&env, 1, 0);
            assert_eq!(results.len(), 1);
            assert!(results.get(0).unwrap().met);
            assert!(all_conditions_met(&env, 1, 0));
        });
    }

    #[test]
    fn timestamp_condition_flips_from_unmet_to_met() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(100);
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));
            let mut conditions = Vec::new(&env);
            conditions.push_back(cond(&env, ConditionType::TimestampAfter, 1_000));
            attach_conditions(&env, &owner, 1, 0, conditions).unwrap();

            // now = 100, threshold = 1000 -> not met.
            assert!(!all_conditions_met(&env, 1, 0));
        });

        env.ledger().set_timestamp(2_000);
        env.as_contract(&contract_id, || {
            // now = 2000, threshold = 1000 -> met.
            assert!(all_conditions_met(&env, 1, 0));
        });
    }

    #[test]
    fn ledger_sequence_condition_is_evaluated() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(10);
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));
            let mut conditions = Vec::new(&env);
            conditions.push_back(cond(&env, ConditionType::LedgerSequenceAfter, 100));
            attach_conditions(&env, &owner, 1, 0, conditions).unwrap();
            assert!(!all_conditions_met(&env, 1, 0));
        });

        env.ledger().set_sequence_number(200);
        env.as_contract(&contract_id, || {
            assert!(all_conditions_met(&env, 1, 0));
        });
    }

    #[test]
    fn all_conditions_met_is_false_when_any_condition_unmet() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(100);
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner));
            let mut conditions = Vec::new(&env);
            conditions.push_back(cond(&env, ConditionType::AlwaysTrue, 0));
            conditions.push_back(cond(&env, ConditionType::TimestampAfter, 1_000_000));
            attach_conditions(&env, &owner, 1, 0, conditions).unwrap();

            let results = check_conditions(&env, 1, 0);
            assert_eq!(results.len(), 2);
            assert!(results.get(0).unwrap().met);
            assert!(!results.get(1).unwrap().met);
            assert!(!all_conditions_met(&env, 1, 0));
        });
    }
}
