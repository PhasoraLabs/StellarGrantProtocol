use soroban_sdk::{testutils::Address as _, Address, Env, String};
use stellar_grants::{ProtocolModule, StellarGrantsContractClient};

/// Sets up a test client with the Bounty module circuit breaker tripped.
fn setup_tripped_breaker(env: &Env) -> (StellarGrantsContractClient<'_>, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(env, &contract_id);
    client.initialize(&admin);
    client.set_global_admin(&admin, &admin);

    client.breaker_trip(
        &admin,
        &ProtocolModule::Bounty,
        &String::from_str(env, "security test"),
        &None,
    );

    (client, admin)
}

#[test]
fn test_start_bounty_review_blocked_by_circuit_breaker() {
    let env = Env::default();
    let (client, admin) = setup_tripped_breaker(&env);

    let result = client.try_start_bounty_review(&admin, &1);
    assert_eq!(
        result,
        Err(Ok(stellar_grants::ContractError::ModuleTripped))
    );
}

#[test]
fn test_select_bounty_winner_blocked_by_circuit_breaker() {
    let env = Env::default();
    let (client, admin) = setup_tripped_breaker(&env);
    let winner = Address::generate(&env);

    let result = client.try_select_bounty_winner(&admin, &1, &winner);
    assert_eq!(
        result,
        Err(Ok(stellar_grants::ContractError::ModuleTripped))
    );
}

#[test]
fn test_cancel_bounty_blocked_by_circuit_breaker() {
    let env = Env::default();
    let (client, admin) = setup_tripped_breaker(&env);

    let result = client.try_cancel_bounty(&admin, &1);
    assert_eq!(
        result,
        Err(Ok(stellar_grants::ContractError::ModuleTripped))
    );
}
