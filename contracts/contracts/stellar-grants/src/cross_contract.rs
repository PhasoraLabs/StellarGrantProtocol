use soroban_sdk::{Address, Env, Symbol, Val, Vec};

use crate::errors::ContractError;
use crate::interfaces::{HookReceiverClient, OracleClient};

/// Call a hook receiver contract's `on_hook` function using the typed interface.
/// Returns false if the call fails (non-fatal).
pub fn call_hook_receiver(
    env: &Env,
    contract: &Address,
    event_type: u32,
    payload: soroban_sdk::Bytes,
) -> bool {
    let client = HookReceiverClient::new(env, contract);
    client.try_on_hook(&event_type, &payload).is_ok()
}

/// Read a price from an oracle contract using the typed interface.
pub fn read_oracle_price(
    env: &Env,
    oracle_contract: &Address,
    token: &Address,
) -> Result<(i128, u64), ContractError> {
    let client = OracleClient::new(env, oracle_contract);
    client
        .try_price(token)
        .map_err(|_| ContractError::InvalidInput)?
        .map_err(|_| ContractError::InvalidInput)
}

/// Safely invoke a contract method by symbol, returning `Err` on trap/wrong
/// type/missing contract instead of aborting the whole transaction.
///
/// Restored from the pre-merge branch — used by `relay::dispatch_action`
/// to invoke `contributor_register` / `milestone_submit` on the same
/// contract as `RelayableAction::ContributorRegister` / `MilestoneSubmit`.
pub fn safe_call(
    env: &Env,
    contract: &Address,
    symbol: &Symbol,
    args: Vec<Val>,
) -> Result<(), ContractError> {
    env.try_invoke_contract::<soroban_sdk::Error, Val>(contract, symbol, args)
        .map(|_| ())
        .map_err(|_| ContractError::InvalidInput)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    #[test]
    fn test_call_hook_receiver_failure_returns_false() {
        let env = Env::default();
        let contract = Address::generate(&env);
        let payload = soroban_sdk::Bytes::new(&env);
        assert!(!call_hook_receiver(&env, &contract, 1, payload));
    }
}
