use soroban_sdk::{contractevent, Address, Env, String, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{BreakerState, ProtocolModule};

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakerTripped {
    pub module: ProtocolModule,
    pub tripped_by: Address,
    pub reason: String,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakerReset {
    pub module: ProtocolModule,
    pub reset_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakerAutoReset {
    pub module: ProtocolModule,
}

fn require_emergency_pauser(env: &Env, caller: &Address) -> Result<(), ContractError> {
    let global_admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if global_admin != *caller {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

pub fn trip(
    env: &Env,
    caller: &Address,
    module: ProtocolModule,
    reason: String,
    auto_reset_ledger: Option<u32>,
) -> Result<(), ContractError> {
    caller.require_auth();
    require_emergency_pauser(env, caller)?;

    let state = BreakerState {
        module: module.clone(),
        tripped: true,
        tripped_by: Some(caller.clone()),
        tripped_at: Some(env.ledger().timestamp()),
        reason: Some(reason.clone()),
        auto_reset_ledger,
    };
    Storage::set_breaker_state(env, &state);

    BreakerTripped {
        module,
        tripped_by: caller.clone(),
        reason,
    }
    .publish(env);

    Ok(())
}

pub fn reset(env: &Env, caller: &Address, module: ProtocolModule) -> Result<(), ContractError> {
    caller.require_auth();

    let prev = Storage::get_breaker_state(env, &module);
    let is_pauser = Storage::get_global_admin(env) == Some(caller.clone());
    if !is_pauser {
        return Err(ContractError::Unauthorized);
    }

    if prev.as_ref().map(|s| !s.tripped).unwrap_or(true) {
        return Err(ContractError::BreakerNotTripped);
    }

    Storage::remove_breaker(env, &module);

    BreakerReset {
        module,
        reset_by: caller.clone(),
    }
    .publish(env);

    Ok(())
}

pub fn require_open(env: &Env, module: ProtocolModule) -> Result<(), ContractError> {
    if let Some(state) = Storage::get_breaker_state(env, &module) {
        if state.tripped {
            if let Some(reset_ledger) = state.auto_reset_ledger {
                if env.ledger().sequence() > reset_ledger {
                    Storage::remove_breaker(env, &module);
                    BreakerAutoReset { module }.publish(env);
                    return Ok(());
                }
            }
            return Err(ContractError::ModuleTripped);
        }
    }
    Ok(())
}

pub fn is_open(env: &Env, module: ProtocolModule) -> bool {
    require_open(env, module).is_ok()
}

pub fn get_state(env: &Env, module: ProtocolModule) -> BreakerState {
    Storage::get_breaker_state(env, &module).unwrap_or(BreakerState {
        module: module.clone(),
        tripped: false,
        tripped_by: None,
        tripped_at: None,
        reason: None,
        auto_reset_ledger: None,
    })
}

pub fn tripped_modules(env: &Env) -> Vec<ProtocolModule> {
    let all_modules = [
        ProtocolModule::Grants,
        ProtocolModule::Streaming,
        ProtocolModule::Bounty,
        ProtocolModule::Dao,
        ProtocolModule::Staking,
        ProtocolModule::Vesting,
        ProtocolModule::YieldEscrow,
        ProtocolModule::MatchingPool,
        ProtocolModule::Crowdfund,
        ProtocolModule::Insurance,
        ProtocolModule::Relay,
        ProtocolModule::TokenSwap,
        ProtocolModule::Oracle,
    ];

    let mut result: Vec<ProtocolModule> = Vec::new(env);
    for m in all_modules.iter() {
        if let Some(state) = Storage::get_breaker_state(env, m) {
            if state.tripped {
                if let Some(reset_ledger) = state.auto_reset_ledger {
                    if env.ledger().sequence() > reset_ledger {
                        Storage::remove_breaker(env, m);
                        BreakerAutoReset { module: m.clone() }.publish(env);
                        continue;
                    }
                }
                result.push_back(m.clone());
            }
        }
    }
    result
}

pub fn auto_reset_expired(env: &Env) -> u32 {
    let all_modules = [
        ProtocolModule::Grants,
        ProtocolModule::Streaming,
        ProtocolModule::Bounty,
        ProtocolModule::Dao,
        ProtocolModule::Staking,
        ProtocolModule::Vesting,
        ProtocolModule::YieldEscrow,
        ProtocolModule::MatchingPool,
        ProtocolModule::Crowdfund,
        ProtocolModule::Insurance,
        ProtocolModule::Relay,
        ProtocolModule::TokenSwap,
        ProtocolModule::Oracle,
    ];

    let mut count = 0;
    let current_ledger = env.ledger().sequence();
    for m in all_modules.iter() {
        if let Some(state) = Storage::get_breaker_state(env, m) {
            if state.tripped {
                if let Some(reset_ledger) = state.auto_reset_ledger {
                    if current_ledger > reset_ledger {
                        Storage::remove_breaker(env, m);
                        BreakerAutoReset { module: m.clone() }.publish(env);
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StellarGrantsContract;
    use crate::StellarGrantsContractClient;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    fn setup(env: &Env) -> (soroban_sdk::Address, Address) {
        let contract_id = env.register(StellarGrantsContract, ());
        let admin = Address::generate(env);
        let client = StellarGrantsContractClient::new(env, &contract_id);
        client.set_global_admin(&admin, &admin);
        (contract_id, admin)
    }

    #[test]
    fn test_trip_and_require_open_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, admin) = setup(&env);

        env.as_contract(&contract_id, || {
            trip(
                &env,
                &admin,
                ProtocolModule::Streaming,
                String::from_str(&env, "bug found"),
                None,
            )
            .unwrap();
        });

        let result = env.as_contract(&contract_id, || {
            require_open(&env, ProtocolModule::Streaming)
        });
        assert_eq!(result, Err(ContractError::ModuleTripped));

        assert!(
            env.as_contract(&contract_id, || require_open(&env, ProtocolModule::Grants)
                .is_ok())
        );
    }

    #[test]
    fn test_reset_restores_operation() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, admin) = setup(&env);

        env.as_contract(&contract_id, || {
            trip(
                &env,
                &admin,
                ProtocolModule::Grants,
                String::from_str(&env, "maintenance"),
                None,
            )
            .unwrap();
        });

        assert!(!env.as_contract(&contract_id, || is_open(&env, ProtocolModule::Grants)));

        env.as_contract(&contract_id, || {
            reset(&env, &admin, ProtocolModule::Grants).unwrap();
        });
        assert!(env.as_contract(&contract_id, || is_open(&env, ProtocolModule::Grants)));
    }

    #[test]
    fn test_auto_reset_after_ledger() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, admin) = setup(&env);

        let reset_ledger = env.ledger().sequence() + 10;
        env.as_contract(&contract_id, || {
            trip(
                &env,
                &admin,
                ProtocolModule::Insurance,
                String::from_str(&env, "auto"),
                Some(reset_ledger),
            )
            .unwrap();
        });

        assert!(!env.as_contract(&contract_id, || is_open(&env, ProtocolModule::Insurance)));

        env.ledger().with_mut(|li| li.sequence_number += 20);

        assert!(env.as_contract(&contract_id, || is_open(&env, ProtocolModule::Insurance)));
    }

    #[test]
    fn test_tripped_modules_list() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, admin) = setup(&env);

        env.as_contract(&contract_id, || {
            trip(
                &env,
                &admin,
                ProtocolModule::Streaming,
                String::from_str(&env, "test"),
                None,
            )
            .unwrap();
        });
        env.as_contract(&contract_id, || {
            trip(
                &env,
                &admin,
                ProtocolModule::Oracle,
                String::from_str(&env, "test2"),
                None,
            )
            .unwrap();
        });

        let tripped = env.as_contract(&contract_id, || tripped_modules(&env));
        assert!(tripped.contains(ProtocolModule::Streaming));
        assert!(tripped.contains(ProtocolModule::Oracle));
        assert!(!tripped.contains(ProtocolModule::Grants));
    }

    #[test]
    fn test_get_state_default() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _) = setup(&env);

        env.as_contract(&contract_id, || {
            let state = get_state(&env, ProtocolModule::Grants);
            assert!(!state.tripped);
            assert_eq!(state.module, ProtocolModule::Grants);
        });
    }

    #[test]
    fn test_reset_not_tripped_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, admin) = setup(&env);

        let result = env.as_contract(&contract_id, || reset(&env, &admin, ProtocolModule::Grants));
        assert_eq!(result, Err(ContractError::BreakerNotTripped));
    }

    #[test]
    fn test_auto_reset_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, admin) = setup(&env);

        let reset_ledger = env.ledger().sequence() + 5;
        env.as_contract(&contract_id, || {
            trip(
                &env,
                &admin,
                ProtocolModule::Relay,
                String::from_str(&env, "auto"),
                Some(reset_ledger),
            )
            .unwrap();
        });

        env.ledger().with_mut(|li| li.sequence_number += 10);
        let count = env.as_contract(&contract_id, || auto_reset_expired(&env));
        assert_eq!(count, 1);
        assert!(env.as_contract(&contract_id, || is_open(&env, ProtocolModule::Relay)));
    }
}
