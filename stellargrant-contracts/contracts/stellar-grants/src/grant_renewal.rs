use crate::storage::Storage;
use crate::types::{ContractError, GrantStatus, RenewalProposal, RenewalStatus};
use soroban_sdk::{Address, Env, String};

pub fn propose_renewal(
    env: &Env,
    proposer: &Address,
    original_grant_id: u64,
    new_title: String,
    new_description: String,
    new_total_amount: i128,
    new_num_milestones: u32,
    inherit_reviewers: bool,
    inherit_contributor: bool,
    ttl_ledgers: u32,
) -> Result<(), ContractError> {
    proposer.require_auth();

    let grant = Storage::get_grant_v(&env, original_grant_id);

    if grant.status != GrantStatus::Completed
        && grant.milestones_paid_out < grant.total_milestones - 1
    {
        return Err(ContractError::InvalidState);
    }

    let proposal = RenewalProposal {
        original_grant_id,
        proposed_by: proposer.clone(),
        new_title,
        new_description,
        new_total_amount,
        new_num_milestones,
        inherit_reviewers,
        inherit_contributor,
        status: RenewalStatus::Proposed,
        reviewer_votes: 0,
        proposed_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (ttl_ledgers as u64 * 5),
        new_grant_id: None,
    };

    Storage::set_renewal_proposal(&env, &proposal);
    Ok(())
}

pub fn approve_renewal(
    env: &Env,
    reviewer: &Address,
    original_grant_id: u64,
) -> Result<RenewalStatus, ContractError> {
    reviewer.require_auth();

    let mut proposal = Storage::get_renewal_proposal(&env, original_grant_id)
        .ok_or(ContractError::InvalidState)?;

    if proposal.status != RenewalStatus::Proposed {
        return Err(ContractError::InvalidState);
    }

    if env.ledger().timestamp() > proposal.expires_at {
        return Err(ContractError::InvalidState);
    }

    proposal.reviewer_votes += 1;
    proposal.status = RenewalStatus::ReviewerApproved;
    Storage::set_renewal_proposal(&env, &proposal);
    Ok(proposal.status)
}

pub fn activate_renewal(
    env: &Env,
    owner: &Address,
    original_grant_id: u64,
) -> Result<u64, ContractError> {
    owner.require_auth();

    let mut proposal = Storage::get_renewal_proposal(&env, original_grant_id)
        .ok_or(ContractError::InvalidState)?;

    if proposal.status != RenewalStatus::ReviewerApproved {
        return Err(ContractError::InvalidState);
    }

    let original_grant = Storage::get_grant_v(&env, original_grant_id);
    if original_grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    let new_grant_id = Storage::increment_grant_counter(&env);

    proposal.status = RenewalStatus::Active;
    proposal.new_grant_id = Some(new_grant_id);
    Storage::set_renewal_proposal(&env, &proposal);
    Storage::set_renewal_history(&env, new_grant_id, original_grant_id);

    Ok(new_grant_id)
}

pub fn decline_renewal(
    env: &Env,
    caller: &Address,
    original_grant_id: u64,
) -> Result<(), ContractError> {
    caller.require_auth();

    let mut proposal = Storage::get_renewal_proposal(&env, original_grant_id)
        .ok_or(ContractError::InvalidState)?;

    if proposal.status == RenewalStatus::Declined || proposal.status == RenewalStatus::Expired {
        return Err(ContractError::InvalidState);
    }

    let grant = Storage::get_grant_v(&env, original_grant_id);
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }

    proposal.status = RenewalStatus::Declined;
    Storage::set_renewal_proposal(&env, &proposal);
    Ok(())
}

pub fn get_proposal(env: &Env, original_grant_id: u64) -> Option<RenewalProposal> {
    Storage::get_renewal_proposal(&env, original_grant_id)
}

pub fn renewal_chain(env: &Env, original_grant_id: u64) -> soroban_sdk::Vec<u64> {
    let mut chain = soroban_sdk::Vec::new(env);
    chain.push_back(original_grant_id);

    let mut current_id = original_grant_id;
    loop {
        if let Some(proposal) = Storage::get_renewal_proposal(&env, current_id) {
            if let Some(new_id) = proposal.new_grant_id {
                chain.push_back(new_id);
                current_id = new_id;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    chain
}
