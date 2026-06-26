use soroban_sdk::{Address, Env, String, Vec};

use crate::constants::BASIS_POINTS_SCALE;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{ScoreResult, ScoringDimension, ScoringRubric, ScoringWeight};

fn require_global_admin(env: &Env, admin: &Address) -> Result<(), ContractError> {
    let global_admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if global_admin != *admin {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

pub fn validate_rubric(weights: &Vec<ScoringWeight>) -> Result<(), ContractError> {
    if weights.is_empty() {
        return Err(ContractError::InvalidWeights);
    }
    let mut sum: u32 = 0;
    for w in weights.iter() {
        sum = sum
            .checked_add(w.weight_bps)
            .ok_or(ContractError::InvalidWeights)?;
    }
    if sum != BASIS_POINTS_SCALE {
        return Err(ContractError::InvalidWeights);
    }
    Ok(())
}

pub fn define_rubric(
    env: &Env,
    admin: &Address,
    name: String,
    weights: Vec<ScoringWeight>,
) -> Result<u32, ContractError> {
    admin.require_auth();
    require_global_admin(env, admin)?;
    validate_rubric(&weights)?;

    let id = Storage::next_rubric_id(env);
    let rubric = ScoringRubric {
        id,
        name,
        weights,
        created_by: admin.clone(),
        created_at: env.ledger().timestamp(),
    };
    Storage::set_scoring_rubric(env, &rubric);
    Ok(id)
}

pub fn get_rubric(env: &Env, rubric_id: u32) -> Result<ScoringRubric, ContractError> {
    Storage::get_scoring_rubric(env, rubric_id).ok_or(ContractError::RubricNotFound)
}

pub fn list_rubrics(env: &Env) -> Vec<u32> {
    let mut ids: Vec<u32> = Vec::new(env);
    let mut i = 1;
    loop {
        if Storage::get_scoring_rubric(env, i).is_some() {
            ids.push_back(i);
            i += 1;
        } else if i > 100 {
            break;
        } else {
            i += 1;
        }
    }
    ids
}

fn compute_dimension_score(env: &Env, contributor: &Address, dimension: &ScoringDimension) -> u32 {
    match dimension {
        ScoringDimension::DeliverySpeed => {
            let profile = Storage::get_contributor(env, contributor.clone());
            match profile {
                Some(p) => {
                    let total = p.milestones_completed + p.milestones_rejected;
                    if total == 0 {
                        return 500;
                    }
                    let rate = p
                        .milestones_completed
                        .saturating_mul(1000)
                        .checked_div(total)
                        .unwrap_or(0);
                    rate.min(1000)
                }
                None => 0,
            }
        }
        ScoringDimension::ApprovalRate => {
            let profile = Storage::get_contributor(env, contributor.clone());
            match profile {
                Some(p) => {
                    let total = p.milestones_completed + p.milestones_rejected;
                    if total == 0 {
                        return 500;
                    }
                    let rate = p
                        .milestones_completed
                        .saturating_mul(1000)
                        .checked_div(total)
                        .unwrap_or(0);
                    rate.min(1000)
                }
                None => 0,
            }
        }
        ScoringDimension::ReputationScore => {
            let profile = Storage::get_contributor(env, contributor.clone());
            match profile {
                Some(p) => {
                    let score = p.reputation_score.min(1000) as u32;
                    score.min(1000)
                }
                None => 0,
            }
        }
        ScoringDimension::TotalEarned => {
            let profile = Storage::get_contributor(env, contributor.clone());
            match profile {
                Some(p) => {
                    let normalized = (p.total_earned as u64).min(1_000_000_000_000) as u32;
                    (normalized / 1_000_000_000).min(1000)
                }
                None => 0,
            }
        }
        ScoringDimension::DisputeRate => {
            let profile = Storage::get_contributor(env, contributor.clone());
            match profile {
                Some(p) => {
                    let total = p.grants_count;
                    if total == 0 {
                        return 1000;
                    }
                    let disputes = p.milestones_rejected.min(1000);
                    let rate = 1000u32.saturating_sub(
                        disputes
                            .saturating_mul(1000)
                            .checked_div(total.max(1))
                            .unwrap_or(0),
                    );
                    rate.min(1000)
                }
                None => 1000,
            }
        }
        ScoringDimension::ReviewerSatisfaction => {
            let profile = Storage::get_contributor(env, contributor.clone());
            match profile {
                Some(p) => {
                    let total = p.milestones_completed + p.milestones_rejected;
                    if total == 0 {
                        return 500;
                    }
                    let satisfaction = p
                        .milestones_completed
                        .saturating_mul(1000)
                        .checked_div(total)
                        .unwrap_or(0);
                    satisfaction.min(1000)
                }
                None => 500,
            }
        }
    }
}

pub fn score_contributor(
    env: &Env,
    contributor: &Address,
    rubric_id: u32,
) -> Result<ScoreResult, ContractError> {
    let rubric = get_rubric(env, rubric_id)?;

    let mut dimension_scores: Vec<(ScoringDimension, u32)> = Vec::new(env);
    let mut total_score: u32 = 0;

    for w in rubric.weights.iter() {
        let raw = compute_dimension_score(env, contributor, &w.dimension);
        let weighted = raw
            .saturating_mul(w.weight_bps)
            .checked_div(BASIS_POINTS_SCALE)
            .unwrap_or(0);
        let score = if w.invert {
            1000u32.saturating_sub(weighted.min(1000))
        } else {
            weighted.min(1000)
        };
        total_score = total_score.saturating_add(score);
        dimension_scores.push_back((w.dimension.clone(), score));
    }

    total_score = total_score.min(1000);

    let result = ScoreResult {
        subject: contributor.clone(),
        rubric_id,
        total_score,
        dimension_scores,
        computed_at: env.ledger().timestamp(),
    };

    Ok(result)
}

fn insertion_sort(results: &mut Vec<ScoreResult>) {
    let n = results.len();
    for i in 1..n {
        let mut j = i;
        while j > 0 && results.get(j - 1).unwrap().total_score < results.get(j).unwrap().total_score
        {
            let tmp = results.get(j).unwrap();
            let prev = results.get(j - 1).unwrap();
            let tmp_clone = tmp.clone();
            let prev_clone = prev.clone();
            results.set(j, prev_clone);
            results.set(j - 1, tmp_clone);
            j -= 1;
        }
    }
}

pub fn rank_contributors(
    env: &Env,
    contributors: Vec<Address>,
    rubric_id: u32,
) -> Vec<ScoreResult> {
    let mut results: Vec<ScoreResult> = Vec::new(env);
    for c in contributors.iter() {
        if let Ok(s) = score_contributor(env, &c, rubric_id) {
            results.push_back(s);
        }
    }
    insertion_sort(&mut results);
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    fn setup_admin(env: &Env, contract_id: &soroban_sdk::Address) -> Address {
        let admin = Address::generate(env);
        env.as_contract(contract_id, || {
            Storage::set_global_admin(env, &admin);
        });
        admin
    }

    fn make_contributor(env: &Env, contract_id: &soroban_sdk::Address) -> Address {
        let c = Address::generate(env);
        let profile = crate::types::ContributorProfile {
            contributor: c.clone(),
            name: soroban_sdk::String::from_str(env, "test"),
            bio: soroban_sdk::String::from_str(env, "bio"),
            skills: soroban_sdk::Vec::new(env),
            github_url: soroban_sdk::String::from_str(env, "https://github.com"),
            registration_timestamp: env.ledger().timestamp(),
            reputation_score: 750,
            grants_count: 5,
            total_earned: 500_000_000_000,
            milestones_completed: 8,
            milestones_rejected: 2,
            last_action_at: env.ledger().timestamp(),
        };
        env.as_contract(contract_id, || {
            Storage::set_contributor(env, c.clone(), &profile);
        });
        c
    }

    #[test]
    fn test_define_and_get_rubric() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let admin = setup_admin(&env, &contract_id);

        let mut weights: Vec<ScoringWeight> = Vec::new(&env);
        weights.push_back(ScoringWeight {
            dimension: ScoringDimension::ReputationScore,
            weight_bps: 5000,
            invert: false,
        });
        weights.push_back(ScoringWeight {
            dimension: ScoringDimension::ApprovalRate,
            weight_bps: 5000,
            invert: false,
        });

        let name = String::from_str(&env, "Standard");
        let id = env.as_contract(&contract_id, || {
            define_rubric(&env, &admin, name.clone(), weights.clone()).unwrap()
        });
        assert_eq!(id, 1);

        let rubric = env.as_contract(&contract_id, || get_rubric(&env, id).unwrap());
        assert_eq!(rubric.name, name);
        assert_eq!(rubric.weights.len(), 2);
    }

    #[test]
    fn test_invalid_weights_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let admin = setup_admin(&env, &contract_id);

        let mut weights: Vec<ScoringWeight> = Vec::new(&env);
        weights.push_back(ScoringWeight {
            dimension: ScoringDimension::ReputationScore,
            weight_bps: 3000,
            invert: false,
        });

        let name = String::from_str(&env, "Bad");
        let result = env.as_contract(&contract_id, || define_rubric(&env, &admin, name, weights));
        assert_eq!(result, Err(ContractError::InvalidWeights));
    }

    #[test]
    fn test_empty_weights_rejected() {
        let env = Env::default();
        let weights: Vec<ScoringWeight> = Vec::new(&env);
        assert_eq!(
            validate_rubric(&weights),
            Err(ContractError::InvalidWeights)
        );
    }

    #[test]
    fn test_score_contributor() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let admin = setup_admin(&env, &contract_id);
        let contributor = make_contributor(&env, &contract_id);

        let mut weights: Vec<ScoringWeight> = Vec::new(&env);
        weights.push_back(ScoringWeight {
            dimension: ScoringDimension::ReputationScore,
            weight_bps: 10000,
            invert: false,
        });

        let name = String::from_str(&env, "RepOnly");
        let id = env.as_contract(&contract_id, || {
            define_rubric(&env, &admin, name, weights).unwrap()
        });

        let result = env.as_contract(&contract_id, || {
            score_contributor(&env, &contributor, id).unwrap()
        });
        assert_eq!(result.subject, contributor);
        assert!(result.total_score > 0);
    }

    #[test]
    fn test_rank_contributors() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let admin = setup_admin(&env, &contract_id);

        let c1 = make_contributor(&env, &contract_id);

        let c2 = Address::generate(&env);
        let profile2 = crate::types::ContributorProfile {
            contributor: c2.clone(),
            name: soroban_sdk::String::from_str(&env, "c2"),
            bio: soroban_sdk::String::from_str(&env, "bio"),
            skills: soroban_sdk::Vec::new(&env),
            github_url: soroban_sdk::String::from_str(&env, "url"),
            registration_timestamp: env.ledger().timestamp(),
            reputation_score: 200,
            grants_count: 2,
            total_earned: 100_000_000_000,
            milestones_completed: 3,
            milestones_rejected: 1,
            last_action_at: env.ledger().timestamp(),
        };
        env.as_contract(&contract_id, || {
            Storage::set_contributor(&env, c2.clone(), &profile2);
        });

        let mut weights: Vec<ScoringWeight> = Vec::new(&env);
        weights.push_back(ScoringWeight {
            dimension: ScoringDimension::ReputationScore,
            weight_bps: 10000,
            invert: false,
        });

        let name = String::from_str(&env, "Rank");
        let id = env.as_contract(&contract_id, || {
            define_rubric(&env, &admin, name, weights).unwrap()
        });

        let mut contributors: Vec<Address> = Vec::new(&env);
        contributors.push_back(c2.clone());
        contributors.push_back(c1.clone());

        let ranked = env.as_contract(&contract_id, || rank_contributors(&env, contributors, id));
        assert_eq!(ranked.len(), 2);
        assert!(ranked.get(0).unwrap().total_score >= ranked.get(1).unwrap().total_score);
    }
}
