use soroban_sdk::{Address, Env, Map, String, Symbol, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{Amendment, AmendmentStatus, Grant, GrantVersion};

fn field(env: &Env, name: &str) -> String {
    String::from_str(env, name)
}

fn grant_to_version(
    env: &Env,
    grant: &Grant,
    version: u32,
    amendment_id: Option<u32>,
) -> GrantVersion {
    GrantVersion {
        grant_id: grant.id,
        version,
        title: grant.title.clone(),
        description: grant.description.clone(),
        total_amount: grant.total_amount,
        total_milestones: grant.total_milestones,
        created_at: env.ledger().timestamp(),
        amendment_id,
    }
}

fn is_material(env: &Env, changed_fields: &Vec<String>) -> bool {
    let title = field(env, "title");
    let total_amount = field(env, "total_amount");
    let total_milestones = field(env, "total_milestones");
    for changed in changed_fields.iter() {
        if changed == title || changed == total_amount || changed == total_milestones {
            return true;
        }
    }
    false
}

fn current_snapshot(env: &Env, grant_id: u64) -> Result<GrantVersion, ContractError> {
    let current = current_version(env, grant_id);
    if current == 0 {
        let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
        let snapshot = grant_to_version(env, &grant, 1, None);
        Storage::set_grant_version(env, grant_id, 1, &snapshot);
        Storage::set_current_version(env, grant_id, 1);
        return Ok(snapshot);
    }
    Storage::get_grant_version(env, grant_id, current).ok_or(ContractError::GrantNotFound)
}

pub fn create_initial_version(env: &Env, grant: &Grant) {
    let snapshot = grant_to_version(env, grant, 1, None);
    Storage::set_grant_version(env, grant.id, 1, &snapshot);
    Storage::set_current_version(env, grant.id, 1);
}

/// Propose an amendment to a grant. Owner only.
pub fn propose_amendment(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    changed_fields: Vec<String>,
    new_values: Vec<String>,
    rationale: String,
) -> Result<u32, ContractError> {
    if changed_fields.is_empty() || changed_fields.len() != new_values.len() {
        return Err(ContractError::InvalidInput);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    let current = current_snapshot(env, grant_id)?;
    let amendment_version = current.version.saturating_add(1);
    let mut previous_values = Vec::new(env);
    let title = field(env, "title");
    let description = field(env, "description");
    let total_amount = field(env, "total_amount");
    let total_milestones = field(env, "total_milestones");

    for changed in changed_fields.iter() {
        if changed == title {
            previous_values.push_back(current.title.clone());
        } else if changed == description {
            previous_values.push_back(current.description.clone());
        } else if changed == total_amount {
            previous_values.push_back(field(env, "current_total_amount"));
        } else if changed == total_milestones {
            previous_values.push_back(field(env, "current_total_milestones"));
        } else {
            return Err(ContractError::InvalidInput);
        }
    }

    let status = if is_material(env, &changed_fields) {
        AmendmentStatus::Proposed
    } else {
        AmendmentStatus::Approved
    };
    let resolved_at = if status == AmendmentStatus::Approved {
        Some(env.ledger().timestamp())
    } else {
        None
    };
    let amendment = Amendment {
        grant_id,
        version: amendment_version,
        proposed_by: owner.clone(),
        changed_fields,
        previous_values,
        new_values,
        rationale,
        status: status.clone(),
        reviewer_votes: Map::new(env),
        proposed_at: env.ledger().timestamp(),
        resolved_at,
    };

    Storage::set_amendment(env, grant_id, amendment_version, &amendment);
    let mut history = Storage::get_amendment_history(env, grant_id);
    history.push_back(amendment_version);
    Storage::set_amendment_history(env, grant_id, &history);
    env.events().publish(
        (Symbol::new(env, "amendment_proposed"), grant_id),
        (owner.clone(), amendment_version),
    );
    if status == AmendmentStatus::Approved {
        env.events().publish(
            (Symbol::new(env, "amendment_approved"), grant_id),
            amendment_version,
        );
    }
    Ok(amendment_version)
}

/// Reviewer votes on an amendment.
pub fn vote_amendment(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    amendment_version: u32,
    approve: bool,
) -> Result<AmendmentStatus, ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if !grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let mut amendment = Storage::get_amendment(env, grant_id, amendment_version)
        .ok_or(ContractError::InvalidInput)?;
    if amendment.status != AmendmentStatus::Proposed {
        return Err(ContractError::InvalidState);
    }
    if amendment.reviewer_votes.contains_key(reviewer.clone()) {
        return Err(ContractError::AlreadyVoted);
    }

    amendment.reviewer_votes.set(reviewer.clone(), approve);
    amendment.status = if approve {
        AmendmentStatus::Approved
    } else {
        AmendmentStatus::Rejected
    };
    amendment.resolved_at = Some(env.ledger().timestamp());
    Storage::set_amendment(env, grant_id, amendment_version, &amendment);

    if amendment.status == AmendmentStatus::Approved {
        env.events().publish(
            (Symbol::new(env, "amendment_approved"), grant_id),
            amendment_version,
        );
    }
    Ok(amendment.status)
}

/// Apply an approved amendment, creating a new version snapshot.
pub fn apply_amendment(
    env: &Env,
    grant_id: u64,
    amendment_version: u32,
) -> Result<GrantVersion, ContractError> {
    let amendment = Storage::get_amendment(env, grant_id, amendment_version)
        .ok_or(ContractError::InvalidInput)?;
    if amendment.status != AmendmentStatus::Approved {
        return Err(ContractError::InvalidState);
    }

    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let mut snapshot = current_snapshot(env, grant_id)?;
    let title = field(env, "title");
    let description = field(env, "description");
    for i in 0..amendment.changed_fields.len() {
        let changed = amendment.changed_fields.get(i).unwrap();
        let value = amendment.new_values.get(i).unwrap();
        if changed == title {
            snapshot.title = value.clone();
            grant.title = value;
        } else if changed == description {
            snapshot.description = value.clone();
            grant.description = value;
        }
    }

    snapshot.version = amendment_version;
    snapshot.created_at = env.ledger().timestamp();
    snapshot.amendment_id = Some(amendment_version);
    Storage::set_grant(env, grant_id, &grant);
    Storage::set_grant_version(env, grant_id, amendment_version, &snapshot);
    Storage::set_current_version(env, grant_id, amendment_version);
    env.events().publish(
        (Symbol::new(env, "amendment_applied"), grant_id),
        amendment_version,
    );
    Ok(snapshot)
}

/// Return a specific version snapshot.
pub fn get_version(env: &Env, grant_id: u64, version: u32) -> Option<GrantVersion> {
    Storage::get_grant_version(env, grant_id, version)
}

/// Return the current version number for a grant.
pub fn current_version(env: &Env, grant_id: u64) -> u32 {
    Storage::get_current_version(env, grant_id)
}

/// Return the full amendment history for a grant.
pub fn amendment_history(env: &Env, grant_id: u64) -> Vec<Amendment> {
    let mut amendments = Vec::new(env);
    for version in Storage::get_amendment_history(env, grant_id).iter() {
        if let Some(amendment) = Storage::get_amendment(env, grant_id, version) {
            amendments.push_back(amendment);
        }
    }
    amendments
}
