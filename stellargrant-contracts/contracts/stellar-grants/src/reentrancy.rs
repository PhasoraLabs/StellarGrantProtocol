use crate::types::ContractError;
use soroban_sdk::{contracttype, Env};

#[contracttype]
pub enum ReentrancyKey {
    EntryGuard,
    ExternalCallGuard,
}

/// Existing guard: wraps `f` in a non-reentrant context. Panics on re-entry.
pub fn with_non_reentrant<F, R>(env: &Env, f: F) -> R
where
    F: FnOnce() -> R,
{
    if env.storage().temporary().has(&ReentrancyKey::EntryGuard)
        || env
            .storage()
            .temporary()
            .has(&ReentrancyKey::ExternalCallGuard)
    {
        env.panic_with_error(ContractError::Reentrancy);
    }
    env.storage()
        .temporary()
        .set(&ReentrancyKey::EntryGuard, &());
    let result = f();
    env.storage().temporary().remove(&ReentrancyKey::EntryGuard);
    result
}

/// Lightweight reentrancy check. Returns `Err(Reentrancy)` while an
/// external call is in progress.
pub fn protect(env: &Env) -> Result<(), ContractError> {
    if env
        .storage()
        .temporary()
        .has(&ReentrancyKey::ExternalCallGuard)
    {
        return Err(ContractError::Reentrancy);
    }
    Ok(())
}

/// Acquire the reentrancy guard, execute `f`, then release the guard.
/// Returns `Err(Reentrancy)` if the guard is already held.
/// The guard is always cleared, even when `f` returns an error.
pub fn protect_external_call<F, T>(env: &Env, f: F) -> Result<T, ContractError>
where
    F: FnOnce() -> Result<T, ContractError>,
{
    if env
        .storage()
        .temporary()
        .has(&ReentrancyKey::ExternalCallGuard)
    {
        return Err(ContractError::Reentrancy);
    }
    env.storage()
        .temporary()
        .set(&ReentrancyKey::ExternalCallGuard, &());
    let result = f();
    env.storage()
        .temporary()
        .remove(&ReentrancyKey::ExternalCallGuard);
    result
}

// ── Unit Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn with_contract(env: &Env, f: impl FnOnce()) {
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, f);
    }

    #[test]
    fn test_protect_succeeds_when_guard_not_held() {
        let env = Env::default();
        with_contract(&env, || {
            assert_eq!(protect(&env), Ok(()));
        });
    }

    #[test]
    fn test_protect_fails_when_guard_held() {
        let env = Env::default();
        with_contract(&env, || {
            env.storage()
                .temporary()
                .set(&ReentrancyKey::ExternalCallGuard, &());
            assert_eq!(protect(&env), Err(ContractError::Reentrancy));
        });
    }

    #[test]
    fn test_protect_external_call_succeeds() {
        let env = Env::default();
        with_contract(&env, || {
            let result = protect_external_call(&env, || Ok(42i128));
            assert_eq!(result, Ok(42));
            // Guard should be cleared
            assert!(!env
                .storage()
                .temporary()
                .has(&ReentrancyKey::ExternalCallGuard));
        });
    }

    #[test]
    fn test_protect_external_call_clears_guard_on_error() {
        let env = Env::default();
        with_contract(&env, || {
            let result: Result<i128, ContractError> =
                protect_external_call(&env, || Err(ContractError::InvalidInput));
            assert_eq!(result, Err(ContractError::InvalidInput));
            // Guard must be cleared even after inner error
            assert!(!env
                .storage()
                .temporary()
                .has(&ReentrancyKey::ExternalCallGuard));
        });
    }

    #[test]
    fn test_protect_external_call_rejects_reentry() {
        let env = Env::default();
        with_contract(&env, || {
            env.storage()
                .temporary()
                .set(&ReentrancyKey::ExternalCallGuard, &());
            let result: Result<i128, ContractError> = protect_external_call(&env, || Ok(0));
            assert_eq!(result, Err(ContractError::Reentrancy));
        });
    }

    #[test]
    fn test_nested_protect_inside_external_call_detects_reentrancy() {
        let env = Env::default();
        with_contract(&env, || {
            let result: Result<(), ContractError> = protect_external_call(&env, || {
                // Simulates a callback trying to re-enter
                protect(&env)
            });
            // The inner protect() should detect the guard and fail
            assert_eq!(result, Err(ContractError::Reentrancy));
        });
    }
}
