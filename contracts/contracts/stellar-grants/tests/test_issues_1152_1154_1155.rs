use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger as _},
    Address, Env, String, Vec,
};
use stellar_grants::{ContractError, MilestoneState, StellarGrantsContractClient};

const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

/// Test for Issue #1154: milestone_reject should return Result::Err instead of panicking
#[test]
fn test_milestone_reject_returns_error_not_submitted() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);

    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);

    let mut reviewers = Vec::new(&env);
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

    let ts = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(ts.saturating_add(COMMUNITY_REVIEW_PERIOD).saturating_add(1));

    client.milestone_reject(
        &grant_id,
        &0,
        &reviewers.get(0).unwrap(),
        &String::from_str(&env, "initial rejection"),
    );

    // Try to reject a milestone that is not submitted (it is now Rejected) - should return MilestoneNotSubmitted
    let result = client.try_milestone_reject(
        &grant_id,
        &0,
        &reviewers.get(0).unwrap(),
        &String::from_str(&env, "rejected"),
    );

    // Verify it returns an error instead of panicking
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(ContractError::MilestoneNotSubmitted)));
}

#[test]
fn test_milestone_reject_returns_error_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);

    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);

    let mut reviewers = Vec::new(&env);
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

    let ts = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(ts.saturating_add(COMMUNITY_REVIEW_PERIOD).saturating_add(1));

    // Try to reject with an unauthorized reviewer
    let unauthorized_reviewer = <Address as TestAddress>::generate(&env);
    let result = client.try_milestone_reject(
        &grant_id,
        &0,
        &unauthorized_reviewer,
        &String::from_str(&env, "rejected"),
    );

    // Verify it returns an error instead of panicking
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_milestone_reject_returns_error_already_voted() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
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

    let ts = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(ts.saturating_add(COMMUNITY_REVIEW_PERIOD).saturating_add(1));

    let reviewer = reviewers.get(0).unwrap();

    // First rejection should succeed (quorum not reached yet with 2 reviewers)
    let result1 = client.milestone_reject(
        &grant_id,
        &0,
        &reviewer,
        &String::from_str(&env, "rejected"),
    );
    assert!(!result1);

    // Second rejection by same reviewer should return error
    let result2 = client.try_milestone_reject(
        &grant_id,
        &0,
        &reviewer,
        &String::from_str(&env, "rejected again"),
    );

    // Verify it returns an error instead of panicking
    assert!(result2.is_err());
    assert_eq!(result2.err(), Some(Ok(ContractError::AlreadyVoted)));
}

/// Test for Issue #1155: contributor milestones_rejected field is incremented
#[test]
fn test_milestone_rejection_increments_contributor_rejected_count() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);

    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);

    // Register contributor
    client.contributor_register(
        &owner,
        &String::from_str(&env, "Alice"),
        &String::from_str(&env, "A developer"),
        &Vec::new(&env),
        &String::from_str(&env, "https://github.com/alice"),
    );

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

    // Get initial profile state
    let profile_before = client.get_contributor(&owner);
    assert_eq!(profile_before.milestones_rejected, 0);
    let score_before = profile_before.reputation_score;

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

    // Reject the milestone with majority votes
    client.milestone_vote(
        &grant_id,
        &0,
        &reviewers.get(0).unwrap(),
        &false,
        &Some(String::from_str(&env, "no")),
    );

    client.milestone_vote(
        &grant_id,
        &0,
        &reviewers.get(1).unwrap(),
        &false,
        &Some(String::from_str(&env, "no2")),
    );

    // Verify milestone is rejected
    let milestone = client.get_milestone(&grant_id, &0);
    assert_eq!(milestone.state, MilestoneState::Rejected);

    // Verify contributor's rejected count was incremented
    let profile_after = client.get_contributor(&owner);
    assert_eq!(profile_after.milestones_rejected, 1);

    // Verify reputation score was affected by the rejection
    assert!(profile_after.reputation_score < score_before || score_before == 0);
}

#[test]
fn test_milestone_rejection_via_milestone_reject_increments_rejected_count() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);

    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);

    // Register contributor
    client.contributor_register(
        &owner,
        &String::from_str(&env, "Bob"),
        &String::from_str(&env, "A developer"),
        &Vec::new(&env),
        &String::from_str(&env, "https://github.com/bob"),
    );

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

    // Get initial profile state
    let profile_before = client.get_contributor(&owner);
    assert_eq!(profile_before.milestones_rejected, 0);

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

    // Reject the milestone using milestone_reject function
    client.milestone_reject(
        &grant_id,
        &0,
        &reviewers.get(0).unwrap(),
        &String::from_str(&env, "rejected"),
    );

    client.milestone_reject(
        &grant_id,
        &0,
        &reviewers.get(1).unwrap(),
        &String::from_str(&env, "rejected2"),
    );

    // Verify milestone is rejected
    let milestone = client.get_milestone(&grant_id, &0);
    assert_eq!(milestone.state, MilestoneState::Rejected);

    // Verify contributor's rejected count was incremented
    let profile_after = client.get_contributor(&owner);
    assert_eq!(profile_after.milestones_rejected, 1);
}
