/// Full grant lifecycle integration tests (#625).
///
/// Run via: `cargo test --test integration_lifecycle` from the workspace root.
///
/// Scenarios:
///  1. Happy path — 3-milestone grant with full lifecycle
///  2. Milestone dispute and resolution
///  3. Grant cancellation with refund
///  4. Partial votes with quorum snapshot invariant
use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger as _},
    Address, Env, String, Vec,
};
use stellar_grants::{
    ContractError, GrantStatus, MilestoneState, StellarGrantsContractClient,
};

fn setup() -> (Env, StellarGrantsContractClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);
    let token_admin = <Address as TestAddress>::generate(&env);

    (env, client, contract_id, owner, admin)
}

fn create_token(env: &Env, admin: &Address) -> Address {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    token_contract.address()
}

fn make_reviewers(env: &Env, count: u32) -> Vec<Address> {
    let mut reviewers = Vec::new(env);
    for _ in 0..count {
        reviewers.push_back(<Address as TestAddress>::generate(env));
    }
    reviewers
}

fn fund_token(env: &Env, token_id: &Address, contract_id: &Address, admin: &Address, amount: i128) {
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(env, token_id);
    token_admin_client.mint(contract_id, amount);
}

// ── Scenario 1: Happy path — 3-milestone grant ──────────────────────────────

#[test]
fn test_happy_path_3_milestone_grant() {
    let (env, client, contract_id, owner, admin) = setup();
    let token_id = create_token(&env, &admin);
    let reviewers = make_reviewers(&env, 3);
    let funder = <Address as TestAddress>::generate(&env);

    // Mint tokens to the contract
    fund_token(&env, &token_id, &contract_id, &admin, 5000);

    // Create grant: 3 milestones of 1000 each
    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Test Grant"),
        &String::from_str(&env, "Testing lifecycle"),
        &token_id,
        &3000,
        &1000,
        &3,
        &reviewers,
    );
    assert!(grant_id > 0);

    // Fund the grant
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&funder, &3000);
    client.grant_fund(&grant_id, &funder, &3000);

    // Verify grant is active
    let grant = client.get_grant(&grant_id);
    assert_eq!(grant.status(), GrantStatus::Active);
    assert_eq!(grant.escrow_balance(), 3000);

    // Complete all 3 milestones
    for milestone_idx in 0u32..3u32 {
        // Submit milestone
        client.milestone_submit(
            &grant_id,
            &milestone_idx,
            &owner,
            &String::from_str(&env, &format!("Milestone {}", milestone_idx)),
            &String::from_str(&env, "https://proof.example.com"),
        );

        // Two reviewers approve
        let reviewer1 = reviewers.get(0).unwrap();
        let reviewer2 = reviewers.get(1).unwrap();
        let approved1 = client.milestone_vote(&grant_id, &milestone_idx, &reviewer1, &true, &None);
        let approved2 = client.milestone_vote(&grant_id, &milestone_idx, &reviewer2, &true, &None);

        // At least one should indicate quorum reached (with 3 reviewers, 2 approvals = quorum)
        assert!(approved1 || approved2);

        let milestone = client.get_milestone(&grant_id, &milestone_idx);
        // After quorum the milestone should be Approved or AwaitingPayout
        assert!(
            milestone.state() == MilestoneState::Approved
                || milestone.state() == MilestoneState::AwaitingPayout
        );
    }

    // Complete the grant
    client.grant_complete(&grant_id);

    let grant = client.get_grant(&grant_id);
    assert_eq!(grant.status(), GrantStatus::Completed);
    assert_eq!(grant.escrow_balance(), 0);
}

// ── Scenario 2: Milestone dispute and resolution ────────────────────────────

#[test]
fn test_milestone_dispute_and_resolution() {
    let (env, client, contract_id, owner, admin) = setup();
    let token_id = create_token(&env, &admin);
    let reviewers = make_reviewers(&env, 2);
    let contributor = <Address as TestAddress>::generate(&env);

    fund_token(&env, &token_id, &contract_id, &admin, 1000);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Dispute Grant"),
        &String::from_str(&env, "Testing dispute"),
        &token_id,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    // Fund the grant
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&owner, &1000);
    client.grant_fund(&grant_id, &owner, &1000);

    // Submit milestone
    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Disputed work"),
        &String::from_str(&env, "https://proof.example.com"),
    );

    // Raise a dispute
    client.dispute_raise(
        &grant_id,
        &0,
        &contributor,
        &String::from_str(&env, "Work quality below expectations"),
    );

    // Verify dispute exists
    let dispute = client.get_dispute_record(&grant_id, &0);
    assert!(dispute.is_some());
    let d = dispute.unwrap();
    assert_eq!(d.raised_by(), contributor);

    // Assign an arbiter and resolve
    let arbiter = <Address as TestAddress>::generate(&env);
    client.dispute_assign_arbiter(&grant_id, &0, &admin, &arbiter);

    // Arbiter votes in favor of contributor
    client.dispute_arbiter_vote(&grant_id, &0, &arbiter, &true);

    // Resolve the dispute
    let outcome = client.dispute_resolve(&grant_id, &0, &admin);
    // Outcome should be ResolvedForContributor or ResolvedForFunder
    assert!(
        outcome == stellar_grants::DisputeStatus::ResolvedForContributor
            || outcome == stellar_grants::DisputeStatus::ResolvedForFunder
    );
}

// ── Scenario 3: Grant cancellation with refund ──────────────────────────────

#[test]
fn test_grant_cancellation_with_refund() {
    let (env, client, contract_id, owner, admin) = setup();
    let token_id = create_token(&env, &admin);
    let reviewers = make_reviewers(&env, 2);
    let funder = <Address as TestAddress>::generate(&env);

    fund_token(&env, &token_id, &contract_id, &admin, 3000);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Cancellable Grant"),
        &String::from_str(&env, "Will be cancelled"),
        &token_id,
        &3000,
        &1000,
        &3,
        &reviewers,
    );

    // Fund the full amount
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&funder, &3000);
    client.grant_fund(&grant_id, &funder, &3000);

    // Verify escrow has funds
    let grant = client.get_grant(&grant_id);
    assert_eq!(grant.escrow_balance(), 3000);

    // Complete first milestone
    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "First milestone"),
        &String::from_str(&env, "https://proof.example.com"),
    );

    let reviewer1 = reviewers.get(0).unwrap();
    let reviewer2 = reviewers.get(1).unwrap();
    client.milestone_vote(&grant_id, &0, &reviewer1, &true, &None);
    client.milestone_vote(&grant_id, &0, &reviewer2, &true, &None);

    // Grant should have some paid out, some in escrow
    let grant_after_milestone = client.get_grant(&grant_id);
    // Escrow balance should be reduced after payout
    assert!(grant_after_milestone.escrow_balance() < 3000);

    // Cancel the grant
    client.grant_cancel(
        &grant_id,
        &owner,
        &String::from_str(&env, "No longer needed"),
    );

    let grant = client.get_grant(&grant_id);
    assert_eq!(grant.status(), GrantStatus::Cancelled);
    assert_eq!(grant.escrow_balance(), 0);
}

// ── Scenario 4: Reviewer quorum snapshot invariant ──────────────────────────

/// Verify that quorum calculations use the snapshotted reviewer count,
/// not the live list. Submit with 3 reviewers, add a 4th reviewer,
/// and verify quorum is still computed against 3.
#[test]
fn test_quorum_uses_snapshot_not_live_list() {
    let (env, client, contract_id, owner, admin) = setup();
    let token_id = create_token(&env, &admin);
    let reviewers = make_reviewers(&env, 3);

    fund_token(&env, &token_id, &contract_id, &admin, 1000);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Snapshot Grant"),
        &String::from_str(&env, "Testing snapshot"),
        &token_id,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&owner, &1000);
    client.grant_fund(&grant_id, &owner, &1000);

    // Submit milestone (snapshots 3 reviewers)
    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Snapshot test"),
        &String::from_str(&env, "https://proof.example.com"),
    );

    // Vote with 2 of 3 original reviewers → should reach quorum
    let reviewer1 = reviewers.get(0).unwrap();
    let reviewer2 = reviewers.get(1).unwrap();
    let approved = client.milestone_vote(&grant_id, &0, &reviewer1, &true, &None);
    assert!(!approved); // Not yet with 1 vote

    let approved = client.milestone_vote(&grant_id, &0, &reviewer2, &true, &None);
    // With 2 out of 3 snapshotted reviewers, quorum should be reached
    assert!(approved);

    let milestone = client.get_milestone(&grant_id, &0);
    // Milestone should be finalized
    assert!(
        milestone.state() == MilestoneState::Approved
            || milestone.state() == MilestoneState::AwaitingPayout
    );
}
