use crate::storage::Storage;
use crate::types::{ContractError, GrantStatus, TransferProposal, TransferableRole};
use soroban_sdk::{Address, Env};

/// Propose a two-step ownership or reviewer-role transfer for a grant.
/// For Owner transfers, the caller must be the current owner.
/// For Reviewer transfers, the caller must be the owner OR the reviewer being replaced,
/// and `reviewer_to_replace` must currently be in the grant's reviewer list.
pub fn propose_transfer(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    new_holder: Address,
    role: TransferableRole,
    reviewer_to_replace: Option<Address>,
) -> Result<(), ContractError> {
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }

    match role {
        TransferableRole::Owner => {
            if grant.owner != *caller {
                return Err(ContractError::Unauthorized);
            }
        }
        TransferableRole::Reviewer => {
            let to_replace = reviewer_to_replace
                .as_ref()
                .ok_or(ContractError::InvalidInput)?;
            if !grant.reviewers.contains(to_replace.clone()) {
                return Err(ContractError::Unauthorized);
            }
            if grant.owner != *caller && to_replace != caller {
                return Err(ContractError::Unauthorized);
            }
        }
    }

    let proposal = TransferProposal {
        grant_id,
        current_holder: caller.clone(),
        proposed_new_holder: new_holder,
        role,
        reviewer_to_replace,
        proposed_at: env.ledger().timestamp(),
    };
    Storage::set_transfer_proposal(env, grant_id, &proposal);
    Ok(())
}

/// Accept a pending transfer proposal. The caller must be the proposed new holder.
/// On acceptance the grant state is updated and the proposal is cleared.
pub fn accept_transfer(
    env: &Env,
    new_holder: &Address,
    grant_id: u64,
) -> Result<(), ContractError> {
    new_holder.require_auth();

    let proposal =
        Storage::get_transfer_proposal(env, grant_id).ok_or(ContractError::InvalidState)?;

    if proposal.proposed_new_holder != *new_holder {
        return Err(ContractError::Unauthorized);
    }

    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    match proposal.role {
        TransferableRole::Owner => {
            grant.owner = new_holder.clone();
        }
        TransferableRole::Reviewer => {
            let to_replace = proposal
                .reviewer_to_replace
                .ok_or(ContractError::InvalidInput)?;
            for i in 0..grant.reviewers.len() {
                if grant.reviewers.get(i).unwrap() == to_replace {
                    grant.reviewers.set(i, new_holder.clone());
                    break;
                }
            }
        }
    }

    Storage::set_grant(env, grant_id, &grant);
    Storage::remove_transfer_proposal(env, grant_id);
    Ok(())
}

pub fn get_transfer_proposal(env: &Env, grant_id: u64) -> Option<TransferProposal> {
    Storage::get_transfer_proposal(env, grant_id)
}
