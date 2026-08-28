use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, GrantStatus, SplitRecipient};
use soroban_sdk::{Address, Env, Vec};

pub fn register_split(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
    recipients: Vec<SplitRecipient>,
) -> Result<(), ContractError> {
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }

    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }

    if recipients.is_empty() {
        return Err(ContractError::InvalidInput);
    }

    let mut total_basis_points: u32 = 0;
    for r in recipients.iter() {
        if r.basis_points == 0 {
            return Err(ContractError::InvalidInput);
        }
        total_basis_points = total_basis_points
            .checked_add(r.basis_points)
            .ok_or(ContractError::InvalidInput)?;
    }

    if total_basis_points != 10000 {
        return Err(ContractError::InvalidInput);
    }

    Storage::set_split_recipients(env, grant_id, milestone_idx, &recipients);
    Events::emit_split_registered(env, grant_id, milestone_idx, recipients.len());

    Ok(())
}
