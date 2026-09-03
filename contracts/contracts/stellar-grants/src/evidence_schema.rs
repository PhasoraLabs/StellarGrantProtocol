use crate::storage::Storage;
use crate::types::{ContractError, EvidenceField, EvidenceSchema, StructuredEvidence};
use soroban_sdk::{Address, Env, Map, String, Vec};

/// Define the evidence schema for a milestone. Only the grant owner or global admin may call this.
/// Fields describe what structured evidence contributors must supply on milestone submission.
pub fn set_schema(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
    fields: Vec<EvidenceField>,
) -> Result<(), ContractError> {
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let is_admin = Storage::get_global_admin(env).map_or(false, |a| a == *caller);
    if grant.owner != *caller && !is_admin {
        return Err(ContractError::Unauthorized);
    }
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }
    if fields.is_empty() {
        return Err(ContractError::InvalidInput);
    }

    let schema = EvidenceSchema {
        grant_id,
        milestone_idx,
        fields,
    };
    Storage::set_evidence_schema(env, grant_id, milestone_idx, &schema);
    Ok(())
}

/// Submit structured evidence for a milestone. Must conform to the registered schema (if any).
/// This must be called before `milestone_submit` when a schema exists.
pub fn submit_evidence(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
    values: Map<String, String>,
) -> Result<(), ContractError> {
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    if let Some(schema) = Storage::get_evidence_schema(env, grant_id, milestone_idx) {
        for field in schema.fields.iter() {
            if field.required && !values.contains_key(field.name.clone()) {
                return Err(ContractError::InvalidInput);
            }
        }
    }

    let evidence = StructuredEvidence {
        grant_id,
        milestone_idx,
        values,
        submitted_by: caller.clone(),
        submitted_at: env.ledger().timestamp(),
    };
    Storage::set_structured_evidence(env, grant_id, milestone_idx, &evidence);
    Ok(())
}

/// Validate that structured evidence satisfying the schema has been submitted.
/// Returns Ok(()) if no schema exists (nothing to validate).
/// Returns Err(InvalidInput) if required fields are missing from submitted evidence.
pub fn validate_evidence(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    let schema = match Storage::get_evidence_schema(env, grant_id, milestone_idx) {
        Some(s) => s,
        None => return Ok(()),
    };

    let evidence = Storage::get_structured_evidence(env, grant_id, milestone_idx)
        .ok_or(ContractError::InvalidInput)?;

    for field in schema.fields.iter() {
        if field.required && !evidence.values.contains_key(field.name.clone()) {
            return Err(ContractError::InvalidInput);
        }
    }
    Ok(())
}

pub fn get_schema(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<EvidenceSchema> {
    Storage::get_evidence_schema(env, grant_id, milestone_idx)
}

pub fn get_evidence(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<StructuredEvidence> {
    Storage::get_structured_evidence(env, grant_id, milestone_idx)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{EvidenceFieldType, Grant, GrantStatus};
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{vec, Address, Env, Map, String};

    fn setup(env: &Env) -> Address {
        env.mock_all_auths();
        env.register(StellarGrantsContract, ())
    }

    fn setup_grant(env: &Env, id: u64, owner: &Address) {
        let grant = Grant {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "Original Grant"),
            description: String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 100,
            reviewers: vec![env],
            total_milestones: 10,
            milestones_paid_out: 0,
            escrow_balance: 1000,
            funders: vec![env],
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, id, &grant);
    }

    #[test]
    fn test_set_and_get_schema() {
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            setup_grant(&env, 1, &owner);

            let fields = vec![
                &env,
                EvidenceField {
                    name: String::from_str(&env, "github_pr"),
                    field_type: EvidenceFieldType::Url,
                    required: true,
                },
            ];

            let result = set_schema(&env, &owner, 1, 0, fields.clone());
            assert_eq!(result, Ok(()));

            let schema = get_schema(&env, 1, 0).unwrap();
            assert_eq!(schema.grant_id, 1);
            assert_eq!(schema.milestone_idx, 0);
            assert_eq!(schema.fields.len(), 1);
        });
    }

    #[test]
    fn test_submit_evidence_with_schema() {
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            setup_grant(&env, 1, &owner);

            let fields = vec![
                &env,
                EvidenceField {
                    name: String::from_str(&env, "github_pr"),
                    field_type: EvidenceFieldType::Url,
                    required: true,
                },
                EvidenceField {
                    name: String::from_str(&env, "notes"),
                    field_type: EvidenceFieldType::Text,
                    required: false,
                },
            ];
            set_schema(&env, &owner, 1, 0, fields).unwrap();

            // Missing required field
            let mut invalid_values = Map::new(&env);
            invalid_values.set(
                String::from_str(&env, "notes"),
                String::from_str(&env, "some notes"),
            );
            let result = submit_evidence(&env, &owner, 1, 0, invalid_values);
            assert_eq!(result, Err(ContractError::InvalidInput));

            // Valid submission
            let mut valid_values = Map::new(&env);
            valid_values.set(
                String::from_str(&env, "github_pr"),
                String::from_str(&env, "https://github.com"),
            );
            let result = submit_evidence(&env, &owner, 1, 0, valid_values);
            assert_eq!(result, Ok(()));

            // Validation succeeds
            let validate_result = validate_evidence(&env, 1, 0);
            assert_eq!(validate_result, Ok(()));
        });
    }

    #[test]
    fn test_submit_evidence_without_schema() {
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            setup_grant(&env, 1, &owner);

            // Validation succeeds without any schema
            let validate_result = validate_evidence(&env, 1, 0);
            assert_eq!(validate_result, Ok(()));

            let mut values = Map::new(&env);
            values.set(
                String::from_str(&env, "anything"),
                String::from_str(&env, "value"),
            );

            let submit_result = submit_evidence(&env, &owner, 1, 0, values);
            assert_eq!(submit_result, Ok(()));
        });
    }

    #[test]
    fn test_submit_evidence_rejects_out_of_range_milestone() {
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            setup_grant(&env, 1, &owner);

            let mut values = Map::new(&env);
            values.set(
                String::from_str(&env, "anything"),
                String::from_str(&env, "value"),
            );

            let result = submit_evidence(&env, &owner, 1, 999_999, values);
            assert_eq!(result, Err(ContractError::MilestoneIndexOutOfBounds));
        });
    }
}
