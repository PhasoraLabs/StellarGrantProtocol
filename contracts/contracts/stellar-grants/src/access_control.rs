use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{Role, RoleAssignment};
use soroban_sdk::{Address, Env, Vec};

/// One-time bootstrap that grants `Role::SuperAdmin` to the deployer.
///
/// Without this, the RBAC system is unreachable: `grant_role` requires the
/// caller to already hold `Role::SuperAdmin` or `Role::ProtocolAdmin`, but
/// nothing ever writes the first such assignment. This is guarded so it can
/// only ever run once: if `Role::SuperAdmin` already has any members, it
/// fails with `ContractError::AlreadyInitialized` instead of silently
/// re-granting or overwriting the existing assignment.
pub fn bootstrap_super_admin(env: &Env, deployer: &Address) -> Result<(), ContractError> {
    if !Storage::get_role_members(env, &Role::SuperAdmin).is_empty() {
        return Err(ContractError::AlreadyInitialized);
    }

    let assignment = RoleAssignment {
        holder: deployer.clone(),
        role: Role::SuperAdmin,
        granted_by: deployer.clone(),
        granted_at: env.ledger().timestamp(),
        expires_at: None,
        is_active: true,
    };

    Storage::set_role_assignment(env, deployer, &Role::SuperAdmin, &assignment);
    Storage::set_role_members(
        env,
        &Role::SuperAdmin,
        &soroban_sdk::vec![env, deployer.clone()],
    );

    Events::role_granted(env, deployer.clone(), Role::SuperAdmin, deployer.clone());

    Ok(())
}

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

    // Issue #901: revoking a role the holder does not have is a no-op and must
    // not emit a misleading RoleRevoked event.
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
        members.remove(index as u32);
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

    let mut all_roles = Vec::new(env);
    all_roles.push_back(Role::SuperAdmin);
    all_roles.push_back(Role::ProtocolAdmin);
    all_roles.push_back(Role::TreasuryManager);
    all_roles.push_back(Role::ComplianceOfficer);
    all_roles.push_back(Role::DisputeArbiter);
    all_roles.push_back(Role::OracleOperator);
    all_roles.push_back(Role::ReviewerModerator);
    all_roles.push_back(Role::EmergencyPauser);
    all_roles.push_back(Role::Relayer);

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
        members.remove(index as u32);
        Storage::set_role_members(env, &role, &members);
    }

    Events::role_renounced(env, holder.clone(), role);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _, testutils::Events as _, testutils::Ledger as _, Address, Env,
    };

    fn set_ledger(env: &Env, sequence: u32, timestamp: u64) {
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp,
            protocol_version: 25,
            sequence_number: sequence,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 100_000,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 1_000_000,
        });
    }

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let admin = Address::generate(&env);
        env.as_contract(&contract_id, || {
            // Bootstrap a SuperAdmin directly in storage
            let assignment = RoleAssignment {
                holder: admin.clone(),
                role: Role::SuperAdmin,
                granted_by: admin.clone(),
                granted_at: 0,
                expires_at: None,
                is_active: true,
            };
            Storage::set_role_assignment(&env, &admin, &Role::SuperAdmin, &assignment);
            Storage::set_role_members(
                &env,
                &Role::SuperAdmin,
                &soroban_sdk::vec![&env, admin.clone()],
            );
        });
        (env, admin, contract_id)
    }

    fn grant_role_via_client(
        env: &Env,
        contract_id: &Address,
        granter: &Address,
        grantee: &Address,
        role: Role,
        expires_at: Option<u64>,
    ) {
        crate::StellarGrantsContractClient::new(env, contract_id).rbac_grant_role(
            granter,
            grantee,
            &role,
            &expires_at,
        );
    }

    fn revoke_role_via_client(
        env: &Env,
        contract_id: &Address,
        revoker: &Address,
        holder: &Address,
        role: Role,
    ) {
        crate::StellarGrantsContractClient::new(env, contract_id)
            .rbac_revoke_role(revoker, holder, &role);
    }

    fn role_members_via_client(env: &Env, contract_id: &Address, role: Role) -> Vec<Address> {
        crate::StellarGrantsContractClient::new(env, contract_id).rbac_role_members(&role)
    }

    fn has_role_via_client(
        env: &Env,
        contract_id: &Address,
        address: &Address,
        role: Role,
    ) -> bool {
        crate::StellarGrantsContractClient::new(env, contract_id).rbac_has_role(address, &role)
    }

    fn roles_of_via_client(env: &Env, contract_id: &Address, address: &Address) -> Vec<Role> {
        crate::StellarGrantsContractClient::new(env, contract_id).rbac_roles_of(address)
    }

    // ── Bootstrap (#1077) ───────────────────────────────────────────────────

    #[test]
    fn bootstrap_super_admin_via_initialize_allows_first_grant() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let deployer = Address::generate(&env);

        // Fresh deploy -> initialize should bootstrap SuperAdmin for the deployer.
        client.initialize(&deployer);

        env.as_contract(&contract_id, || {
            assert!(has_role(&env, &deployer, Role::SuperAdmin));

            // The bootstrap must unblock the rest of the RBAC system: the
            // deployer, now a SuperAdmin, can grant other roles.
            let alice = Address::generate(&env);
            grant_role(&env, &deployer, &alice, Role::ProtocolAdmin, None).unwrap();
            assert!(has_role(&env, &alice, Role::ProtocolAdmin));
        });
    }

    #[test]
    fn bootstrap_super_admin_is_one_time_only() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let deployer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            bootstrap_super_admin(&env, &deployer).unwrap();
            assert!(has_role(&env, &deployer, Role::SuperAdmin));

            // A second bootstrap attempt (e.g. from calling initialize again)
            // must fail rather than silently re-granting SuperAdmin to a
            // different address.
            let attacker = Address::generate(&env);
            let err = bootstrap_super_admin(&env, &attacker);
            assert_eq!(err, Err(ContractError::AlreadyInitialized));
            assert!(!has_role(&env, &attacker, Role::SuperAdmin));
            assert!(has_role(&env, &deployer, Role::SuperAdmin));
        });
    }

    // ── Grant role ───────────────────────────────────────────────────────

    #[test]
    fn superadmin_can_grant_any_role() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::TreasuryManager, None).unwrap();
            assert!(has_role(&env, &alice, Role::TreasuryManager));
        });
    }

    #[test]
    fn superadmin_can_grant_protocol_admin() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::ProtocolAdmin, None).unwrap();
            assert!(has_role(&env, &alice, Role::ProtocolAdmin));
        });
    }

    #[test]
    fn non_admin_cannot_grant_roles() {
        let (env, _admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let bob = Address::generate(&env);
            let alice = Address::generate(&env);
            let err = grant_role(&env, &bob, &alice, Role::TreasuryManager, None);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn protocol_admin_cannot_grant_superadmin() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let pa = Address::generate(&env);
            grant_role(&env, &admin, &pa, Role::ProtocolAdmin, None).unwrap();
            let bob = Address::generate(&env);
            let err = grant_role(&env, &pa, &bob, Role::SuperAdmin, None);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn protocol_admin_cannot_grant_protocol_admin() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let pa = Address::generate(&env);
            grant_role(&env, &admin, &pa, Role::ProtocolAdmin, None).unwrap();
            let bob = Address::generate(&env);
            let err = grant_role(&env, &pa, &bob, Role::ProtocolAdmin, None);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn protocol_admin_can_grant_lesser_roles() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let pa = Address::generate(&env);
            grant_role(&env, &admin, &pa, Role::ProtocolAdmin, None).unwrap();

            let bob = Address::generate(&env);
            grant_role(&env, &pa, &bob, Role::DisputeArbiter, None).unwrap();
            assert!(has_role(&env, &bob, Role::DisputeArbiter));
        });
    }

    #[test]
    fn grant_role_with_expiry() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::EmergencyPauser, Some(100)).unwrap();
            assert!(has_role(&env, &alice, Role::EmergencyPauser));
        });
    }

    #[test]
    fn grant_role_updates_members_list() {
        let (env, admin, contract_id) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::OracleOperator,
            None,
        );
        grant_role_via_client(&env, &contract_id, &admin, &bob, Role::OracleOperator, None);
        let members = client.rbac_role_members(&Role::OracleOperator);
        assert_eq!(members.len(), 2);
        assert!(members.contains(alice));
        assert!(members.contains(bob));
    }

    #[test]
    fn grant_same_role_twice_does_not_duplicate_member() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(&env, &contract_id, &admin, &alice, Role::Relayer, None);
        grant_role_via_client(&env, &contract_id, &admin, &alice, Role::Relayer, None);
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::Relayer).len(),
            1
        );
    }

    // ── Revoke role ──────────────────────────────────────────────────────

    #[test]
    fn superadmin_can_revoke_roles() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::TreasuryManager,
            None,
        );
        revoke_role_via_client(&env, &contract_id, &admin, &alice, Role::TreasuryManager);
        assert!(!has_role_via_client(
            &env,
            &contract_id,
            &alice,
            Role::TreasuryManager
        ));
    }

    // Issue #901
    #[test]
    fn revoke_role_errors_when_holder_lacks_role() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            let err = revoke_role(&env, &admin, &alice, Role::TreasuryManager);
            assert_eq!(err, Err(ContractError::InvalidState));
        });
        // No RoleRevoked event may be emitted for the no-op attempt.
        assert_eq!(env.events().all().events().len(), 0);
    }

    #[test]
    fn revoke_removes_from_members_list() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::DisputeArbiter,
            None,
        );
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::DisputeArbiter).len(),
            1
        );
        revoke_role_via_client(&env, &contract_id, &admin, &alice, Role::DisputeArbiter);
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::DisputeArbiter).len(),
            0
        );
    }

    #[test]
    fn non_admin_cannot_revoke_roles() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            let bob = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::TreasuryManager, None).unwrap();
            let err = revoke_role(&env, &bob, &alice, Role::TreasuryManager);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn protocol_admin_cannot_revoke_superadmin() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let pa = Address::generate(&env);
            grant_role(&env, &admin, &pa, Role::ProtocolAdmin, None).unwrap();
            let err = revoke_role(&env, &pa, &admin, Role::SuperAdmin);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn protocol_admin_cannot_revoke_protocol_admin() {
        let (env, admin, contract_id) = setup();
        let pa1 = Address::generate(&env);
        let pa2 = Address::generate(&env);
        grant_role_via_client(&env, &contract_id, &admin, &pa1, Role::ProtocolAdmin, None);
        grant_role_via_client(&env, &contract_id, &admin, &pa2, Role::ProtocolAdmin, None);
        let err = env.as_contract(&contract_id, || {
            revoke_role(&env, &pa1, &pa2, Role::ProtocolAdmin)
        });
        assert_eq!(err, Err(ContractError::Unauthorized));
    }

    #[test]
    fn protocol_admin_can_revoke_lesser_roles() {
        let (env, admin, contract_id) = setup();
        let pa = Address::generate(&env);
        let alice = Address::generate(&env);
        grant_role_via_client(&env, &contract_id, &admin, &pa, Role::ProtocolAdmin, None);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::ReviewerModerator,
            None,
        );
        revoke_role_via_client(&env, &contract_id, &pa, &alice, Role::ReviewerModerator);
        assert!(!has_role_via_client(
            &env,
            &contract_id,
            &alice,
            Role::ReviewerModerator
        ));
    }

    // ── has_role / negative cases ─────────────────────────────────────────

    #[test]
    fn has_role_false_when_no_assignment() {
        let (env, _admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let nobody = Address::generate(&env);
            assert!(!has_role(&env, &nobody, Role::SuperAdmin));
        });
    }

    #[test]
    fn has_role_false_for_wrong_role() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            assert!(!has_role(&env, &admin, Role::TreasuryManager));
        });
    }

    #[test]
    fn has_role_false_when_expired() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::EmergencyPauser, Some(50)).unwrap();
            set_ledger(&env, 1, 51);
            assert!(!has_role(&env, &alice, Role::EmergencyPauser));
        });
    }

    #[test]
    fn has_role_true_before_expiry() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::EmergencyPauser, Some(100)).unwrap();
            set_ledger(&env, 1, 99);
            assert!(has_role(&env, &alice, Role::EmergencyPauser));
        });
    }

    #[test]
    fn has_role_false_after_revoke() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::DisputeArbiter,
            None,
        );
        revoke_role_via_client(&env, &contract_id, &admin, &alice, Role::DisputeArbiter);
        assert!(!has_role_via_client(
            &env,
            &contract_id,
            &alice,
            Role::DisputeArbiter
        ));
    }

    // ── require_role / require_any_role ───────────────────────────────────

    #[test]
    fn require_role_ok_when_holding() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            require_role(&env, &admin, Role::SuperAdmin).unwrap();
        });
    }

    #[test]
    fn require_role_err_when_missing() {
        let (env, _admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let nobody = Address::generate(&env);
            let err = require_role(&env, &nobody, Role::TreasuryManager);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn require_any_role_ok_when_holding_one() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let roles = soroban_sdk::vec![
                &env,
                Role::TreasuryManager,
                Role::SuperAdmin,
                Role::ComplianceOfficer,
            ];
            require_any_role(&env, &admin, roles).unwrap();
        });
    }

    #[test]
    fn require_any_role_err_when_holding_none() {
        let (env, _admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let nobody = Address::generate(&env);
            let roles = soroban_sdk::vec![
                &env,
                Role::TreasuryManager,
                Role::ComplianceOfficer,
                Role::DisputeArbiter,
            ];
            let err = require_any_role(&env, &nobody, roles);
            assert_eq!(err, Err(ContractError::Unauthorized));
        });
    }

    // ── role_members / roles_of ───────────────────────────────────────────

    #[test]
    fn role_members_empty_by_default() {
        let (env, _admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            assert_eq!(role_members(&env, Role::TreasuryManager).len(), 0);
        });
    }

    #[test]
    fn roles_of_empty_for_no_roles() {
        let (env, _admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let nobody = Address::generate(&env);
            assert_eq!(roles_of(&env, &nobody).len(), 0);
        });
    }

    #[test]
    fn roles_of_lists_all_granted_roles() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::TreasuryManager,
            None,
        );
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::DisputeArbiter,
            None,
        );
        let roles = roles_of_via_client(&env, &contract_id, &alice);
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(Role::TreasuryManager));
        assert!(roles.contains(Role::DisputeArbiter));
    }

    #[test]
    fn roles_of_excludes_revoked_roles() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::TreasuryManager,
            None,
        );
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::OracleOperator,
            None,
        );
        revoke_role_via_client(&env, &contract_id, &admin, &alice, Role::TreasuryManager);
        let roles = roles_of_via_client(&env, &contract_id, &alice);
        assert_eq!(roles.len(), 1);
        assert!(roles.contains(Role::OracleOperator));
    }

    // ── renounce_role ─────────────────────────────────────────────────────

    #[test]
    fn renounce_removes_own_role() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::TreasuryManager, None).unwrap();
            renounce_role(&env, &alice, Role::TreasuryManager).unwrap();
            assert!(!has_role(&env, &alice, Role::TreasuryManager));
        });
    }

    #[test]
    fn renounce_removes_from_members() {
        let (env, admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            grant_role(&env, &admin, &alice, Role::Relayer, None).unwrap();
            renounce_role(&env, &alice, Role::Relayer).unwrap();
            assert_eq!(role_members(&env, Role::Relayer).len(), 0);
        });
    }

    #[test]
    fn renounce_nonexistent_role_fails() {
        let (env, _admin, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let alice = Address::generate(&env);
            let err = renounce_role(&env, &alice, Role::TreasuryManager);
            assert_eq!(err, Err(ContractError::InvalidState));
        });
    }

    #[test]
    fn renounce_already_revoked_role_fails() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::OracleOperator,
            None,
        );
        revoke_role_via_client(&env, &contract_id, &admin, &alice, Role::OracleOperator);
        let err = env.as_contract(&contract_id, || {
            renounce_role(&env, &alice, Role::OracleOperator)
        });
        assert_eq!(err, Err(ContractError::InvalidState));
    }

    // ── Role-count tracking ───────────────────────────────────────────────

    #[test]
    fn grant_increases_member_count() {
        let (env, admin, contract_id) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        client.rbac_grant_role(&admin, &alice, &Role::ComplianceOfficer, &None);
        assert_eq!(client.rbac_role_members(&Role::ComplianceOfficer).len(), 1);
        client.rbac_grant_role(&admin, &bob, &Role::ComplianceOfficer, &None);
        assert_eq!(client.rbac_role_members(&Role::ComplianceOfficer).len(), 2);
    }

    #[test]
    fn revoke_decreases_member_count() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::ComplianceOfficer,
            None,
        );
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &bob,
            Role::ComplianceOfficer,
            None,
        );
        revoke_role_via_client(&env, &contract_id, &admin, &alice, Role::ComplianceOfficer);
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::ComplianceOfficer).len(),
            1
        );
    }

    #[test]
    fn multiple_roles_independent_member_lists() {
        let (env, admin, contract_id) = setup();
        let alice = Address::generate(&env);
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::TreasuryManager,
            None,
        );
        grant_role_via_client(
            &env,
            &contract_id,
            &admin,
            &alice,
            Role::DisputeArbiter,
            None,
        );
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::TreasuryManager).len(),
            1
        );
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::DisputeArbiter).len(),
            1
        );
        revoke_role_via_client(&env, &contract_id, &admin, &alice, Role::TreasuryManager);
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::TreasuryManager).len(),
            0
        );
        assert_eq!(
            role_members_via_client(&env, &contract_id, Role::DisputeArbiter).len(),
            1
        );
    }

    // ── All role variants ─────────────────────────────────────────────────

    #[test]
    fn can_grant_and_check_each_role_variant() {
        let (env, admin, contract_id) = setup();
        let roles = [
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
        for role in roles {
            let user = Address::generate(&env);
            grant_role_via_client(&env, &contract_id, &admin, &user, role.clone(), None);
            assert!(has_role_via_client(&env, &contract_id, &user, role.clone()));
            let members = role_members_via_client(&env, &contract_id, role);
            assert!(members.contains(user));
        }
    }
}
