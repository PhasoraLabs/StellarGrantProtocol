use soroban_sdk::{contractevent, Address, Env, String, Vec};

use crate::constants::MAX_CRITERIA_PER_MILESTONE;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{AcceptanceCriteria, ChecklistSubmission, CriterionStatus};

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChecklistSubmitted {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub submitted_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriterionReviewed {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub criterion_idx: u32,
    pub approved: bool,
}

pub fn define_criteria(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    milestone_idx: u32,
    criteria: Vec<AcceptanceCriteria>,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    if criteria.len() > MAX_CRITERIA_PER_MILESTONE {
        return Err(ContractError::MaxCriteriaExceeded);
    }

    Storage::set_milestone_checklist(env, grant_id, milestone_idx, &criteria);
    Ok(())
}

pub fn submit_checklist(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    milestone_idx: u32,
    evidence_urls: Vec<Option<String>>,
) -> Result<(), ContractError> {
    contributor.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }

    let criteria = Storage::get_milestone_checklist(env, grant_id, milestone_idx)
        .ok_or(ContractError::ChecklistNotFound)?;

    if evidence_urls.len() != criteria.len() {
        return Err(ContractError::InvalidInput);
    }

    if Storage::get_checklist_submission(env, grant_id, milestone_idx).is_some() {
        return Err(ContractError::ChecklistAlreadySubmitted);
    }

    let mut statuses: Vec<CriterionStatus> = Vec::new(env);
    for _ in 0..criteria.len() {
        statuses.push_back(CriterionStatus::CheckedByContributor);
    }

    let submission = ChecklistSubmission {
        grant_id,
        milestone_idx,
        criteria,
        statuses,
        evidence_urls,
        submitted_at: env.ledger().timestamp(),
        all_required_met: false,
    };

    Storage::set_checklist_submission(env, &submission);

    ChecklistSubmitted {
        grant_id,
        milestone_idx,
        submitted_at: submission.submitted_at,
    }
    .publish(env);

    Ok(())
}

pub fn review_criterion(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    milestone_idx: u32,
    criterion_idx: u32,
    approve: bool,
) -> Result<(), ContractError> {
    reviewer.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if !grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let mut submission = Storage::get_checklist_submission(env, grant_id, milestone_idx)
        .ok_or(ContractError::ChecklistNotFound)?;

    if criterion_idx >= submission.criteria.len() {
        return Err(ContractError::CriterionNotFound);
    }

    let new_status = if approve {
        CriterionStatus::ApprovedByReviewer
    } else {
        CriterionStatus::RejectedByReviewer
    };
    submission.statuses.set(criterion_idx, new_status);

    let mut all_required_met = true;
    for i in 0..submission.criteria.len() {
        let criterion = submission.criteria.get(i).unwrap();
        if criterion.is_required {
            let status = submission.statuses.get(i).unwrap();
            if status != CriterionStatus::ApprovedByReviewer {
                all_required_met = false;
                break;
            }
        }
    }
    submission.all_required_met = all_required_met;

    Storage::set_checklist_submission(env, &submission);

    CriterionReviewed {
        grant_id,
        milestone_idx,
        criterion_idx,
        approved: approve,
    }
    .publish(env);

    Ok(())
}

pub fn all_required_approved(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    Storage::get_checklist_submission(env, grant_id, milestone_idx)
        .map(|s| s.all_required_met)
        .unwrap_or(false)
}

pub fn get_checklist(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<ChecklistSubmission> {
    Storage::get_checklist_submission(env, grant_id, milestone_idx)
}

pub fn get_criterion_status(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    criterion_idx: u32,
) -> Option<CriterionStatus> {
    let submission = Storage::get_checklist_submission(env, grant_id, milestone_idx)?;
    submission.statuses.get(criterion_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    fn setup_grant(env: &Env, owner: &Address, reviewer: &Address) -> u64 {
        Storage::set_global_admin(env, owner);
        crate::escrow::open(env, 1, owner, &Address::generate(env)).unwrap();
        let reviewers: Vec<Address> = {
            let mut v = Vec::new(env);
            v.push_back(reviewer.clone());
            v
        };
        let grant = crate::types::Grant {
            id: 1,
            owner: owner.clone(),
            title: soroban_sdk::String::from_str(env, "test"),
            description: soroban_sdk::String::from_str(env, "desc"),
            token: Address::generate(env),
            status: crate::types::GrantStatus::Active,
            total_amount: 100_000,
            milestone_amount: 10_000,
            reviewers,
            total_milestones: 3,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: soroban_sdk::Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, 1, &grant);
        1
    }

    #[test]
    fn test_define_criteria() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, &reviewer);
        });

        let mut criteria: Vec<AcceptanceCriteria> = Vec::new(&env);
        criteria.push_back(AcceptanceCriteria {
            idx: 0,
            description: soroban_sdk::String::from_str(&env, "Code compiles"),
            is_required: true,
        });

        let result = env.as_contract(&contract_id, || {
            define_criteria(&env, &owner, 1, 0, criteria.clone())
        });
        assert!(result.is_ok());

        let stored = env.as_contract(&contract_id, || {
            Storage::get_milestone_checklist(&env, 1, 0).unwrap()
        });
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored.get(0).unwrap().description,
            soroban_sdk::String::from_str(&env, "Code compiles")
        );
    }

    #[test]
    fn test_submit_and_review_required_approved() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, &reviewer);
        });

        let mut criteria: Vec<AcceptanceCriteria> = Vec::new(&env);
        criteria.push_back(AcceptanceCriteria {
            idx: 0,
            description: soroban_sdk::String::from_str(&env, "Code compiles"),
            is_required: true,
        });
        env.as_contract(&contract_id, || {
            define_criteria(&env, &owner, 1, 0, criteria).unwrap();
        });

        let mut evidence: Vec<Option<String>> = Vec::new(&env);
        evidence.push_back(Some(soroban_sdk::String::from_str(
            &env,
            "https://example.com",
        )));
        env.as_contract(&contract_id, || {
            submit_checklist(&env, &owner, 1, 0, evidence).unwrap();
        });

        assert!(!env.as_contract(&contract_id, || all_required_approved(&env, 1, 0)));

        env.as_contract(&contract_id, || {
            review_criterion(&env, &reviewer, 1, 0, 0, true).unwrap();
        });
        assert!(env.as_contract(&contract_id, || all_required_approved(&env, 1, 0)));
    }

    #[test]
    fn test_required_rejected_blocks_approval() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, &reviewer);
        });

        let mut criteria: Vec<AcceptanceCriteria> = Vec::new(&env);
        criteria.push_back(AcceptanceCriteria {
            idx: 0,
            description: soroban_sdk::String::from_str(&env, "Must pass"),
            is_required: true,
        });
        env.as_contract(&contract_id, || {
            define_criteria(&env, &owner, 1, 0, criteria).unwrap();
        });

        let mut evidence: Vec<Option<String>> = Vec::new(&env);
        evidence.push_back(Some(soroban_sdk::String::from_str(&env, "proof")));
        env.as_contract(&contract_id, || {
            submit_checklist(&env, &owner, 1, 0, evidence).unwrap();
            review_criterion(&env, &reviewer, 1, 0, 0, false).unwrap();
        });

        assert!(!env.as_contract(&contract_id, || all_required_approved(&env, 1, 0)));
    }

    #[test]
    fn test_optional_rejected_does_not_block() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, &reviewer);
        });

        let mut criteria: Vec<AcceptanceCriteria> = Vec::new(&env);
        criteria.push_back(AcceptanceCriteria {
            idx: 0,
            description: soroban_sdk::String::from_str(&env, "Nice to have"),
            is_required: false,
        });
        env.as_contract(&contract_id, || {
            define_criteria(&env, &owner, 1, 0, criteria).unwrap();
        });

        let mut evidence: Vec<Option<String>> = Vec::new(&env);
        evidence.push_back(Some(soroban_sdk::String::from_str(&env, "proof")));
        env.as_contract(&contract_id, || {
            submit_checklist(&env, &owner, 1, 0, evidence).unwrap();
            review_criterion(&env, &reviewer, 1, 0, 0, false).unwrap();
        });

        assert!(env.as_contract(&contract_id, || all_required_approved(&env, 1, 0)));
    }

    #[test]
    fn test_max_criteria_limit() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, &reviewer);
        });

        let mut criteria: Vec<AcceptanceCriteria> = Vec::new(&env);
        for i in 0..MAX_CRITERIA_PER_MILESTONE + 1 {
            criteria.push_back(AcceptanceCriteria {
                idx: i,
                description: soroban_sdk::String::from_str(&env, "criterion"),
                is_required: false,
            });
        }
        let result = env.as_contract(&contract_id, || {
            define_criteria(&env, &owner, 1, 0, criteria)
        });
        assert_eq!(result, Err(ContractError::MaxCriteriaExceeded));
    }
}
