use soroban_sdk::{Address, Bytes, Env, Error, IntoVal, InvokeError, Symbol, TryFromVal, Val, Vec};

use crate::errors::ContractError;

/// Generic safe cross-contract call with try-semantics (no panic on failure).
pub fn safe_call(
    env: &Env,
    contract: &Address,
    function_name: &Symbol,
    args: Vec<Val>,
) -> Result<Val, ContractError> {
    type CallResult = Result<Result<Val, soroban_sdk::ConversionError>, Result<Error, InvokeError>>;
    let result: CallResult = env.try_invoke_contract(contract, function_name, args);
    match result {
        Ok(Ok(val)) => Ok(val),
        _ => Err(ContractError::InvalidInput),
    }
}

/// Call a hook receiver contract's `on_hook` function.
/// Returns false if the call fails (non-fatal).
pub fn call_hook_receiver(env: &Env, contract: &Address, event_type: u32, payload: Bytes) -> bool {
    let args: Vec<Val> = Vec::from_array(env, [event_type.into_val(env), payload.into_val(env)]);
    let symbol = Symbol::new(env, "on_hook");
    safe_call(env, contract, &symbol, args).is_ok()
}

/// Read a price from an oracle contract following the standard interface.
pub fn read_oracle_price(
    env: &Env,
    oracle_contract: &Address,
    token: &Address,
) -> Result<(i128, u64), ContractError> {
    let args: Vec<Val> = Vec::from_array(env, [token.clone().into_val(env)]);
    let symbol = Symbol::new(env, "price");
    let val = safe_call(env, oracle_contract, &symbol, args)?;
    <(i128, u64)>::try_from_val(env, &val).map_err(|_| ContractError::InvalidInput)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    #[test]
    fn test_safe_call_failure_returns_invalid_input() {
        let env = Env::default();
        let contract = Address::generate(&env);
        let symbol = Symbol::new(&env, "missing_fn");
        let args = Vec::new(&env);
        assert!(safe_call(&env, &contract, &symbol, args).is_err());
    }

    #[test]
    fn test_call_hook_receiver_failure_returns_false() {
        let env = Env::default();
        let contract = Address::generate(&env);
        let payload = Bytes::new(&env);
        assert!(!call_hook_receiver(&env, &contract, 1, payload));
    }
}
