use soroban_sdk::{token, Address, Env, String, Vec};

use crate::access_control::{has_role, require_role};
use crate::constants::CLAWBACK_DISPUTE_WINDOW_SECONDS;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ClawbackRequest, ClawbackStatus, ContractError, MilestoneState, Role};

/// Initiate a clawback. Requires DisputeArbiter role.
pub fn initiate(
    env: &Env,
    initiator: &Address,
    grant_id: u64,
    milestone_idx: u32,
    reason: String,
) -> Result<(), ContractError> {
    initiator.require_auth();

    // Check authorization - requires DisputeArbiter role
    require_role(env, initiator, Role::DisputeArbiter)?;

    // Verify grant exists
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    // Verify milestone exists and is Paid
    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;

    if milestone.state != MilestoneState::Paid {
        return Err(ContractError::InvalidState);
    }

    // Check if clawback already exists
    if Storage::get_clawback(env, grant_id, milestone_idx).is_some() {
        return Err(ContractError::InvalidState);
    }

    let now = env.ledger().timestamp();
    let dispute_window_ends = now.saturating_add(CLAWBACK_DISPUTE_WINDOW_SECONDS);

    // Create clawback request
    let clawback = ClawbackRequest {
        grant_id,
        milestone_idx,
        target: grant.owner.clone(),
        amount: milestone.amount,
        token: grant.token.clone(),
        reason,
        initiated_by: initiator.clone(),
        initiated_at: now,
        dispute_window_ends,
        approvals: Vec::new(env),
        required_approvals: 2, // ProtocolAdmin + DisputeArbiter
        status: ClawbackStatus::Pending,
    };

    Storage::set_clawback(env, grant_id, milestone_idx, &clawback);

    Events::emit_clawback_initiated(
        env,
        grant_id,
        milestone_idx,
        clawback.target.clone(),
        clawback.amount,
        clawback.token.clone(),
        initiator.clone(),
        dispute_window_ends,
    );

    Ok(())
}

/// Approve a pending clawback. Required approvers: ProtocolAdmin + DisputeArbiter.
pub fn approve(
    env: &Env,
    approver: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    approver.require_auth();

    let mut clawback =
        Storage::get_clawback(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;

    // Check status
    if clawback.status != ClawbackStatus::Pending {
        return Err(ContractError::InvalidState);
    }

    // Check if already approved by this address
    if clawback.approvals.contains(approver.clone()) {
        return Err(ContractError::AlreadyVoted);
    }

    // Approver must be either ProtocolAdmin or DisputeArbiter
    let is_protocol_admin = has_role(env, approver, Role::ProtocolAdmin);
    let is_dispute_arbiter = has_role(env, approver, Role::DisputeArbiter);

    if !is_protocol_admin && !is_dispute_arbiter {
        return Err(ContractError::Unauthorized);
    }

    // Add approval
    clawback.approvals.push_back(approver.clone());

    // Check if we have enough approvals
    if clawback.approvals.len() >= clawback.required_approvals {
        clawback.status = ClawbackStatus::Approved;
    }

    Storage::set_clawback(env, grant_id, milestone_idx, &clawback);

    Events::emit_clawback_approved(env, grant_id, milestone_idx, approver.clone());

    Ok(())
}

/// Contributor disputes the clawback during the dispute window.
pub fn dispute(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    contributor.require_auth();

    let mut clawback =
        Storage::get_clawback(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;

    // Verify contributor is the target
    if clawback.target != *contributor {
        return Err(ContractError::Unauthorized);
    }

    // Check status - must be Pending or Approved
    if clawback.status != ClawbackStatus::Pending && clawback.status != ClawbackStatus::Approved {
        return Err(ContractError::InvalidState);
    }

    // Check dispute window
    let now = env.ledger().timestamp();
    if now > clawback.dispute_window_ends {
        return Err(ContractError::DeadlinePassed);
    }

    // Update status to disputed
    clawback.status = ClawbackStatus::DisputedByContributor;
    Storage::set_clawback(env, grant_id, milestone_idx, &clawback);

    Events::emit_clawback_disputed(env, grant_id, milestone_idx, contributor.clone());

    // Raise a formal dispute for arbitration
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let dispute_reason = String::from_str(env, "Clawback disputed by contributor");
    crate::dispute::raise_dispute(env, &grant, milestone_idx, contributor, dispute_reason)?;

    Ok(())
}

/// Pre-authorize the contract to later pull up to `amount` of `token` from the
/// contributor's wallet via the SEP-41 allowance mechanism (`approve` /
/// `transfer_from`), so a future clawback can actually recover funds even if
/// the contributor later becomes uncooperative.
///
/// Must be called by the contributor themselves (their own signature, via
/// `contributor.require_auth()`) while they are still willing to cooperate —
/// e.g. right after a milestone payout, or as a standing authorization tied
/// to accepting a grant that carries clawback risk. This is the real-world
/// mechanism `execute` relies on: `approve` requires the contributor's
/// signature now; the later `transfer_from` in `execute` only requires the
/// *contract's own* authorization (auto-satisfied for calls the contract
/// makes as itself), not a fresh signature from the (possibly unwilling)
/// contributor.
pub fn authorize_pull(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    token: &Address,
    amount: i128,
    live_until_ledger: u32,
) -> Result<(), ContractError> {
    contributor.require_auth();

    if amount <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }

    token::Client::new(env, token).approve(
        contributor,
        &env.current_contract_address(),
        &amount,
        &live_until_ledger,
    );

    Events::emit_clawback_allowance_authorized(
        env,
        grant_id,
        contributor.clone(),
        token.clone(),
        amount,
        live_until_ledger,
    );

    Ok(())
}

/// Execute an approved clawback after the dispute window.
pub fn execute(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<i128, ContractError> {
    caller.require_auth();

    let mut clawback =
        Storage::get_clawback(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;

    // Check status - must be Approved
    if clawback.status != ClawbackStatus::Approved {
        return Err(ContractError::InvalidState);
    }

    // Check dispute window has passed
    let now = env.ledger().timestamp();
    if now <= clawback.dispute_window_ends {
        return Err(ContractError::DeadlinePassed);
    }

    // Verify milestone is still in Paid state
    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;

    if milestone.state != MilestoneState::Paid {
        return Err(ContractError::InvalidState);
    }

    // Get treasury address
    let treasury = Storage::get_treasury(env).ok_or(ContractError::TreasuryNotConfigured)?;

    // Pull funds from the contributor via a pre-authorized SEP-41 allowance
    // (see `authorize_pull`) instead of a plain `transfer`, which requires
    // the contributor's live signature — something an uncooperative target
    // will never provide. `transfer_from` only requires *our own* contract
    // to authorize itself as spender, which is automatic for calls the
    // contract makes as itself.
    let token_client = token::Client::new(env, &clawback.token);
    let allowance = token_client.allowance(&clawback.target, &env.current_contract_address());
    if allowance < clawback.amount {
        return Err(ContractError::InsufficientClawbackAllowance);
    }
    token_client.transfer_from(
        &env.current_contract_address(),
        &clawback.target,
        &treasury,
        &clawback.amount,
    );

    // Update status
    clawback.status = ClawbackStatus::Executed;
    Storage::set_clawback(env, grant_id, milestone_idx, &clawback);

    Events::emit_clawback_executed(
        env,
        grant_id,
        milestone_idx,
        clawback.amount,
        clawback.token.clone(),
        treasury,
    );

    Ok(clawback.amount)
}

/// Cancel a clawback (if not yet executed).
pub fn cancel(
    env: &Env,
    admin: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    admin.require_auth();

    // Must be ProtocolAdmin or DisputeArbiter
    let is_protocol_admin = has_role(env, admin, Role::ProtocolAdmin);
    let is_dispute_arbiter = has_role(env, admin, Role::DisputeArbiter);

    if !is_protocol_admin && !is_dispute_arbiter {
        return Err(ContractError::Unauthorized);
    }

    let mut clawback =
        Storage::get_clawback(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;

    // Cannot cancel if already executed
    if clawback.status == ClawbackStatus::Executed {
        return Err(ContractError::InvalidState);
    }

    // Update status
    clawback.status = ClawbackStatus::Cancelled;
    Storage::set_clawback(env, grant_id, milestone_idx, &clawback);

    Events::emit_clawback_cancelled(env, grant_id, milestone_idx, admin.clone());

    Ok(())
}

/// Return the clawback request.
pub fn get_request(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<ClawbackRequest> {
    Storage::get_clawback(env, grant_id, milestone_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_control::grant_role;
    use crate::types::{Grant, Milestone};
    use crate::{StellarGrantsContract, StellarGrantsContractClient};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token, Env, Vec};

    /// Register the contract and return its address so tests can wrap
    /// module-level calls in `env.as_contract(&contract_id, || { ... })` —
    /// required because this soroban-sdk version rejects storage access
    /// outside of a contract execution context.
    ///
    /// Each individual authorized call (anything that ends up calling
    /// `Address::require_auth()`) must live in its *own* `as_contract`
    /// block: under `mock_all_auths_allowing_non_root_auth`, calling
    /// `require_auth()` twice for the same address within one block trips
    /// "frame is already authorized", since direct Rust calls don't create
    /// the separate invocation frames a real dispatched contract call would.
    fn register(env: &Env) -> Address {
        env.register(StellarGrantsContract, ())
    }

    /// Registers a real SEP-41 token — must be called *outside* any
    /// `env.as_contract(..)` block, since deploying a new contract while
    /// nested inside another contract's frame fails auth setup.
    fn register_token(env: &Env) -> Address {
        let token_admin = Address::generate(env);
        env.register_stellar_asset_contract_v2(token_admin)
            .address()
    }

    /// `grant_role`'s own authorization check requires the granter to
    /// already hold a `SuperAdmin` `RoleAssignment` in the RBAC system —
    /// `Storage::set_global_admin` is a separate, unrelated concept and does
    /// not grant that role. Seed it directly, mirroring how
    /// `access_control.rs`'s own tests bootstrap a SuperAdmin.
    fn bootstrap_super_admin(env: &Env, admin: &Address) {
        let assignment = crate::types::RoleAssignment {
            holder: admin.clone(),
            role: Role::SuperAdmin,
            granted_by: admin.clone(),
            granted_at: 0,
            expires_at: None,
            is_active: true,
        };
        Storage::set_role_assignment(env, admin, &Role::SuperAdmin, &assignment);
        Storage::set_role_members(
            env,
            &Role::SuperAdmin,
            &soroban_sdk::vec![env, admin.clone()],
        );
    }

    fn setup_grant(env: &Env, owner: Address, token: Address) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Test"),
            token: token.clone(),
            status: crate::types::GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 2,
            milestones_paid_out: 1,
            escrow_balance: 500,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        }
    }

    fn setup_milestone(env: &Env, idx: u32, amount: i128) -> Milestone {
        Milestone {
            idx,
            description: String::from_str(env, "Milestone"),
            amount,
            state: MilestoneState::Paid,
            votes: soroban_sdk::Map::new(env),
            approvals: 1,
            rejections: 0,
            reasons: soroban_sdk::Map::new(env),
            status_updated_at: env.ledger().timestamp(),
            proof_url: None,
            submission_timestamp: env.ledger().timestamp(),
            deadline: None,
            reviewer_count_snapshot: 0,
        }
    }

    /// Bootstraps admin/protocol_admin/arbiter roles and seeds a grant +
    /// Paid milestone at grant_id=1/milestone_idx=0. Returns the addresses
    /// used, so callers can drive `initiate`/`approve`/etc. themselves, each
    /// in its own `as_contract` block.
    #[allow(clippy::too_many_arguments)]
    fn seed(
        env: &Env,
        contract_id: &Address,
        owner: &Address,
        token: &Address,
        admin: &Address,
        protocol_admin: &Address,
        arbiter: &Address,
        set_treasury: bool,
    ) {
        env.as_contract(contract_id, || {
            Storage::set_global_admin(env, admin);
            bootstrap_super_admin(env, admin);
            if set_treasury {
                Storage::set_treasury(env, admin);
            }
            let grant = setup_grant(env, owner.clone(), token.clone());
            Storage::set_grant(env, 1, &grant);
            let milestone = setup_milestone(env, 0, 500);
            Storage::set_milestone(env, 1, 0, &milestone);
            // `dispute()` calls into `escrow::lock`, which requires an
            // escrow account to already exist for the grant.
            crate::escrow::open(env, 1, owner, token).unwrap();
        });
        env.as_contract(contract_id, || {
            grant_role(env, admin, protocol_admin, Role::ProtocolAdmin, None).unwrap();
        });
        env.as_contract(contract_id, || {
            grant_role(env, admin, arbiter, Role::DisputeArbiter, None).unwrap();
        });
    }

    #[test]
    fn test_initiate_clawback_success() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);

        let admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &Address::generate(&env),
            &arbiter,
            false,
        );

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Plagiarism detected");
            let result = initiate(&env, &arbiter, 1, 0, reason);
            assert!(result.is_ok());

            let clawback = get_request(&env, 1, 0);
            assert!(clawback.is_some());
            assert_eq!(clawback.unwrap().status, ClawbackStatus::Pending);
        });
    }

    #[test]
    fn test_initiate_unauthorized() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);

        env.as_contract(&contract_id, || {
            let stranger = Address::generate(&env);
            let owner = Address::generate(&env);
            let token = Address::generate(&env);

            let grant = setup_grant(&env, owner, token);
            Storage::set_grant(&env, 1, &grant);

            let milestone = setup_milestone(&env, 0, 500);
            Storage::set_milestone(&env, 1, 0, &milestone);

            let reason = String::from_str(&env, "Test");
            let result = initiate(&env, &stranger, 1, 0, reason);
            assert_eq!(result, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn test_approve_clawback_success() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &protocol_admin,
            &arbiter,
            false,
        );

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });

        // First approval
        env.as_contract(&contract_id, || {
            approve(&env, &protocol_admin, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            let clawback = get_request(&env, 1, 0).unwrap();
            assert_eq!(clawback.status, ClawbackStatus::Pending);
            assert_eq!(clawback.approvals.len(), 1);
        });

        // Second approval - should change to Approved
        env.as_contract(&contract_id, || {
            approve(&env, &arbiter, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            let clawback = get_request(&env, 1, 0).unwrap();
            assert_eq!(clawback.status, ClawbackStatus::Approved);
            assert_eq!(clawback.approvals.len(), 2);
        });
    }

    #[test]
    fn test_dispute_clawback_success() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &protocol_admin,
            &arbiter,
            true,
        );

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &protocol_admin, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &arbiter, 1, 0).unwrap();
        });

        // Contributor disputes
        env.as_contract(&contract_id, || {
            let result = dispute(&env, &owner, 1, 0);
            assert!(result.is_ok());

            let clawback = get_request(&env, 1, 0).unwrap();
            assert_eq!(clawback.status, ClawbackStatus::DisputedByContributor);
        });
    }

    #[test]
    fn test_dispute_after_window_fails() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &protocol_admin,
            &arbiter,
            true,
        );

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &protocol_admin, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &arbiter, 1, 0).unwrap();
        });

        // Advance time past dispute window
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + CLAWBACK_DISPUTE_WINDOW_SECONDS + 1);

        // Contributor disputes - should fail
        env.as_contract(&contract_id, || {
            let result = dispute(&env, &owner, 1, 0);
            assert_eq!(result, Err(ContractError::DeadlinePassed));
        });
    }

    #[test]
    fn test_execute_before_window_fails() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &protocol_admin,
            &arbiter,
            true,
        );

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &protocol_admin, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &arbiter, 1, 0).unwrap();
        });

        // Try to execute before window ends - should fail regardless of
        // allowance state.
        env.as_contract(&contract_id, || {
            let result = execute(&env, &admin, 1, 0);
            assert_eq!(result, Err(ContractError::DeadlinePassed));
        });
    }

    #[test]
    fn test_execute_fails_without_allowance() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);
        let token = register_token(&env);

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &protocol_admin,
            &arbiter,
            true,
        );

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &protocol_admin, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &arbiter, 1, 0).unwrap();
        });

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + CLAWBACK_DISPUTE_WINDOW_SECONDS + 1);

        // No authorize_pull was ever called — a clean error, not a
        // token-contract panic.
        env.as_contract(&contract_id, || {
            let result = execute(&env, &admin, 1, 0);
            assert_eq!(result, Err(ContractError::InsufficientClawbackAllowance));
        });
    }

    #[test]
    fn test_execute_success() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);
        let token = register_token(&env);

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &protocol_admin,
            &arbiter,
            true,
        );

        token::StellarAssetClient::new(&env, &token).mint(&owner, &500);
        env.as_contract(&contract_id, || {
            authorize_pull(&env, &owner, 1, &token, 500, env.ledger().sequence() + 1000).unwrap();
        });

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &protocol_admin, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &arbiter, 1, 0).unwrap();
        });

        // Advance time past dispute window
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + CLAWBACK_DISPUTE_WINDOW_SECONDS + 1);

        // Execute should succeed
        env.as_contract(&contract_id, || {
            let result = execute(&env, &admin, 1, 0);
            assert!(result.is_ok());

            let clawback = get_request(&env, 1, 0).unwrap();
            assert_eq!(clawback.status, ClawbackStatus::Executed);
        });
    }

    #[test]
    fn test_cancel_success() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);

        let admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        env.as_contract(&contract_id, || {
            Storage::set_global_admin(&env, &admin);
            bootstrap_super_admin(&env, &admin);
            let grant = setup_grant(&env, owner, token);
            Storage::set_grant(&env, 1, &grant);
            let milestone = setup_milestone(&env, 0, 500);
            Storage::set_milestone(&env, 1, 0, &milestone);
        });
        env.as_contract(&contract_id, || {
            grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();
        });

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });

        // Cancel should succeed
        env.as_contract(&contract_id, || {
            let result = cancel(&env, &arbiter, 1, 0);
            assert!(result.is_ok());

            let clawback = get_request(&env, 1, 0).unwrap();
            assert_eq!(clawback.status, ClawbackStatus::Cancelled);
        });
    }

    #[test]
    fn test_cancel_after_execute_fails() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);
        let token = register_token(&env);

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);

        seed(
            &env,
            &contract_id,
            &owner,
            &token,
            &admin,
            &protocol_admin,
            &arbiter,
            true,
        );

        token::StellarAssetClient::new(&env, &token).mint(&owner, &500);
        env.as_contract(&contract_id, || {
            authorize_pull(&env, &owner, 1, &token, 500, env.ledger().sequence() + 1000).unwrap();
        });

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test");
            initiate(&env, &arbiter, 1, 0, reason).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &protocol_admin, 1, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, &arbiter, 1, 0).unwrap();
        });

        // Advance time past dispute window
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + CLAWBACK_DISPUTE_WINDOW_SECONDS + 1);

        env.as_contract(&contract_id, || {
            execute(&env, &admin, 1, 0).unwrap();
        });

        // Cancel after execute should fail
        env.as_contract(&contract_id, || {
            let result = cancel(&env, &arbiter, 1, 0);
            assert_eq!(result, Err(ContractError::InvalidState));
        });
    }

    #[test]
    fn test_authorize_pull_rejects_non_owner() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);
        let token = register_token(&env);

        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let grant = setup_grant(&env, owner, token.clone());
            Storage::set_grant(&env, 1, &grant);
        });

        env.as_contract(&contract_id, || {
            let result = authorize_pull(&env, &stranger, 1, &token, 500, u32::MAX);
            assert_eq!(result, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn test_authorize_pull_rejects_non_positive_amount() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = register(&env);
        let token = register_token(&env);

        let owner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let grant = setup_grant(&env, owner.clone(), token.clone());
            Storage::set_grant(&env, 1, &grant);
        });

        env.as_contract(&contract_id, || {
            let result = authorize_pull(&env, &owner, 1, &token, 0, u32::MAX);
            assert_eq!(result, Err(ContractError::InvalidInput));
        });
    }

    /// The key regression test for #685: proves `execute` moves funds
    /// without ever requiring the target's own signature — demonstrating a
    /// truly unwilling contributor can't block recovery once they've
    /// pre-authorized the pull. Drives real entry points through the
    /// generated contract client under `mock_all_auths_allowing_non_root_auth`
    /// (which records, rather than blindly satisfies, every `require_auth`
    /// call), then inspects `env.auths()` after the `execute` call to assert
    /// the target's address never appears — the thing `mock_all_auths()`
    /// alone would silently paper over.
    #[test]
    fn test_execute_succeeds_via_preauthorized_allowance_without_target_signature() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let treasury = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = asset.address();
        token::StellarAssetClient::new(&env, &token_id).mint(&owner, &1_000);

        // Seed grant/milestone/an already-Approved, past-window clawback
        // record directly — the initiate/approve authorization flow is
        // already covered by the tests above; this test isolates the
        // execute-time authorization gap #685 is about.
        env.as_contract(&contract_id, || {
            Storage::set_treasury(&env, &treasury);

            let grant = setup_grant(&env, owner.clone(), token_id.clone());
            Storage::set_grant(&env, 1, &grant);

            let milestone = setup_milestone(&env, 0, 500);
            Storage::set_milestone(&env, 1, 0, &milestone);

            let now = env.ledger().timestamp();
            let clawback = ClawbackRequest {
                grant_id: 1,
                milestone_idx: 0,
                target: owner.clone(),
                amount: 500,
                token: token_id.clone(),
                reason: String::from_str(&env, "test"),
                initiated_by: admin.clone(),
                initiated_at: now,
                dispute_window_ends: now,
                approvals: Vec::new(&env),
                required_approvals: 2,
                status: ClawbackStatus::Approved,
            };
            Storage::set_clawback(&env, 1, 0, &clawback);
        });
        env.ledger().with_mut(|l| l.timestamp += 1);

        // The contributor pre-authorizes the pull while still cooperative.
        client.clawback_authorize_pull(
            &owner,
            &1,
            &token_id,
            &500,
            &(env.ledger().sequence() + 1000),
        );

        // Execute the clawback — only `admin` drives this call.
        client.clawback_execute(&admin, &1, &0);

        // The decisive assertion: whatever addresses had to authorize this
        // specific `execute` call, the target (`owner`) is not among them.
        // Under the old `transfer`-based implementation this call would
        // have panicked (the token contract's own `from.require_auth()` for
        // the target has no signature to satisfy); under the fix it
        // succeeds via `transfer_from`, which only needs the contract's own
        // (automatic) authorization as spender.
        let authorized = env.auths();
        assert!(authorized.iter().any(|(addr, _)| addr == &admin));
        assert!(!authorized.iter().any(|(addr, _)| addr == &owner));

        let token_client = token::Client::new(&env, &token_id);
        assert_eq!(token_client.balance(&treasury), 500);
        assert_eq!(token_client.balance(&owner), 500);

        env.as_contract(&contract_id, || {
            assert_eq!(
                get_request(&env, 1, 0).unwrap().status,
                ClawbackStatus::Executed
            );
        });
    }
}
