use soroban_sdk::{contractclient, Address, Env};

use crate::cross_contract;
use crate::errors::ContractError;

/// Standard oracle interface: `fn price(env: Env, token: Address) -> (i128, u64)`.
#[contractclient(name = "OracleClient")]
pub trait Oracle {
    fn price(env: Env, token: Address) -> (i128, u64);
}

/// Typed helper that delegates to [`cross_contract::read_oracle_price`].
pub fn read_price(
    env: &Env,
    oracle: &Address,
    token: &Address,
) -> Result<(i128, u64), ContractError> {
    cross_contract::read_oracle_price(env, oracle, token)
}
