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

/// Release funds proportionally to funders (dispute funder-favor resolution).
pub fn release_to_funders(
    env: &Env,
    grant_id: u64,
    funders: &soroban_sdk::Vec<crate::types::GrantFund>,
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

    if funders.is_empty() {
        return Err(ContractError::InvalidInput);
    }

    let token_client = token::Client::new(env, &account.token);
    let mut total_contributed: i128 = 0;
    for fund in funders.iter() {
        total_contributed = total_contributed.saturating_add(fund.amount);
    }

    if total_contributed <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let mut distributed: i128 = 0;
    let funders_len = funders.len();
    for i in 0..funders_len {
        let fund = funders.get(i).ok_or(ContractError::InvalidInput)?;
        let is_last = i + 1 == funders_len;
        let share = if is_last {
            amount - distributed
        } else {
            fund.amount
                .checked_mul(amount)
                .ok_or(ContractError::InvalidInput)?
                .checked_div(total_contributed)
                .ok_or(ContractError::InvalidInput)?
        };
        if share > 0 {
            crate::reentrancy::protect_external_call(env, || {
                token_client.transfer(&env.current_contract_address(), &fund.funder, &share);
                Ok(())
            })?;
            distributed = distributed.saturating_add(share);
        }
    }

    account.balance -= distributed;
    account.total_released = account
        .total_released
        .checked_add(distributed)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_escrow_account(env, grant_id, &account);

    if let Some(mut grant) = Storage::get_grant(env, grant_id) {
        grant.escrow_balance = account.balance;
        Storage::set_grant(env, grant_id, &grant);
    }

    Ok(())
}

/// Thin wrapper for non-escrow token transfers (e.g. reviewer staking).
pub fn transfer_token(env: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
    token::Client::new(env, token).transfer(from, to, &amount);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{EscrowAccount, FunderLedger, GrantFund};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, Vec};

    fn setup(env: &Env, grant_id: u64) -> (Address, Address) {
        let owner = Address::generate(env);
        let token = Address::generate(env);
        let _ = open(env, grant_id, &owner, &token);
        (owner, token)
    }

    #[test]
    fn test_open_escrow_account() {
        let env = Env::default();
        let grant_id = 1u64;
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        let result = open(&env, grant_id, &owner, &token);
        assert_eq!(result, Ok(()));

        let account = Storage::get_escrow_account(&env, grant_id);
        assert!(account.is_some());
        let acc = account.unwrap();
        assert_eq!(acc.balance, 0);
        assert_eq!(acc.total_deposited, 0);
        assert_eq!(acc.total_released, 0);
        assert_eq!(acc.locked, false);
    }

    #[test]
    fn test_open_escrow_already_exists() {
        let env = Env::default();
        let grant_id = 1u64;
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        let _ = open(&env, grant_id, &owner, &token);
        let result = open(&env, grant_id, &owner, &token);
        assert_eq!(result, Err(ContractError::EscrowAlreadyOpen));
    }

    #[test]
    fn test_lock_unlock() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let result = lock(&env, grant_id);
        assert_eq!(result, Ok(()));

        let account = Storage::get_escrow_account(&env, grant_id).unwrap();
        assert_eq!(account.locked, true);

        let result = unlock(&env, grant_id);
        assert_eq!(result, Ok(()));

        let account = Storage::get_escrow_account(&env, grant_id).unwrap();
        assert_eq!(account.locked, false);
    }

    #[test]
    fn test_release_updates_balance() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let mut account = Storage::get_escrow_account(&env, grant_id).unwrap();
        account.balance = 1000;
        Storage::set_escrow_account(&env, grant_id, &account);

        let recipient = Address::generate(&env);
        let result = release(&env, grant_id, &recipient, 500);
        assert_eq!(result, Ok(()));

        let account = Storage::get_escrow_account(&env, grant_id).unwrap();
        assert_eq!(account.balance, 500);
        assert_eq!(account.total_released, 500);
    }

    #[test]
    fn test_release_locked_escrow() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let mut account = Storage::get_escrow_account(&env, grant_id).unwrap();
        account.balance = 1000;
        account.locked = true;
        Storage::set_escrow_account(&env, grant_id, &account);

        let recipient = Address::generate(&env);
        let result = release(&env, grant_id, &recipient, 500);
        assert_eq!(result, Err(ContractError::EscrowLocked));
    }

    #[test]
    fn test_release_insufficient_balance() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let account = Storage::get_escrow_account(&env, grant_id).unwrap();
        assert_eq!(account.balance, 0);

        let recipient = Address::generate(&env);
        let result = release(&env, grant_id, &recipient, 500);
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_refund_single_funder() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let funder = Address::generate(&env);
        let mut account = Storage::get_escrow_account(&env, grant_id).unwrap();
        account.balance = 1000;
        Storage::set_escrow_account(&env, grant_id, &account);

        let ledger = FunderLedger {
            funder: funder.clone(),
            contributed: 500,
            refunded: 0,
            last_contribution_at: 0,
        };
        Storage::set_funder_ledger(&env, grant_id, &funder, &ledger);

        let result = refund(&env, grant_id, &funder);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 500);

        let account = Storage::get_escrow_account(&env, grant_id).unwrap();
        assert_eq!(account.balance, 500);

        let ledger = Storage::get_funder_ledger(&env, grant_id, &funder).unwrap();
        assert_eq!(ledger.refunded, 500);
    }

    #[test]
    fn test_refund_no_refundable_amount() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let funder = Address::generate(&env);
        let result = refund(&env, grant_id, &funder);
        assert_eq!(result, Err(ContractError::NoRefundableAmount));
    }

    #[test]
    fn test_refund_all_empty_escrow() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let result = refund_all(&env, grant_id);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_refund_all_multiple_funders() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let funder1 = Address::generate(&env);
        let funder2 = Address::generate(&env);

        let mut account = Storage::get_escrow_account(&env, grant_id).unwrap();
        account.balance = 1000;
        Storage::set_escrow_account(&env, grant_id, &account);

        let ledger1 = FunderLedger {
            funder: funder1.clone(),
            contributed: 600,
            refunded: 0,
            last_contribution_at: 0,
        };
        let ledger2 = FunderLedger {
            funder: funder2.clone(),
            contributed: 400,
            refunded: 0,
            last_contribution_at: 0,
        };
        Storage::set_funder_ledger(&env, grant_id, &funder1, &ledger1);
        Storage::set_funder_ledger(&env, grant_id, &funder2, &ledger2);

        let mut funders = Vec::new(&env);
        funders.push_back(funder1.clone());
        funders.push_back(funder2.clone());
        Storage::set_escrow_funders_list(&env, grant_id, &funders);

        let result = refund_all(&env, grant_id);
        assert_eq!(result, Ok(()));

        let account = Storage::get_escrow_account(&env, grant_id).unwrap();
        assert_eq!(account.balance, 0);
    }

    #[test]
    fn test_release_to_funders() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let funder1 = Address::generate(&env);
        let funder2 = Address::generate(&env);

        let mut account = Storage::get_escrow_account(&env, grant_id).unwrap();
        account.balance = 1000;
        Storage::set_escrow_account(&env, grant_id, &account);

        let fund1 = GrantFund {
            funder: funder1.clone(),
            amount: 600,
        };
        let fund2 = GrantFund {
            funder: funder2.clone(),
            amount: 400,
        };
        let mut funders = Vec::new(&env);
        funders.push_back(fund1);
        funders.push_back(fund2);

        let result = release_to_funders(&env, grant_id, &funders, 500);
        assert_eq!(result, Ok(()));

        let account = Storage::get_escrow_account(&env, grant_id).unwrap();
        assert_eq!(account.balance, 500);
        assert_eq!(account.total_released, 500);
    }

    #[test]
    fn test_release_to_funders_locked() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let mut account = Storage::get_escrow_account(&env, grant_id).unwrap();
        account.balance = 1000;
        account.locked = true;
        Storage::set_escrow_account(&env, grant_id, &account);

        let fund1 = GrantFund {
            funder: Address::generate(&env),
            amount: 600,
        };
        let mut funders = Vec::new(&env);
        funders.push_back(fund1);

        let result = release_to_funders(&env, grant_id, &funders, 500);
        assert_eq!(result, Err(ContractError::EscrowLocked));
    }

    #[test]
    fn test_get_account() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let result = get_account(&env, grant_id);
        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.balance, 0);
        assert_eq!(account.total_deposited, 0);
    }

    #[test]
    fn test_get_funder_ledger() {
        let env = Env::default();
        env.mock_all_auths();
        let grant_id = 1u64;
        let (owner, _token) = setup(&env, grant_id);

        let funder = Address::generate(&env);
        let ledger = FunderLedger {
            funder: funder.clone(),
            contributed: 500,
            refunded: 0,
            last_contribution_at: 100,
        };
        Storage::set_funder_ledger(&env, grant_id, &funder, &ledger);

        let result = get_funder_ledger(&env, grant_id, &funder);
        assert!(result.is_some());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.contributed, 500);
        assert_eq!(retrieved.refunded, 0);
    }
}
