use soroban_sdk::{Address, Env, Vec};
use crate::types::{ContractError, EscrowReleaseRequest, EscrowReleaseApproval, ProtocolConfig};
use crate::storage::keys::DataKey;

const SECONDS_PER_WEEK: u64 = 604800;

pub fn create_request(env: &Env, grant_id: u64, milestone_idx: u32, amount: i128, recipient: Address) -> Result<(), ContractError> {
    let key = DataKey::EscrowReleaseRequest(grant_id, milestone_idx);
    let expires_at = env.ledger().timestamp() + (SECONDS_PER_WEEK * 2);
    let request = EscrowReleaseRequest {
        grant_id,
        milestone_idx,
        amount,
        recipient,
        approvals: Vec::new(env),
        expires_at,
        executed: false,
    };
    env.storage().persistent().set(&key, &request);
    Ok(())
}

pub fn approve(env: &Env, approver: Address, grant_id: u64, milestone_idx: u32) -> Result<(), ContractError> {
    approver.require_auth();
    // Use ContractError::NotFound, but wait, types.rs has ContractError::GrantNotFound or similar? We can just use ContractError::InvalidState or whatever.
    // I'll use ContractError::InvalidState if not found since there is no EscrowRequestNotFound.
    let mut request = get_request(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;
    
    if request.executed {
        return Err(ContractError::InvalidState);
    }
    if env.ledger().timestamp() > request.expires_at {
        return Err(ContractError::InvalidState);
    }
    
    for approval in request.approvals.iter() {
        if approval.approver == approver {
            return Err(ContractError::AlreadyVoted);
        }
    }
    
    request.approvals.push_back(EscrowReleaseApproval {
        approver: approver.clone(),
        timestamp: env.ledger().timestamp(),
    });
    
    env.storage().persistent().set(&DataKey::EscrowReleaseRequest(grant_id, milestone_idx), &request);
    Ok(())
}

pub fn is_approved(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    if let Some(request) = get_request(env, grant_id, milestone_idx) {
        if let Some(config) = env.storage().persistent().get::<DataKey, ProtocolConfig>(&DataKey::Config) {
            let threshold = config.multisig_escrow_threshold;
            return request.approvals.len() >= threshold;
        }
    }
    false
}

pub fn execute_release(env: &Env, grant_id: u64, milestone_idx: u32) -> Result<(), ContractError> {
    let mut request = get_request(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;
    
    if request.executed {
        return Err(ContractError::InvalidState);
    }
    if env.ledger().timestamp() > request.expires_at {
        return Err(ContractError::InvalidState);
    }
    
    if !is_approved(env, grant_id, milestone_idx) {
        return Err(ContractError::Unauthorized);
    }
    
    request.executed = true;
    env.storage().persistent().set(&DataKey::EscrowReleaseRequest(grant_id, milestone_idx), &request);
    
    crate::escrow::release(env, grant_id, &request.recipient, request.amount)
}

pub fn get_request(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<EscrowReleaseRequest> {
    env.storage().persistent().get(&DataKey::EscrowReleaseRequest(grant_id, milestone_idx))
}
