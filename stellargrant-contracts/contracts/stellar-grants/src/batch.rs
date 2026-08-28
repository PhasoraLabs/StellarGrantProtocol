use crate::events::Events;
use crate::reentrancy;
use crate::storage::Storage;
use crate::types::{ContractError, GrantStatus};
use soroban_sdk::{token, Address, Env, String};

pub fn try_cancel_grant(
    env: &Env,
    grant_id: u64,
    caller: &Address,
    reason: String,
) -> Result<(), ContractError> {
    caller.require_auth();
    reentrancy::with_non_reentrant(env, || {
        let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

        let caller_is_owner = grant.owner == *caller;
        let caller_is_admin = Storage::get_global_admin(env) == Some(caller.clone());
        if !caller_is_owner && !caller_is_admin {
            return Err(ContractError::Unauthorized);
        }

        if grant.status != GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        if grant.milestones_paid_out >= grant.total_milestones {
            return Err(ContractError::InvalidState);
        }

        let total_refundable = grant.escrow_balance;
        if total_refundable > 0 {
            let mut total_contributions: i128 = 0;
            for fund_entry in grant.funders.iter() {
                total_contributions = total_contributions
                    .checked_add(fund_entry.amount)
                    .ok_or(ContractError::InvalidInput)?;
            }

            if total_contributions <= 0 {
                return Err(ContractError::InvalidInput);
            }

            let token_client = token::Client::new(env, &grant.token);
            let funders_len = grant.funders.len();
            let mut distributed = 0i128;

            for i in 0..funders_len {
                let fund_entry = grant.funders.get(i).unwrap();
                let is_last = i + 1 == funders_len;
                let refund_amount = if is_last {
                    total_refundable - distributed
                } else {
                    let amount = fund_entry
                        .amount
                        .checked_mul(total_refundable)
                        .ok_or(ContractError::InvalidInput)?
                        .checked_div(total_contributions)
                        .ok_or(ContractError::InvalidInput)?;
                    distributed += amount;
                    amount
                };

                if refund_amount > 0 {
                    token_client.transfer(
                        &env.current_contract_address(),
                        &fund_entry.funder,
                        &refund_amount,
                    );
                    Events::emit_refund_issued(
                        env,
                        grant_id,
                        fund_entry.funder.clone(),
                        refund_amount,
                    );
                }
            }
        }

        grant.status = GrantStatus::Cancelled;
        grant.escrow_balance = 0;
        grant.reason = Some(reason.clone());
        grant.timestamp = env.ledger().timestamp();

        Storage::set_grant(env, grant_id, &grant);
        Events::emit_grant_cancelled(env, grant_id, caller.clone(), reason, total_refundable);

        Ok(())
    })
}
