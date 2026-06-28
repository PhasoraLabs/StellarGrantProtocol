use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{Role, RoleAssignment};
use soroban_sdk::{Address, Env, Vec};

/// Grant a role to an address. SuperAdmin only (or ProtocolAdmin for lesser roles).
pub fn grant_role(
    env: &Env,
    granter: &Address,
    grantee: &Address,
    role: Role,
    expires_at: Option<u64>,
) -> Result<(), ContractError> {
    granter.require_auth();

    // Check authorization based on role hierarchy
    let granter_roles = roles_of(env, granter);

    let can_grant = if granter_roles.contains(Role::SuperAdmin) {
        // SuperAdmin can grant any role
        true
    } else if granter_roles.contains(Role::ProtocolAdmin) {
        // ProtocolAdmin can grant roles 2-8 (not SuperAdmin or ProtocolAdmin)
        match role {
            Role::SuperAdmin | Role::ProtocolAdmin => false,
            _ => true,
        }
    } else {
        false
    };

    if !can_grant {
        return Err(ContractError::Unauthorized);
    }

    let assignment = RoleAssignment {
        holder: grantee.clone(),
        role: role.clone(),
        granted_by: granter.clone(),
        granted_at: env.ledger().timestamp(),
        expires_at,
        is_active: true,
    };

    Storage::set_role_assignment(env, grantee, &role, &assignment);

    // Update role members list
    let mut members = Storage::get_role_members(env, &role);
    if !members.contains(grantee.clone()) {
        members.push_back(grantee.clone());
        Storage::set_role_members(env, &role, &members);
    }

    Events::role_granted(env, grantee.clone(), role, granter.clone());

    Ok(())
}

/// Revoke a role. SuperAdmin or ProtocolAdmin.
pub fn revoke_role(
    env: &Env,
    revoker: &Address,
    holder: &Address,
    role: Role,
) -> Result<(), ContractError> {
    revoker.require_auth();

    let revoker_roles = roles_of(env, revoker);

    let can_revoke = if revoker_roles.contains(Role::SuperAdmin) {
        true
    } else if revoker_roles.contains(Role::ProtocolAdmin) {
        match role {
            Role::SuperAdmin | Role::ProtocolAdmin => false,
            _ => true,
        }
    } else {
        false
    };

    if !can_revoke {
        return Err(ContractError::Unauthorized);
    }

    // Mark assignment as inactive
    if let Some(mut assignment) = Storage::get_role_assignment(env, holder, &role) {
        assignment.is_active = false;
        Storage::set_role_assignment(env, holder, &role, &assignment);
    }

    // Remove from role members list
    let mut members = Storage::get_role_members(env, &role);
    if let Some(index) = members.iter().position(|a| a == *holder) {
        members.remove(index);
        Storage::set_role_members(env, &role, &members);
    }

    Events::role_revoked(env, holder.clone(), role, revoker.clone());

    Ok(())
}

/// Check if an address holds a specific role (respects expiry).
pub fn has_role(env: &Env, address: &Address, role: Role) -> bool {
    if let Some(assignment) = Storage::get_role_assignment(env, address, &role) {
        if !assignment.is_active {
            return false;
        }

        // Check expiry
        if let Some(expires_at) = assignment.expires_at {
            if env.ledger().timestamp() > expires_at {
                return false;
            }
        }

        return true;
    }

    false
}

/// Assert that an address holds a role; return Err(Unauthorized) if not.
pub fn require_role(env: &Env, address: &Address, role: Role) -> Result<(), ContractError> {
    if !has_role(env, address, role) {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

/// Assert any of a list of roles (OR logic). Returns Ok if holder has at least one.
pub fn require_any_role(
    env: &Env,
    address: &Address,
    roles: Vec<Role>,
) -> Result<(), ContractError> {
    for role in roles.iter() {
        if has_role(env, address, role) {
            return Ok(());
        }
    }
    Err(ContractError::Unauthorized)
}

/// Return all addresses holding a specific role.
pub fn role_members(env: &Env, role: Role) -> Vec<Address> {
    Storage::get_role_members(env, &role)
}

/// Return all roles held by an address.
pub fn roles_of(env: &Env, address: &Address) -> Vec<Role> {
    let mut roles = Vec::new(env);

    // Check all role types
    let all_roles = vec![
        Role::SuperAdmin,
        Role::ProtocolAdmin,
        Role::TreasuryManager,
        Role::ComplianceOfficer,
        Role::DisputeArbiter,
        Role::OracleOperator,
        Role::ReviewerModerator,
        Role::EmergencyPauser,
        Role::Relayer,
    ];

    for role in all_roles {
        if has_role(env, address, role.clone()) {
            roles.push_back(role);
        }
    }

    roles
}

/// Renounce your own role (voluntary self-removal).
pub fn renounce_role(env: &Env, holder: &Address, role: Role) -> Result<(), ContractError> {
    holder.require_auth();

    if !has_role(env, holder, role.clone()) {
        return Err(ContractError::InvalidState);
    }

    // Mark assignment as inactive
    if let Some(mut assignment) = Storage::get_role_assignment(env, holder, &role) {
        assignment.is_active = false;
        Storage::set_role_assignment(env, holder, &role, &assignment);
    }

    // Remove from role members list
    let mut members = Storage::get_role_members(env, &role);
    if let Some(index) = members.iter().position(|a| a == *holder) {
        members.remove(index);
        Storage::set_role_members(env, &role, &members);
    }

    Events::role_renounced(env, holder.clone(), role);

    Ok(())
}
