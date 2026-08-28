use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec};
use stellar_grants::StellarGrantsContractClient;

#[test]
fn test_contributor_registration_and_staking() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let funder = Address::generate(&env);
    let tok_adm = Address::generate(&env);
    let tok = env.register_stellar_asset_contract_v2(tok_adm).address();

    let cid = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(&env, &cid);
    let tok_admin = token::StellarAssetClient::new(&env, &tok);

    let treasury = Address::generate(&env);
    let _ = client.initialize();

    // Register contributor
    client.contributor_register(
        &owner,
        &String::from_str(&env, "Alice"),
        &String::from_str(&env, "Bio"),
        &Vec::<String>::new(&env),
        &String::from_str(&env, "https://github.com/alice"),
    );

    let mut revs = Vec::new(&env);
    revs.push_back(reviewer.clone());

    let gid = client.grant_create(
        &owner,
        &String::from_str(&env, "G"),
        &String::from_str(&env, "D"),
        &tok,
        &1000,
        &1000,
        &1,
        &revs,
    );

    tok_admin.mint(&funder, &2000);
    client.grant_fund(&gid, &funder, &1000);

    client.set_staking_config(&admin, &10i128, &treasury);

    tok_admin.mint(&reviewer, &100);
    client.stake_to_review(&reviewer, &gid, &50);

    // Slash reviewer
    client.slash_reviewer(&admin, &gid, &reviewer);

    let tok_client = token::Client::new(&env, &tok);
    assert_eq!(tok_client.balance(&treasury), 50);
}
