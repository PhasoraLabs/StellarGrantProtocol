/// Issue #816: WhitelistScope::GlobalContributor was fully implemented in
/// whitelist.rs but never consulted by contributor_register, so restricting
/// the scope had no real effect. These tests exercise the real contract
/// entrypoints to confirm the gate is now enforced.
use soroban_sdk::{testutils::Address as TestAddress, Address, Env, String, Vec};
use stellar_grants::{
    StellarGrantsContract, StellarGrantsContractClient, WhitelistMode, WhitelistScope,
};

fn setup() -> (Env, StellarGrantsContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let admin = <Address as TestAddress>::generate(&env);
    client.set_global_admin(&admin, &admin);
    (env, client, admin)
}

#[test]
fn test_contributor_register_open_mode_unaffected() {
    let (env, client, _admin) = setup();
    let contributor = <Address as TestAddress>::generate(&env);

    // GlobalContributor defaults to Open — registration must remain unrestricted.
    assert_eq!(
        client.whitelist_get_mode(&WhitelistScope::GlobalContributor),
        WhitelistMode::Open
    );

    let result = client.try_contributor_register(
        &contributor,
        &String::from_str(&env, "Alice"),
        &String::from_str(&env, "Rust developer"),
        &Vec::new(&env),
        &String::from_str(&env, "https://github.com/alice"),
    );
    assert!(result.is_ok());
}

#[test]
fn test_contributor_register_blocked_when_restricted_and_not_whitelisted() {
    let (env, client, admin) = setup();
    let contributor = <Address as TestAddress>::generate(&env);

    client.whitelist_set_mode(
        &admin,
        &WhitelistScope::GlobalContributor,
        &WhitelistMode::Restricted,
    );

    let result = client.try_contributor_register(
        &contributor,
        &String::from_str(&env, "Alice"),
        &String::from_str(&env, "Rust developer"),
        &Vec::new(&env),
        &String::from_str(&env, "https://github.com/alice"),
    );
    assert!(result.is_err());

    // Registration must not have gone through.
    assert_eq!(client.contributor_count(), 0);
}

#[test]
fn test_contributor_register_succeeds_once_whitelisted() {
    let (env, client, admin) = setup();
    let contributor = <Address as TestAddress>::generate(&env);

    client.whitelist_set_mode(
        &admin,
        &WhitelistScope::GlobalContributor,
        &WhitelistMode::Restricted,
    );
    client.whitelist_add(&admin, &contributor, &WhitelistScope::GlobalContributor);

    let result = client.try_contributor_register(
        &contributor,
        &String::from_str(&env, "Alice"),
        &String::from_str(&env, "Rust developer"),
        &Vec::new(&env),
        &String::from_str(&env, "https://github.com/alice"),
    );
    assert!(result.is_ok());
    assert_eq!(client.contributor_count(), 1);
}
