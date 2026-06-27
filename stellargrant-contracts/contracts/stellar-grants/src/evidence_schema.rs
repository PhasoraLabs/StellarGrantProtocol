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
