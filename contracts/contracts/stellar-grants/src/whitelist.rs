use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{WhitelistEntry, WhitelistMode, WhitelistScope};

/// Add an address to a whitelist scope. Admin or grant owner only.
pub fn add(
    env: &Env,
    caller: &Address,
    address: &Address,
    scope: &WhitelistScope,
) -> Result<(), ContractError> {
    caller.require_auth();

    // Authorization: global admin or grant owner for per-grant scopes.
    if !is_scope_admin(env, caller, scope) {
        return Err(ContractError::Unauthorized);
    }

    // Check if already whitelisted.
    if is_allowed_exact(env, address, scope) {
        return Err(ContractError::AlreadyRegistered);
    }

    let entry = WhitelistEntry {
        address: address.clone(),
        added_by: caller.clone(),
        added_at: env.ledger().timestamp(),
        scope: scope.clone(),
    };

    Storage::push_whitelist_entry(env, scope, &entry);

    Events::emit_whitelist_address_added(env, address.clone(), scope.clone());

    Ok(())
}

/// Remove an address from a whitelist scope. Admin or grant owner only.
pub fn remove(
    env: &Env,
    caller: &Address,
    address: &Address,
    scope: &WhitelistScope,
) -> Result<(), ContractError> {
    caller.require_auth();

    if !is_scope_admin(env, caller, scope) {
        return Err(ContractError::Unauthorized);
    }

    let entries = Storage::get_whitelist_entries(env, scope);
    let mut new_entries: Vec<WhitelistEntry> = Vec::new(env);
    let mut removed = false;

    for entry in entries.iter() {
        if entry.address != *address {
            new_entries.push_back(entry);
        } else {
            removed = true;
        }
    }

    if !removed {
        // Not on list; no-op or error — let's silently succeed for idempotency.
        return Ok(());
    }

    Storage::set_whitelist_entries(env, scope, &new_entries);

    Events::emit_whitelist_address_removed(env, address.clone(), scope.clone());

    Ok(())
}

/// Check if an address is on the whitelist for a scope.
/// If mode is Open, always returns true.
pub fn is_allowed(env: &Env, address: &Address, scope: &WhitelistScope) -> bool {
    let mode = get_mode(env, scope);
    if mode == WhitelistMode::Open {
        return true;
    }
    is_allowed_exact(env, address, scope)
}

/// Set the operating mode for a scope (Open or Restricted). Admin only.
pub fn set_mode(
    env: &Env,
    admin: &Address,
    scope: &WhitelistScope,
    mode: WhitelistMode,
) -> Result<(), ContractError> {
    admin.require_auth();

    let is_global_admin = Storage::get_global_admin(env) == Some(admin.clone());
    if !is_global_admin {
        // Grant owners can set mode for per-grant scopes.
        if let WhitelistScope::GrantReviewer(grant_id) = scope {
            if let Some(grant) = Storage::get_grant(env, *grant_id) {
                if grant.owner != *admin {
                    return Err(ContractError::Unauthorized);
                }
            } else {
                return Err(ContractError::GrantNotFound);
            }
        } else {
            return Err(ContractError::Unauthorized);
        }
    }

    Storage::set_whitelist_mode(env, scope, mode);
    Ok(())
}

/// Return the current mode for a scope.
pub fn get_mode(env: &Env, scope: &WhitelistScope) -> WhitelistMode {
    Storage::get_whitelist_mode(env, scope).unwrap_or(WhitelistMode::Open)
}

/// Return all entries in a whitelist scope.
pub fn get_entries(env: &Env, scope: &WhitelistScope) -> Vec<WhitelistEntry> {
    Storage::get_whitelist_entries(env, scope)
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Check exact membership (doesn't fall back to Open mode).
fn is_allowed_exact(env: &Env, address: &Address, scope: &WhitelistScope) -> bool {
    let entries = Storage::get_whitelist_entries(env, scope);
    for entry in entries.iter() {
        if entry.address == *address {
            return true;
        }
    }
    false
}

/// Determine if `caller` is authorized to manage the given scope.
fn is_scope_admin(env: &Env, caller: &Address, scope: &WhitelistScope) -> bool {
    // Global admin can manage any scope.
    if Storage::get_global_admin(env) == Some(caller.clone()) {
        return true;
    }

    // Per-grant scopes: grant owner can manage.
    if let WhitelistScope::GrantReviewer(grant_id) = scope {
        if let Some(grant) = Storage::get_grant(env, *grant_id) {
            return grant.owner == *caller;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantFund, GrantStatus};
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn make_grant(env: &Env, owner: &Address) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: soroban_sdk::String::from_str(env, "T"),
            description: soroban_sdk::String::from_str(env, "D"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 2,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        }
    }

    #[test]
    fn test_open_mode_always_allows() {
        let env = Env::default();
        let addr = Address::generate(&env);
        let scope = WhitelistScope::GlobalContributor;

        // Default is Open.
        assert!(is_allowed(&env, &addr, &scope));
    }

    #[test]
    fn test_restricted_mode_blocks_non_member() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let member = Address::generate(&env);
        let non_member = Address::generate(&env);
        let scope = WhitelistScope::GlobalContributor;

        Storage::set_global_admin(&env, &admin);

        // Set Restricted mode.
        set_mode(&env, &admin, &scope, WhitelistMode::Restricted).unwrap();
        assert_eq!(get_mode(&env, &scope), WhitelistMode::Restricted);

        // Non-member should be blocked.
        assert!(!is_allowed(&env, &non_member, &scope));

        // Add member.
        add(&env, &admin, &member, &scope).unwrap();
        assert!(is_allowed(&env, &member, &scope));
    }

    #[test]
    fn test_add_remove_roundtrip() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let addr = Address::generate(&env);
        let scope = WhitelistScope::GlobalReviewer;

        Storage::set_global_admin(&env, &admin);
        set_mode(&env, &admin, &scope, WhitelistMode::Restricted).unwrap();

        // Add.
        add(&env, &admin, &addr, &scope).unwrap();
        assert!(is_allowed(&env, &addr, &scope));

        // Remove.
        remove(&env, &admin, &addr, &scope).unwrap();
        assert!(!is_allowed(&env, &addr, &scope));
    }

    #[test]
    fn test_global_vs_per_grant_scope() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let addr = Address::generate(&env);

        let global_scope = WhitelistScope::GlobalReviewer;
        let per_grant_scope = WhitelistScope::GrantReviewer(1);

        Storage::set_global_admin(&env, &admin);
        Storage::set_grant(&env, 1, &make_grant(&env, &owner));

        // Set global scope to restricted; add addr to per-grant only.
        set_mode(&env, &admin, &global_scope, WhitelistMode::Restricted).unwrap();
        add(&env, &owner, &addr, &per_grant_scope).unwrap();

        // Global scope: addr not in global list -> blocked.
        assert!(!is_allowed(&env, &addr, &global_scope));

        // Per-grant scope: addr IS in per-grant list -> allowed.
        assert!(is_allowed(&env, &addr, &per_grant_scope));
    }

    #[test]
    fn test_unauthorized_add_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let stranger = Address::generate(&env);
        let addr = Address::generate(&env);
        let scope = WhitelistScope::GlobalContributor;

        assert_eq!(
            add(&env, &stranger, &addr, &scope),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn test_grant_owner_can_manage_per_grant_scope() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let addr = Address::generate(&env);
        let scope = WhitelistScope::GrantReviewer(1);

        Storage::set_grant(&env, 1, &make_grant(&env, &owner));

        // Grant owner can add to per-grant scope even without global admin.
        add(&env, &owner, &addr, &scope).unwrap();
        assert!(is_allowed(&env, &addr, &scope));
    }
}
