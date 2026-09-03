/// Issue #815: batch_add_reviewer (backed by multi_grant::add_reviewer_to_grant)
/// never checked the GlobalReviewer whitelist, so a reviewer excluded from
/// the whitelist at grant-creation time could still be added post-creation.
/// These tests confirm the same gate is now enforced via the real contract
/// entrypoints.
use soroban_sdk::{testutils::Address as TestAddress, Address, Env, String, Vec};
use stellar_grants::{
    StellarGrantsContract, StellarGrantsContractClient, WhitelistMode, WhitelistScope,
};

fn setup() -> (Env, StellarGrantsContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);
    client.set_global_admin(&admin, &admin);
    (env, client, owner, admin)
}

#[test]
fn test_batch_add_reviewer_blocked_when_not_whitelisted() {
    let (env, client, owner, admin) = setup();
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let reviewer = <Address as TestAddress>::generate(&env);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Whitelist Grant"),
        &String::from_str(&env, "Testing reviewer whitelist bypass"),
        &token_id,
        &1000,
        &1000,
        &1,
        &Vec::new(&env),
    );

    client.whitelist_set_mode(
        &admin,
        &WhitelistScope::GlobalReviewer,
        &WhitelistMode::Restricted,
    );

    let mut grant_ids = Vec::new(&env);
    grant_ids.push_back(grant_id);

    // Not whitelisted: the batch call must not sneak this reviewer onto the grant.
    let result = client.batch_add_reviewer(&owner, &grant_ids, &reviewer);
    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 1);

    let grant = client.get_grant(&grant_id);
    assert!(!grant.reviewers.contains(reviewer));
}

#[test]
fn test_batch_add_reviewer_succeeds_once_whitelisted() {
    let (env, client, owner, admin) = setup();
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let reviewer = <Address as TestAddress>::generate(&env);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Whitelist Grant"),
        &String::from_str(&env, "Testing reviewer whitelist bypass"),
        &token_id,
        &1000,
        &1000,
        &1,
        &Vec::new(&env),
    );

    client.whitelist_set_mode(
        &admin,
        &WhitelistScope::GlobalReviewer,
        &WhitelistMode::Restricted,
    );
    client.whitelist_add(&admin, &reviewer, &WhitelistScope::GlobalReviewer);

    let mut grant_ids = Vec::new(&env);
    grant_ids.push_back(grant_id);

    let result = client.batch_add_reviewer(&owner, &grant_ids, &reviewer);
    assert_eq!(result.successful, 1);
    assert_eq!(result.failed, 0);

    let grant = client.get_grant(&grant_id);
    assert!(grant.reviewers.contains(reviewer));
}
