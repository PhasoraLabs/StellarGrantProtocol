use soroban_sdk::{Address, Env, String, Vec};

use crate::constants::MAX_PAGE_SIZE;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, RegistryEntry, RegistryEntryType};

/// Add a contributor to the global registry index. Called on registration.
pub fn register_contributor(
    env: &Env,
    address: &Address,
    name: &String,
) -> Result<(), ContractError> {
    let mut index = Storage::get_contributor_index(env);

    for entry in index.iter() {
        if entry.address == *address {
            return Err(ContractError::AlreadyRegistered);
        }
    }

    let entry = RegistryEntry {
        address: address.clone(),
        registered_at: env.ledger().timestamp(),
        is_active: true,
        entry_type: RegistryEntryType::Contributor,
    };

    index.push_back(entry);
    Storage::set_contributor_index(env, &index);

    Events::emit_contributor_registered(env, address.clone(), name.clone());

    Ok(())
}

/// Add an address to the approved reviewer allowlist. Admin only.
pub fn approve_reviewer(
    env: &Env,
    admin: &Address,
    reviewer: &Address,
) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;

    let mut allowlist = Storage::get_reviewer_allowlist(env);

    if allowlist.contains(reviewer.clone()) {
        return Ok(());
    }

    allowlist.push_back(reviewer.clone());
    Storage::set_reviewer_allowlist(env, &allowlist);

    Events::emit_reviewer_approved(env, reviewer.clone(), admin.clone());

    Ok(())
}

/// Remove an address from the approved reviewer allowlist. Admin only.
pub fn revoke_reviewer(
    env: &Env,
    admin: &Address,
    reviewer: &Address,
) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;

    let allowlist = Storage::get_reviewer_allowlist(env);
    let mut new_list: Vec<Address> = Vec::new(env);
    let mut found = false;

    for addr in allowlist.iter() {
        if addr == *reviewer {
            found = true;
        } else {
            new_list.push_back(addr);
        }
    }

    if !found {
        return Err(ContractError::InvalidInput);
    }

    Storage::set_reviewer_allowlist(env, &new_list);

    Events::emit_reviewer_revoked(env, reviewer.clone(), admin.clone());

    Ok(())
}

/// Check if an address is on the approved reviewer allowlist.
pub fn is_approved_reviewer(env: &Env, address: &Address) -> bool {
    let allowlist = Storage::get_reviewer_allowlist(env);
    allowlist.contains(address.clone())
}

/// Paginated list of all registered contributor addresses.
pub fn get_contributors_page(env: &Env, offset: u32, limit: u32) -> Vec<RegistryEntry> {
    let index = Storage::get_contributor_index(env);
    let total = index.len();
    let effective_limit = if limit > MAX_PAGE_SIZE {
        MAX_PAGE_SIZE
    } else {
        limit
    };
    let mut result: Vec<RegistryEntry> = Vec::new(env);

    let mut i = offset;
    while i < total && i - offset < effective_limit {
        if let Some(entry) = index.get(i) {
            result.push_back(entry);
        }
        i += 1;
    }

    result
}

/// Total count of registered contributors.
pub fn contributor_count(env: &Env) -> u32 {
    Storage::get_contributor_index(env).len()
}

/// Deactivate a contributor entry (soft-delete). Admin only.
pub fn deactivate(env: &Env, admin: &Address, address: &Address) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;

    let index = Storage::get_contributor_index(env);
    let mut new_index: Vec<RegistryEntry> = Vec::new(env);
    let mut found = false;

    for mut entry in index.iter() {
        if entry.address == *address {
            entry.is_active = false;
            found = true;
        }
        new_index.push_back(entry);
    }

    if !found {
        return Err(ContractError::InvalidInput);
    }

    Storage::set_contributor_index(env, &new_index);

    Ok(())
}

fn require_global_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    let admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if admin != *caller {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}
