use crate::escrow;
use crate::storage::Storage;
use crate::types::{ContractError, PaymentSplit, SplitRecipient};
use soroban_sdk::{Address, Env, Vec};

/// Register a payment split for a milestone. Must be called before grant completion.
/// The sum of all recipient share_bps must equal 10 000 (100%).
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
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }
    if recipients.is_empty() {
        return Err(ContractError::InvalidInput);
    }

    let mut total_bps: u32 = 0;
    for r in recipients.iter() {
        total_bps = total_bps.saturating_add(r.share_bps);
    }
    if total_bps != 10_000 {
        return Err(ContractError::InvalidInput);
    }

    let split = PaymentSplit {
        grant_id,
        milestone_idx,
        recipients,
        registered_by: caller.clone(),
        registered_at: env.ledger().timestamp(),
    };
    Storage::set_payment_split(env, grant_id, milestone_idx, &split);
    Ok(())
}

pub fn has_split(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    Storage::get_payment_split(env, grant_id, milestone_idx).is_some()
}

/// Distribute `total_amount` among the registered recipients proportionally.
/// Called internally from the payout path when a split is registered.
/// The last recipient receives the remainder to prevent integer truncation loss.
pub fn execute_split(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    total_amount: i128,
) -> Result<(), ContractError> {
    let split = Storage::get_payment_split(env, grant_id, milestone_idx)
        .ok_or(ContractError::InvalidState)?;

    let recipients_len = split.recipients.len();
    let mut distributed: i128 = 0;

    for (i, r) in split.recipients.iter().enumerate() {
        let is_last = (i as u32) + 1 == recipients_len;
        let share = if is_last {
            total_amount - distributed
        } else {
            total_amount
                .saturating_mul(r.share_bps as i128)
                .checked_div(10_000)
                .unwrap_or(0)
        };
        if share > 0 {
            escrow::release(env, grant_id, &r.recipient, share)?;
            distributed += share;
        }
    }
    Ok(())
}

pub fn get_split(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<PaymentSplit> {
    Storage::get_payment_split(env, grant_id, milestone_idx)
}
