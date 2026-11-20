//! Shared helpers for unit tests that call module functions directly.
//!
//! Storage is only accessible from inside a contract frame, so tests that
//! exercise module internals must register the contract and run their body
//! under `env.as_contract`. The contract is registered at a fixed address so
//! a `setup()` helper and the test body can agree on it without threading the
//! id through every return tuple.

use soroban_sdk::{Address, Env};

const TEST_CONTRACT_ID: &str = "CB7H47T6PZ7H47T6PZ7H47T6PZ7H47T6PZ7H47T6PZ7H47T6PZ7H4YUV";

/// Address the test contract is registered at.
pub(crate) fn contract_id(env: &Env) -> Address {
    Address::from_str(env, TEST_CONTRACT_ID)
}

/// Registers `StellarGrantsContract` at [`contract_id`]. Call once per `Env`.
pub(crate) fn register_contract(env: &Env) -> Address {
    let id = contract_id(env);
    env.register_at(&id, crate::StellarGrantsContract, ());
    id
}

/// Runs `f` inside the test contract's frame.
pub(crate) fn in_contract<T>(env: &Env, f: impl FnOnce() -> T) -> T {
    env.as_contract(&contract_id(env), f)
}
