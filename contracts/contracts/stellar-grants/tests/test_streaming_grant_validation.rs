/// Issue #814: streaming::create_stream never validated the grant_id it was
/// created against. These tests exercise the real contract entrypoints to
/// confirm a stream can no longer be created against a nonexistent grant or
/// one that isn't Active.
use soroban_sdk::{testutils::Address as TestAddress, Address, Env, String, Vec};
use stellar_grants::{StellarGrantsContract, StellarGrantsContractClient};

fn setup() -> (Env, StellarGrantsContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);
    (env, client, owner, admin)
}

#[test]
fn test_create_stream_rejects_nonexistent_grant() {
    let (env, client, owner, admin) = setup();
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let recipient = <Address as TestAddress>::generate(&env);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&owner, &10_000);

    // Grant 999 was never created.
    let result = client.try_create_stream(&owner, &recipient, &999, &token_id, &100, &10);
    assert!(result.is_err());
}

#[test]
fn test_create_stream_rejects_cancelled_grant() {
    let (env, client, owner, admin) = setup();
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let recipient = <Address as TestAddress>::generate(&env);
    let reviewers = Vec::new(&env);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Stream Grant"),
        &String::from_str(&env, "Testing stream validation"),
        &token_id,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    client.grant_cancel(&grant_id, &owner, &String::from_str(&env, "abandoned"));

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&owner, &10_000);

    let result = client.try_create_stream(&owner, &recipient, &grant_id, &token_id, &100, &10);
    assert!(result.is_err());
}

#[test]
fn test_create_stream_succeeds_for_active_grant() {
    let (env, client, owner, admin) = setup();
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let recipient = <Address as TestAddress>::generate(&env);
    let reviewers = Vec::new(&env);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Stream Grant"),
        &String::from_str(&env, "Testing stream validation"),
        &token_id,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&owner, &10_000);

    let stream_id = client.create_stream(&owner, &recipient, &grant_id, &token_id, &100, &10);
    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.grant_id, grant_id);
}
