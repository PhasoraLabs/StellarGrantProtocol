use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Symbol, Vec};
use stellar_grants::{AcceptanceCriteria, StellarGrantsContractClient};

#[test]
fn test_milestone_submission_and_dispute_auto_capture_snapshots() {
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

    // No snapshots exist yet.
    assert!(client.list_snapshots(&grant_id).is_empty());

    // Submitting a milestone auto-captures a `MilestoneSubmission` snapshot.
    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 1"),
        &String::from_str(&env, "proof"),
    );

    let after_submit = client.list_snapshots(&grant_id);
    assert_eq!(after_submit.len(), 1);

    // Move the milestone from Submitted -> Approved so the two snapshots
    // actually differ.
    let criteria = Vec::from_array(
        &env,
        [AcceptanceCriteria {
            idx: 0,
            description: String::from_str(&env, "Criteria 1"),
            is_required: true,
        }],
    );
    client.checklist_define_criteria(&owner, &grant_id, &0, &criteria);
    let evidence = Vec::from_array(&env, [Some(String::from_str(&env, "https://evidence.com"))]);
    client.checklist_submit(&owner, &grant_id, &0, &evidence);
    client.checklist_review_criterion(&reviewer, &grant_id, &0, &0u32, &true);
    client.milestone_vote(&grant_id, &0, &reviewer, &true, &None);

    // Raising a dispute auto-captures a `DisputeRaised` snapshot.
    client.dispute_raise(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Quality concerns"),
    );

    let snapshots = client.list_snapshots(&grant_id);
    assert_eq!(snapshots.len(), 2);

    let first_id = snapshots.get(0).unwrap().id;
    let second_id = snapshots.get(1).unwrap().id;

    let latest = client.latest_snapshot(&grant_id).unwrap();
    assert_eq!(latest.id, second_id);

    let changes = client.diff_snapshots(&grant_id, &first_id, &second_id);
    let milestone_states_symbol = Symbol::new(&env, "milestone_states");
    assert!(changes.iter().any(|s| s == milestone_states_symbol));
}
