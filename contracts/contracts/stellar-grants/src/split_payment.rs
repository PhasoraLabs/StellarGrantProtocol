use crate::escrow;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, GrantStatus, PaymentSplit, SplitRecipient};
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
    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
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
        recipients: recipients.clone(),
        registered_by: caller.clone(),
        registered_at: env.ledger().timestamp(),
    };
    Storage::set_payment_split(env, grant_id, milestone_idx, &split);
    Storage::set_split_recipients(env, grant_id, milestone_idx, &recipients);
    Events::emit_split_registered(env, grant_id, milestone_idx, recipients.len());

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

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let token = grant.token.clone();

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
            // Emit PayeeReceipt for each split recipient
            Events::emit_payee_receipt(
                env,
                grant_id,
                r.recipient.clone(),
                token.clone(),
                share,
                Some(milestone_idx),
            );
        }
    }
    Ok(())
}

pub fn get_split(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<PaymentSplit> {
    Storage::get_payment_split(env, grant_id, milestone_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec, Env};

    fn setup_grant(
        env: &Env,
        owner: &Address,
        grant_id: u64,
        total_milestones: u32,
        status: GrantStatus,
    ) {
        use crate::types::Grant;
        use soroban_sdk::String;
        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Test"),
            token: Address::generate(env),
            status,
            total_amount: 0,
            milestone_amount: 0,
            reviewers: Vec::new(env),
            total_milestones,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);
    }

    #[test]
    fn test_register_split() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, 1, 3, GrantStatus::Active);

            let recipients = vec![
                &env,
                SplitRecipient {
                    recipient: r1.clone(),
                    share_bps: 6000,
                },
                SplitRecipient {
                    recipient: r2.clone(),
                    share_bps: 4000,
                },
            ];

            register_split(&env, &owner, 1, 0, recipients).unwrap();

            assert!(has_split(&env, 1, 0));
            let split = get_split(&env, 1, 0).unwrap();
            assert_eq!(split.recipients.len(), 2);
        });
    }

    #[test]
    fn test_register_split_inactive_grant() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let r1 = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, 1, 3, GrantStatus::Cancelled);

            let recipients = vec![
                &env,
                SplitRecipient {
                    recipient: r1,
                    share_bps: 10000,
                },
            ];

            let res = register_split(&env, &owner, 1, 0, recipients);
            assert_eq!(res, Err(ContractError::InvalidState));
        });
    }

    #[test]
    #[should_panic]
    fn test_register_split_bps_not_10000() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let r1 = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, 1, 1, GrantStatus::Active);

            let recipients = vec![
                &env,
                SplitRecipient {
                    recipient: r1,
                    share_bps: 5000,
                },
            ];

            register_split(&env, &owner, 1, 0, recipients).unwrap();
        });
    }

    #[test]
    #[should_panic]
    fn test_register_split_empty_recipients() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.mock_all_auths();
        let owner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, 1, 1, GrantStatus::Active);

            let recipients = Vec::new(&env);
            register_split(&env, &owner, 1, 0, recipients).unwrap();
        });
    }

    #[test]
    #[should_panic]
    fn test_register_split_milestone_out_of_bounds() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let r1 = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_grant(&env, &owner, 1, 1, GrantStatus::Active);

            let recipients = vec![
                &env,
                SplitRecipient {
                    recipient: r1,
                    share_bps: 10000,
                },
            ];

            register_split(&env, &owner, 1, 5, recipients).unwrap();
        });
    }

    #[test]
    fn test_has_split_returns_false_when_none() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            assert!(!has_split(&env, 999, 0));
        });
    }
}
