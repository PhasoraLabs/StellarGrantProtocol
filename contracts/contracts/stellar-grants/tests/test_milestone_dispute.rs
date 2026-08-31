use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};
use stellar_grants::{
    AcceptanceCriteria, DisputeStatus, MilestoneState, StellarGrantsContractClient,
};

const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

/// Satisfy the required-criteria checklist gate so an `approve = true` vote
/// actually finalizes the milestone (see
/// tests/integration_lifecycle.rs::setup_checklist).
fn satisfy_checklist(
    env: &Env,
    client: &StellarGrantsContractClient,
    owner: &Address,
    reviewer: &Address,
    grant_id: u64,
    milestone_idx: u32,
) {
    let criteria = Vec::from_array(
        env,
        [AcceptanceCriteria {
            idx: 0,
            description: String::from_str(env, "Criteria 1"),
            is_required: true,
        }],
    );
    client.checklist_define_criteria(owner, &grant_id, &milestone_idx, &criteria);
    let evidence = Vec::from_array(env, [Some(String::from_str(env, "https://evidence.com"))]);
    client.checklist_submit(owner, &grant_id, &milestone_idx, &evidence);
    client.checklist_review_criterion(reviewer, &grant_id, &milestone_idx, &0u32, &true);
}

#[test]
fn test_dispute_and_resolve_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token);
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    client.initialize(&admin);
    client.set_global_admin(&admin, &admin);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Test Grant"),
        &String::from_str(&env, "Desc"),
        &token,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    let funder = Address::generate(&env);
    token_admin.mint(&funder, &1000);
    client.grant_fund(&grant_id, &funder, &1000);

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 1"),
        &String::from_str(&env, "proof"),
    );
    satisfy_checklist(&env, &client, &owner, &reviewer, grant_id, 0);

    let now = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(now + COMMUNITY_REVIEW_PERIOD + 1);
    client.milestone_vote(&grant_id, &0, &reviewer, &true, &None);

    // Raise a dispute
    client.dispute_raise(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Quality concerns"),
    );

    // Verify dispute exists
    let dispute = client.get_dispute_record(&grant_id, &0);
    assert!(dispute.is_some());
    let d = dispute.unwrap();
    assert_eq!(d.raised_by, owner);

    // Assign an arbiter and resolve
    let arbiter = Address::generate(&env);
    client.dispute_assign_arbiter(&grant_id, &0, &admin, &arbiter);

    // Arbiter votes in favor of contributor
    client.dispute_arbiter_vote(&grant_id, &0, &arbiter, &true);

    // Issue #977: prove the resolution moves real funds, not just the status
    // enum. `dispute::resolve_dispute` runs `escrow::release` of exactly
    // `milestone.amount` to the grant owner on a contributor win.
    let token_client = token::Client::new(&env, &token);
    let milestone_amount = client.get_milestone(&grant_id, &0).amount;
    assert_eq!(milestone_amount, 1000);
    let owner_before = token_client.balance(&owner);
    let funder_before = token_client.balance(&funder);
    let escrow_before = client.get_grant(&grant_id).escrow_balance;

    // Resolve the dispute
    let outcome = client.dispute_resolve(&grant_id, &0, &admin);
    assert_eq!(outcome, DisputeStatus::ResolvedForContributor);

    assert_eq!(
        token_client.balance(&owner) - owner_before,
        milestone_amount
    );
    assert_eq!(token_client.balance(&funder), funder_before);
    assert_eq!(
        client.get_grant(&grant_id).escrow_balance,
        escrow_before - milestone_amount
    );

    // Milestone should still be Approved (dispute resolution doesn't change milestone state)
    let milestone = client.get_milestone(&grant_id, &0);
    assert_eq!(milestone.state, MilestoneState::Approved);
}

#[test]
#[should_panic]
fn test_only_admin_can_resolve_dispute() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token);
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    client.initialize(&admin);
    client.set_global_admin(&admin, &admin);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Test Grant"),
        &String::from_str(&env, "Desc"),
        &token,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    let funder = Address::generate(&env);
    token_admin.mint(&funder, &1000);
    client.grant_fund(&grant_id, &funder, &1000);

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 1"),
        &String::from_str(&env, "proof"),
    );
    satisfy_checklist(&env, &client, &owner, &reviewer, grant_id, 0);

    let now = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(now + COMMUNITY_REVIEW_PERIOD + 1);
    client.milestone_vote(&grant_id, &0, &reviewer, &true, &None);

    client.dispute_raise(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Dispute reason"),
    );

    let arbiter = Address::generate(&env);
    client.dispute_assign_arbiter(&grant_id, &0, &admin, &arbiter);
    client.dispute_arbiter_vote(&grant_id, &0, &arbiter, &true);

    // This should panic — owner is not admin
    client.dispute_resolve(&grant_id, &0, &owner);
}
