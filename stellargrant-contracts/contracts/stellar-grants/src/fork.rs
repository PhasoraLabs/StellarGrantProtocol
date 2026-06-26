use soroban_sdk::{Address, Env, String, Vec};

use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ForkRecord, GrantStatus};

const MAX_FORK_DEPTH: u32 = 5;

pub fn fork_grant(
    env: &Env,
    caller: &Address,
    original_grant_id: u64,
    new_title: String,
    new_description: String,
    new_total_amount: i128,
    new_token: &Address,
    inherit_reviewers: bool,
    inherit_milestones: bool,
) -> Result<u64, ContractError> {
    let original = Storage::get_grant(env, original_grant_id).ok_or(ContractError::GrantNotFound)?;

    if original.status == GrantStatus::Cancelled {
        return Err(ContractError::InvalidState);
    }

    let depth = fork_depth(env, original_grant_id);
    if depth >= MAX_FORK_DEPTH {
        return Err(ContractError::InvalidInput);
    }

    let reviewers = if inherit_reviewers {
        original.reviewers.clone()
    } else {
        Vec::new(env)
    };

    let new_grant_id = crate::internal_grant_create(
        env,
        caller,
        new_title,
        new_description,
        new_token,
        new_total_amount,
        original.milestone_amount,
        original.total_milestones,
        reviewers,
    )?;

    let mut inherited_fields = Vec::new(env);
    let mut overridden_fields = Vec::new(env);
    if inherit_reviewers {
        inherited_fields.push_back(String::from_str(env, "reviewers"));
    }
    if inherit_milestones {
        inherited_fields.push_back(String::from_str(env, "milestones"));
    }
    overridden_fields.push_back(String::from_str(env, "title"));
    overridden_fields.push_back(String::from_str(env, "description"));
    overridden_fields.push_back(String::from_str(env, "total_amount"));
    overridden_fields.push_back(String::from_str(env, "token"));

    let record = ForkRecord {
        original_grant_id,
        forked_grant_id: new_grant_id,
        forked_by: caller.clone(),
        forked_at: env.ledger().timestamp(),
        inherited_fields,
        overridden_fields,
    };

    Storage::set_fork_record(env, new_grant_id, &record);

    let mut children: Vec<u64> = Storage::get_fork_children(env, original_grant_id);
    if !children.contains(new_grant_id) {
        children.push_back(new_grant_id);
        Storage::set_fork_children(env, original_grant_id, &children);
    }

    Events::emit_grant_forked(env, original_grant_id, new_grant_id);

    Ok(new_grant_id)
}

pub fn get_fork_record(env: &Env, grant_id: u64) -> Option<ForkRecord> {
    Storage::get_fork_record(env, grant_id)
}

pub fn get_forks(env: &Env, original_grant_id: u64) -> Vec<u64> {
    Storage::get_fork_children(env, original_grant_id)
}

pub fn fork_depth(env: &Env, grant_id: u64) -> u32 {
    let mut depth = 0u32;
    let mut current = grant_id;
    loop {
        if let Some(record) = Storage::get_fork_record(env, current) {
            current = record.original_grant_id;
            depth += 1;
        } else {
            break;
        }
    }
    depth
}

pub fn is_descendant(env: &Env, ancestor_id: u64, descendant_id: u64) -> bool {
    let mut current = descendant_id;
    loop {
        if let Some(record) = Storage::get_fork_record(env, current) {
            if record.original_grant_id == ancestor_id {
                return true;
            }
            current = record.original_grant_id;
        } else {
            return false;
        }
    }
}
