use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};
use stellar_grants::{AcceptanceCriteria, DelegationScope, StellarGrantsContractClient};

/// Satisfy the required-criteria checklist gate for a milestone so an
/// `approve = true` vote actually counts (see integration_lifecycle.rs's
/// `setup_checklist` for the same pattern).
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

/// Bootstrap a contract + funded grant with the given reviewers/milestones,
/// mirroring the setup conventions used in tests/test_milestone_dispute.rs.
fn setup_grant<'a>(
    env: &Env,
    admin: &Address,
    owner: &Address,
    reviewers: &Vec<Address>,
    num_milestones: u32,
) -> (StellarGrantsContractClient<'a>, u64) {
    let token_admin_addr = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(env, &token);
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(env, &contract_id);
    client.initialize(admin);

    let milestone_amount: i128 = 1000;
    let total_amount = milestone_amount * num_milestones as i128;

    let grant_id = client.grant_create(
        owner,
        &String::from_str(env, "Test Grant"),
        &String::from_str(env, "Desc"),
        &token,
        &total_amount,
        &milestone_amount,
        &num_milestones,
        reviewers,
    );

    let funder = Address::generate(env);
    token_admin.mint(&funder, &total_amount);
    client.grant_fund(&grant_id, &funder, &total_amount);

    (client, grant_id)
}

#[test]
fn test_global_delegation_proxy_vote_resolves_to_real_reviewer() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let delegate = Address::generate(&env);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let (client, grant_id) = setup_grant(&env, &admin, &owner, &reviewers, 1);

    client.delegate_vote(&reviewer, &delegate, &DelegationScope::Global, &None, &None);

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 1"),
        &String::from_str(&env, "proof"),
    );
    satisfy_checklist(&env, &client, &owner, &reviewer, grant_id, 0);

    // The delegate casts the vote, authenticating as itself.
    client.milestone_vote(&grant_id, &0, &delegate, &true, &None);

    // Quorum is reached (1/1 reviewers) and the vote is recorded under the
    // real reviewer's address, not the proxy's.
    let milestone = client.get_milestone(&grant_id, &0);
    assert_eq!(milestone.state, stellar_grants::MilestoneState::Approved);
    assert_eq!(milestone.votes.get(reviewer.clone()), Some(true));
    assert!(milestone.votes.get(delegate.clone()).is_none());

    // Unlimited-use global delegation stays active after being used.
    let delegation = client.get_delegation(&reviewer, &DelegationScope::Global);
    assert!(delegation.is_some());
}

#[test]
fn test_per_grant_delegation_max_uses_exhausted_after_one_vote() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let delegate = Address::generate(&env);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let (client, grant_id) = setup_grant(&env, &admin, &owner, &reviewers, 2);

    let scope = DelegationScope::PerGrant(grant_id);
    client.delegate_vote(&reviewer, &delegate, &scope, &None, &Some(1u32));

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 1"),
        &String::from_str(&env, "proof"),
    );
    satisfy_checklist(&env, &client, &owner, &reviewer, grant_id, 0);
    client.milestone_vote(&grant_id, &0, &delegate, &true, &None);

    // The single use has been consumed, so the delegation is now inactive.
    let delegation = client.get_delegation(&reviewer, &scope);
    assert!(delegation.is_none());
}

#[test]
#[should_panic]
fn test_exhausted_delegation_blocks_further_proxy_vote() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let delegate = Address::generate(&env);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let (client, grant_id) = setup_grant(&env, &admin, &owner, &reviewers, 2);

    let scope = DelegationScope::PerGrant(grant_id);
    client.delegate_vote(&reviewer, &delegate, &scope, &None, &Some(1u32));

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 1"),
        &String::from_str(&env, "proof"),
    );
    satisfy_checklist(&env, &client, &owner, &reviewer, grant_id, 0);
    client.milestone_vote(&grant_id, &0, &delegate, &true, &None);

    // Milestone 0 is approved (1/1 quorum), so milestone 1 can now be submitted.
    client.milestone_submit(
        &grant_id,
        &1,
        &owner,
        &String::from_str(&env, "Milestone 2"),
        &String::from_str(&env, "proof"),
    );
    satisfy_checklist(&env, &client, &owner, &reviewer, grant_id, 1);

    // The delegation's single use was already consumed — the delegate is no
    // longer an authorized proxy and isn't a registered reviewer either.
    client.milestone_vote(&grant_id, &1, &delegate, &true, &None);
}

#[test]
#[should_panic]
fn test_expired_delegation_rejects_proxy_vote() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let delegate = Address::generate(&env);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let (client, grant_id) = setup_grant(&env, &admin, &owner, &reviewers, 1);

    let now = env.ledger().timestamp();
    client.delegate_vote(
        &reviewer,
        &delegate,
        &DelegationScope::Global,
        &Some(now + 100),
        &None,
    );

    env.ledger().set_timestamp(now + 200);

    // The delegation has expired.
    assert!(client
        .get_delegation(&reviewer, &DelegationScope::Global)
        .is_none());

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 1"),
        &String::from_str(&env, "proof"),
    );
    satisfy_checklist(&env, &client, &owner, &reviewer, grant_id, 0);

    // The delegate is no longer an authorized proxy and isn't a reviewer.
    client.milestone_vote(&grant_id, &0, &delegate, &true, &None);
}

#[test]
fn test_revoke_delegation() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let delegate = Address::generate(&env);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let (client, _grant_id) = setup_grant(&env, &admin, &owner, &reviewers, 1);

    client.delegate_vote(&reviewer, &delegate, &DelegationScope::Global, &None, &None);
    assert!(client
        .get_delegation(&reviewer, &DelegationScope::Global)
        .is_some());

    client.revoke_delegation(&reviewer, &DelegationScope::Global);
    assert!(client
        .get_delegation(&reviewer, &DelegationScope::Global)
        .is_none());
}

#[test]
fn test_delegation_cycle_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer_a = Address::generate(&env);
    let reviewer_b = Address::generate(&env);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer_a.clone());
    reviewers.push_back(reviewer_b.clone());

    let (client, _grant_id) = setup_grant(&env, &admin, &owner, &reviewers, 1);

    client.delegate_vote(
        &reviewer_a,
        &reviewer_b,
        &DelegationScope::Global,
        &None,
        &None,
    );

    // B delegating back to A would create a cycle — must be rejected.
    let result = client.try_delegate_vote(
        &reviewer_b,
        &reviewer_a,
        &DelegationScope::Global,
        &None,
        &None,
    );
    assert!(result.is_err());
}
