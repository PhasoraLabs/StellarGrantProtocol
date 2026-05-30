use crate::types::ContractError;
use soroban_sdk::{contracttype, Env};

#[contracttype]
pub enum ReentrancyKey {
    Guard,
}

pub fn with_non_reentrant<F, R>(env: &Env, f: F) -> R
where
    F: FnOnce() -> R,
{
    if env.storage().temporary().has(&ReentrancyKey::Guard) {
        env.panic_with_error(ContractError::Unauthorized);
    }
    env.storage().temporary().set(&ReentrancyKey::Guard, &());
    let result = f();
    env.storage().temporary().remove(&ReentrancyKey::Guard);
    result
}
