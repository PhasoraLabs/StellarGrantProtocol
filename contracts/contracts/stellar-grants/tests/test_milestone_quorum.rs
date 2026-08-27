use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger as _},
    Address, Env, String, Vec,
};
use stellar_grants::{MilestoneState, StellarGrantsContractClient};

const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

#[test]
fn test_milestone_voting_quorum_and_events() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);

    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);

    let mut reviewers = Vec::new(&env);
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Test Grant"),
        &String::from_str(&env, "Testing"),
        &token_id,
        &100,
        &10,
        &3,
        &reviewers,
    );

    token_admin.mint(&owner, &1000);
    client.grant_fund(&grant_id, &owner, &100);

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "proof"),
    );

    // Advance past the community review period so reviewer voting is allowed
    let ts = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(ts.saturating_add(COMMUNITY_REVIEW_PERIOD).saturating_add(1));

    let res1 = client.milestone_vote(&grant_id, &0, &reviewers.get(0).unwrap(), &true, &None);
    assert!(!res1); // Quorum not reached yet
    let res2 = client.milestone_vote(&grant_id, &0, &reviewers.get(1).unwrap(), &true, &None);
    assert!(res2);

    let milestone = client.get_milestone(&grant_id, &0);
    // After quorum the milestone should be Approved
    assert_eq!(milestone.state, MilestoneState::Approved);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_milestone_vote_after_quorum_panics() {
    use soroban_sdk::{testutils::Address as TestAddress, Address, Env, String, Vec};
    use stellar_grants::StellarGrantsContractClient;
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);

    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);

    let mut reviewers = Vec::new(&env);
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Test Grant"),
        &String::from_str(&env, "Testing"),
        &token_id,
        &100,
        &10,
        &3,
        &reviewers,
    );

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "proof"),
    );
    let ts = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(ts.saturating_add(COMMUNITY_REVIEW_PERIOD).saturating_add(1));
    let _ = client.milestone_vote(&grant_id, &0, &reviewers.get(0).unwrap(), &true, &None);
    let _ = client.milestone_vote(&grant_id, &0, &reviewers.get(1).unwrap(), &true, &None);
    // This vote should panic (milestone already approved — MilestoneNotSubmitted #7)
    let _ = client.milestone_vote(&grant_id, &0, &reviewers.get(2).unwrap(), &true, &None);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #8)")]
fn test_milestone_double_voting_panics() {
    use soroban_sdk::{testutils::Address as TestAddress, Address, Env, String, Vec};
    use stellar_grants::StellarGrantsContractClient;
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);
    let mut reviewers = Vec::new(&env);
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Test Grant"),
        &String::from_str(&env, "Testing"),
        &token_id,
        &100,
        &10,
        &3,
        &reviewers,
    );

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "proof"),
    );

    let ts = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(ts.saturating_add(COMMUNITY_REVIEW_PERIOD).saturating_add(1));

    let _ = client.milestone_vote(&grant_id, &0, &reviewers.get(0).unwrap(), &true, &None);
    let _ = client.milestone_vote(&grant_id, &0, &reviewers.get(0).unwrap(), &true, &None);
}
