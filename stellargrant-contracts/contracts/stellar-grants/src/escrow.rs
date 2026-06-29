use soroban_sdk::{token, Address, Env};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{EscrowAccount, FunderLedger, GrantFund};

// ── Internal helpers ──────────────────────────────────────────────────────────

fn load_account(env: &Env, grant_id: u64) -> Result<EscrowAccount, ContractError> {
    Storage::get_escrow_account(env, grant_id).ok_or(ContractError::EscrowNotFound)
}

fn proportional_share(funder_net: i128, total_net: i128, total_balance: i128) -> i128 {
    if total_net == 0 || total_balance == 0 {
        return 0;
    }
    funder_net
        .saturating_mul(total_balance)
        .checked_div(total_net)
        .unwrap_or(0)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Initialize an escrow account for a grant. Called once on grant creation.
pub fn open(
    env: &Env,
    grant_id: u64,
    owner: &Address,
    token: &Address,
) -> Result<(), ContractError> {
    if Storage::get_escrow_account(env, grant_id).is_some() {
        return Err(ContractError::EscrowAlreadyOpen);
    }
    let account = EscrowAccount {
        owner: owner.clone(),
        token: token.clone(),
        balance: 0,
        total_deposited: 0,
        total_released: 0,
        locked: false,
    };
    Storage::set_escrow_account(env, grant_id, &account);
    Ok(())
}

/// Deposit funds from a funder into the grant's escrow.
pub fn deposit(
    env: &Env,
    grant_id: u64,
    funder: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;
    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let mut account = load_account(env, grant_id)?;
    let token_client = token::Client::new(env, &account.token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(funder, env.current_contract_address(), &amount);
        Ok(())
    })?;

    account.balance = account
        .balance
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;
    account.total_deposited = account
        .total_deposited
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_escrow_account(env, grant_id, &account);

    // Update FunderLedger.
    let mut ledger = Storage::get_funder_ledger(env, grant_id, funder).unwrap_or(FunderLedger {
        funder: funder.clone(),
        contributed: 0,
        refunded: 0,
        last_contribution_at: 0,
    });
    ledger.contributed = ledger
        .contributed
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;
    ledger.last_contribution_at = env.ledger().timestamp();
    Storage::set_funder_ledger(env, grant_id, funder, &ledger);

    // Track funder address in the list.
    let mut funders = Storage::get_escrow_funders_list(env, grant_id);
    if !funders.contains(funder.clone()) {
        funders.push_back(funder.clone());
        Storage::set_escrow_funders_list(env, grant_id, &funders);
    }

    // Issue #598: track grant in funder's index for report queries.
    Storage::push_funder_grant_index(env, funder, grant_id);

    // Mirror onto Grant struct for backward-compatible queries.
    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    grant.escrow_balance = account.balance;
    let mut funder_found = false;
    for i in 0..grant.funders.len() {
        let mut entry = grant.funders.get(i).unwrap();
        if entry.funder == *funder {
            entry.amount = entry
                .amount
                .checked_add(amount)
                .ok_or(ContractError::InvalidInput)?;
            grant.funders.set(i, entry);
            funder_found = true;
            break;
        }
    }
    if !funder_found {
        grant.funders.push_back(GrantFund {
            funder: funder.clone(),
            amount,
        });
    }
    Storage::set_grant(env, grant_id, &grant);

    Ok(())
}

/// Release `amount` from escrow to `recipient` (milestone payout).
pub fn release(
    env: &Env,
    grant_id: u64,
    recipient: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;
    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let mut account = load_account(env, grant_id)?;
    if account.locked {
        return Err(ContractError::EscrowLocked);
    }
    if account.balance < amount {
        return Err(ContractError::InvalidInput);
    }

    account.balance -= amount;
    account.total_released = account
        .total_released
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_escrow_account(env, grant_id, &account);

    if let Some(mut grant) = Storage::get_grant(env, grant_id) {
        grant.escrow_balance = account.balance;
        Storage::set_grant(env, grant_id, &grant);
    }

    let token_client = token::Client::new(env, &account.token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(&env.current_contract_address(), recipient, &amount);
        Ok(())
    })?;

    Ok(())
}

/// Refund remaining contribution from escrow back to a specific funder.
pub fn refund(env: &Env, grant_id: u64, funder: &Address) -> Result<i128, ContractError> {
    crate::reentrancy::protect(env)?;
    let mut ledger = Storage::get_funder_ledger(env, grant_id, funder)
        .ok_or(ContractError::NoRefundableAmount)?;

    let refundable = ledger.contributed.saturating_sub(ledger.refunded);
    if refundable <= 0 {
        return Err(ContractError::NoRefundableAmount);
    }

    let mut account = load_account(env, grant_id)?;
    let actual_refund = refundable.min(account.balance);
    if actual_refund <= 0 {
        return Err(ContractError::NoRefundableAmount);
    }

    account.balance -= actual_refund;
    Storage::set_escrow_account(env, grant_id, &account);

    ledger.refunded = ledger
        .refunded
        .checked_add(actual_refund)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_funder_ledger(env, grant_id, funder, &ledger);

    if let Some(mut grant) = Storage::get_grant(env, grant_id) {
        grant.escrow_balance = account.balance;
        Storage::set_grant(env, grant_id, &grant);
    }

    let token_client = token::Client::new(env, &account.token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(&env.current_contract_address(), funder, &actual_refund);
        Ok(())
    })?;

    Ok(actual_refund)
}

/// Refund all remaining escrow balance proportionally to all funders.
pub fn refund_all(env: &Env, grant_id: u64) -> Result<(), ContractError> {
    crate::reentrancy::protect(env)?;
    let mut account = load_account(env, grant_id)?;
    let total_balance = account.balance;
    if total_balance == 0 {
        return Ok(());
    }

    let funders = Storage::get_escrow_funders_list(env, grant_id);
    if funders.is_empty() {
        return Ok(());
    }

    let token_client = token::Client::new(env, &account.token);

    // Compute total net contributions for proportion denominator.
    let mut total_net: i128 = 0;
    for addr in funders.iter() {
        if let Some(l) = Storage::get_funder_ledger(env, grant_id, &addr) {
            total_net += l.contributed.saturating_sub(l.refunded);
        }
    }

    let funders_len = funders.len();
    let mut distributed: i128 = 0;

    for (i, addr) in funders.iter().enumerate() {
        let ledger_opt = Storage::get_funder_ledger(env, grant_id, &addr);
        let net = ledger_opt
            .as_ref()
            .map(|l| l.contributed.saturating_sub(l.refunded))
            .unwrap_or(0);

        if net <= 0 {
            continue;
        }

        let is_last = (i as u32) + 1 == funders_len;
        let refund_amount = if is_last {
            total_balance - distributed
        } else {
            proportional_share(net, total_net, total_balance)
        };

        if refund_amount > 0 {
            crate::reentrancy::protect_external_call(env, || {
                token_client.transfer(&env.current_contract_address(), &addr, &refund_amount);
                Ok(())
            })?;
            distributed += refund_amount;

            if let Some(mut ledger) = ledger_opt {
                ledger.refunded = ledger.refunded.saturating_add(refund_amount);
                Storage::set_funder_ledger(env, grant_id, &addr, &ledger);
            }
        }
    }

    account.balance = 0;
    Storage::set_escrow_account(env, grant_id, &account);

    if let Some(mut grant) = Storage::get_grant(env, grant_id) {
        grant.escrow_balance = 0;
        Storage::set_grant(env, grant_id, &grant);
    }

    Ok(())
}

/// Lock escrow during a dispute (blocks release but not deposit).
pub fn lock(env: &Env, grant_id: u64) -> Result<(), ContractError> {
    let mut account = load_account(env, grant_id)?;
    account.locked = true;
    Storage::set_escrow_account(env, grant_id, &account);
    Ok(())
}

/// Unlock escrow after dispute resolution.
pub fn unlock(env: &Env, grant_id: u64) -> Result<(), ContractError> {
    let mut account = load_account(env, grant_id)?;
    account.locked = false;
    Storage::set_escrow_account(env, grant_id, &account);
    Ok(())
}

/// Return current escrow account state.
pub fn get_account(env: &Env, grant_id: u64) -> Result<EscrowAccount, ContractError> {
    load_account(env, grant_id)
}

/// Return funder ledger for a specific contributor.
pub fn get_funder_ledger(env: &Env, grant_id: u64, funder: &Address) -> Option<FunderLedger> {
    Storage::get_funder_ledger(env, grant_id, funder)
}

/// Thin wrapper for non-escrow token transfers (e.g. reviewer staking).
pub fn transfer_token(env: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
    token::Client::new(env, token).transfer(from, to, &amount);
}
