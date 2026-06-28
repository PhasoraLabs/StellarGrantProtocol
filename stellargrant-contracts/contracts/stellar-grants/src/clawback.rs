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

    let mut clawback = Storage::get_clawback(env, grant_id, milestone_idx)
        .ok_or(ContractError::InvalidState)?;

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

    let mut clawback = Storage::get_clawback(env, grant_id, milestone_idx)
        .ok_or(ContractError::InvalidState)?;

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

/// Execute an approved clawback after the dispute window.
pub fn execute(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<i128, ContractError> {
    caller.require_auth();

    let mut clawback = Storage::get_clawback(env, grant_id, milestone_idx)
        .ok_or(ContractError::InvalidState)?;

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

    // Transfer funds from contributor to treasury
    // Note: This assumes the contract holds the funds or has authorization
    // In practice, this may require a different mechanism depending on token setup
    let token_client = token::Client::new(env, &clawback.token);
    token_client.transfer(&clawback.target, &treasury, &clawback.amount);

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

    let mut clawback = Storage::get_clawback(env, grant_id, milestone_idx)
        .ok_or(ContractError::InvalidState)?;

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
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Env, Vec};

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
        }
    }

    #[test]
    fn test_initiate_clawback_success() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner.clone(), token.clone());
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Plagiarism detected");
        let result = initiate(&env, &arbiter, 1, 0, reason);
        assert!(result.is_ok());

        let clawback = get_request(&env, 1, 0);
        assert!(clawback.is_some());
        assert_eq!(clawback.unwrap().status, ClawbackStatus::Pending);
    }

    #[test]
    fn test_initiate_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

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
    }

    #[test]
    fn test_approve_clawback_success() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        grant_role(&env, &admin, &protocol_admin, Role::ProtocolAdmin, None).unwrap();
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner, token);
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Test");
        initiate(&env, &arbiter, 1, 0, reason).unwrap();

        // First approval
        approve(&env, &protocol_admin, 1, 0).unwrap();
        let clawback = get_request(&env, 1, 0).unwrap();
        assert_eq!(clawback.status, ClawbackStatus::Pending);
        assert_eq!(clawback.approvals.len(), 1);

        // Second approval - should change to Approved
        approve(&env, &arbiter, 1, 0).unwrap();
        let clawback = get_request(&env, 1, 0).unwrap();
        assert_eq!(clawback.status, ClawbackStatus::Approved);
        assert_eq!(clawback.approvals.len(), 2);
    }

    #[test]
    fn test_dispute_clawback_success() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        Storage::set_treasury(&env, &admin);
        grant_role(&env, &admin, &protocol_admin, Role::ProtocolAdmin, None).unwrap();
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner.clone(), token);
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Test");
        initiate(&env, &arbiter, 1, 0, reason).unwrap();
        approve(&env, &protocol_admin, 1, 0).unwrap();
        approve(&env, &arbiter, 1, 0).unwrap();

        // Contributor disputes
        let result = dispute(&env, &owner, 1, 0);
        assert!(result.is_ok());

        let clawback = get_request(&env, 1, 0).unwrap();
        assert_eq!(clawback.status, ClawbackStatus::DisputedByContributor);
    }

    #[test]
    fn test_dispute_after_window_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        Storage::set_treasury(&env, &admin);
        grant_role(&env, &admin, &protocol_admin, Role::ProtocolAdmin, None).unwrap();
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner.clone(), token);
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Test");
        initiate(&env, &arbiter, 1, 0, reason).unwrap();
        approve(&env, &protocol_admin, 1, 0).unwrap();
        approve(&env, &arbiter, 1, 0).unwrap();

        // Advance time past dispute window
        env.ledger().set_timestamp(
            env.ledger().timestamp() + CLAWBACK_DISPUTE_WINDOW_SECONDS + 1,
        );

        // Contributor disputes - should fail
        let result = dispute(&env, &owner, 1, 0);
        assert_eq!(result, Err(ContractError::DeadlinePassed));
    }

    #[test]
    fn test_execute_before_window_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        Storage::set_treasury(&env, &admin);
        grant_role(&env, &admin, &protocol_admin, Role::ProtocolAdmin, None).unwrap();
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner, token);
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Test");
        initiate(&env, &arbiter, 1, 0, reason).unwrap();
        approve(&env, &protocol_admin, 1, 0).unwrap();
        approve(&env, &arbiter, 1, 0).unwrap();

        // Try to execute before window ends - should fail
        let result = execute(&env, &admin, 1, 0);
        assert_eq!(result, Err(ContractError::DeadlinePassed));
    }

    #[test]
    fn test_execute_success() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        Storage::set_treasury(&env, &admin);
        grant_role(&env, &admin, &protocol_admin, Role::ProtocolAdmin, None).unwrap();
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner, token);
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Test");
        initiate(&env, &arbiter, 1, 0, reason).unwrap();
        approve(&env, &protocol_admin, 1, 0).unwrap();
        approve(&env, &arbiter, 1, 0).unwrap();

        // Advance time past dispute window
        env.ledger().set_timestamp(
            env.ledger().timestamp() + CLAWBACK_DISPUTE_WINDOW_SECONDS + 1,
        );

        // Execute should succeed
        let result = execute(&env, &admin, 1, 0);
        assert!(result.is_ok());

        let clawback = get_request(&env, 1, 0).unwrap();
        assert_eq!(clawback.status, ClawbackStatus::Executed);
    }

    #[test]
    fn test_cancel_success() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner, token);
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Test");
        initiate(&env, &arbiter, 1, 0, reason).unwrap();

        // Cancel should succeed
        let result = cancel(&env, &arbiter, 1, 0);
        assert!(result.is_ok());

        let clawback = get_request(&env, 1, 0).unwrap();
        assert_eq!(clawback.status, ClawbackStatus::Cancelled);
    }

    #[test]
    fn test_cancel_after_execute_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let protocol_admin = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        Storage::set_global_admin(&env, &admin);
        Storage::set_treasury(&env, &admin);
        grant_role(&env, &admin, &protocol_admin, Role::ProtocolAdmin, None).unwrap();
        grant_role(&env, &admin, &arbiter, Role::DisputeArbiter, None).unwrap();

        let grant = setup_grant(&env, owner, token);
        Storage::set_grant(&env, 1, &grant);

        let milestone = setup_milestone(&env, 0, 500);
        Storage::set_milestone(&env, 1, 0, &milestone);

        let reason = String::from_str(&env, "Test");
        initiate(&env, &arbiter, 1, 0, reason).unwrap();
        approve(&env, &protocol_admin, 1, 0).unwrap();
        approve(&env, &arbiter, 1, 0).unwrap();

        // Advance time past dispute window
        env.ledger().set_timestamp(
            env.ledger().timestamp() + CLAWBACK_DISPUTE_WINDOW_SECONDS + 1,
        );

        execute(&env, &admin, 1, 0).unwrap();

        // Cancel after execute should fail
        let result = cancel(&env, &arbiter, 1, 0);
        assert_eq!(result, Err(ContractError::InvalidState));
    }
}
