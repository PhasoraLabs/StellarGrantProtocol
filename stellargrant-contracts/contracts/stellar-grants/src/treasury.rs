use soroban_sdk::{token, Address, Env};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, TreasurySnapshot};

/// Configure the address authorized to manage the treasury (withdraw / reallocate).
/// Only the current global admin may set or rotate it.
pub fn set_treasury_address(
    env: &Env,
    admin: &Address,
    treasury: &Address,
) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;
    Storage::set_treasury_address(env, treasury);
    Ok(())
}

/// Record protocol-collected funds (fees, unclaimed balances) into the treasury
/// ledger for `token`. This only bookkeeps the balance; callers are responsible
/// for ensuring the tokens were actually transferred into the contract.
pub fn deposit(
    env: &Env,
    token: &Address,
    from: &Address,
    amount: i128,
) -> Result<i128, ContractError> {
    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }
    let current = Storage::get_treasury_balance(env, token);
    let new_balance = current
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_treasury_balance(env, token, new_balance);
    Events::emit_treasury_deposited(env, token.clone(), from.clone(), amount, new_balance);
    Ok(new_balance)
}

/// Withdraw `amount` of `token` from the treasury to `to`. Admin only.
pub fn withdraw(
    env: &Env,
    admin: &Address,
    token: &Address,
    to: &Address,
    amount: i128,
) -> Result<i128, ContractError> {
    require_global_admin(env, admin)?;

    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let balance = Storage::get_treasury_balance(env, token);
    if balance < amount {
        return Err(ContractError::InsufficientTreasuryBalance);
    }

    let new_balance = balance
        .checked_sub(amount)
        .ok_or(ContractError::InvalidInput)?;
    Storage::set_treasury_balance(env, token, new_balance);

    token::Client::new(env, token).transfer(&env.current_contract_address(), to, &amount);

    Events::emit_treasury_withdrawn(
        env,
        token.clone(),
        to.clone(),
        amount,
        new_balance,
        admin.clone(),
    );

    Ok(new_balance)
}

/// Move `amount` of treasury bookkeeping from `from_token` to `to_token`.
/// Does not perform a swap on-chain — it only reallocates the internal
/// accounting between two token balances the treasury already tracks
/// (e.g. after an off-chain or oracle-assisted conversion). Admin only.
pub fn reallocate(
    env: &Env,
    admin: &Address,
    from_token: &Address,
    to_token: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;

    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }
    if from_token == to_token {
        return Err(ContractError::InvalidInput);
    }

    let from_balance = Storage::get_treasury_balance(env, from_token);
    if from_balance < amount {
        return Err(ContractError::InsufficientTreasuryBalance);
    }

    let new_from_balance = from_balance
        .checked_sub(amount)
        .ok_or(ContractError::InvalidInput)?;
    let to_balance = Storage::get_treasury_balance(env, to_token);
    let new_to_balance = to_balance
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;

    Storage::set_treasury_balance(env, from_token, new_from_balance);
    Storage::set_treasury_balance(env, to_token, new_to_balance);

    Events::emit_treasury_reallocated(
        env,
        from_token.clone(),
        to_token.clone(),
        amount,
        admin.clone(),
    );

    Ok(())
}

/// Current treasury balance for `token`.
pub fn balance(env: &Env, token: &Address) -> i128 {
    Storage::get_treasury_balance(env, token)
}

/// Point-in-time snapshot of the treasury balance for `token`, for frontend display.
pub fn snapshot(env: &Env, token: &Address) -> TreasurySnapshot {
    TreasurySnapshot {
        token: token.clone(),
        balance: Storage::get_treasury_balance(env, token),
        taken_at: env.ledger().timestamp(),
    }
}

fn require_global_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    let admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if admin != *caller {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_deposit_zero_amount_rejected() {
        let env = Env::default();
        let token = Address::generate(&env);
        let from = Address::generate(&env);
        let result = deposit(&env, &token, &from, 0);
        assert_eq!(result, Err(ContractError::ZeroAmount));
    }

    #[test]
    fn test_deposit_accumulates_balance() {
        let env = Env::default();
        let token = Address::generate(&env);
        let from = Address::generate(&env);
        deposit(&env, &token, &from, 100).unwrap();
        let new_balance = deposit(&env, &token, &from, 50).unwrap();
        assert_eq!(new_balance, 150);
        assert_eq!(balance(&env, &token), 150);
    }

    #[test]
    fn test_withdraw_unauthorized_caller_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let stranger = Address::generate(&env);
        let token = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let result = withdraw(&env, &stranger, &token, &stranger, 10);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_withdraw_insufficient_balance_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let result = withdraw(&env, &admin, &token, &admin, 10);
        assert_eq!(result, Err(ContractError::InsufficientTreasuryBalance));
    }

    #[test]
    fn test_reallocate_same_token_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let result = reallocate(&env, &admin, &token, &token, 10);
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_reallocate_moves_balance_between_tokens() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        deposit(&env, &from, &admin, 200).unwrap();

        reallocate(&env, &admin, &from, &to, 80).unwrap();

        assert_eq!(balance(&env, &from), 120);
        assert_eq!(balance(&env, &to), 80);
    }

    #[test]
    fn test_snapshot_reflects_current_balance() {
        let env = Env::default();
        let token = Address::generate(&env);
        let from = Address::generate(&env);
        deposit(&env, &token, &from, 333).unwrap();
        let snap = snapshot(&env, &token);
        assert_eq!(snap.balance, 333);
        assert_eq!(snap.token, token);
    }
}
