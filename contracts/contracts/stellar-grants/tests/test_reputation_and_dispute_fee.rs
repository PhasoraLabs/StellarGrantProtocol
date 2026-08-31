/// Tests for dispute resolution flow (#514).
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};
use stellar_grants::{
    AcceptanceCriteria, DisputeStatus, MilestoneState, StellarGrantsContractClient,
};

const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_env_client_token() -> (
    Env,
    StellarGrantsContractClient<'static>,
    Address, // admin
    Address, // owner
    Address, // reviewer
    Address, // funder
    Address, // token
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let funder = Address::generate(&env);
    let tok_adm = Address::generate(&env);
    let tok = env.register_stellar_asset_contract_v2(tok_adm).address();

    let cid = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let env_ref: &'static Env = unsafe { &*(&env as *const Env) };
    let client = StellarGrantsContractClient::new(env_ref, &cid);
    let tok_client = token::StellarAssetClient::new(env_ref, &tok);

    client.initialize(&admin);
    // `initialize` only records the migration version; `dispute_assign_arbiter`
    // and friends check the global admin, which must be set explicitly (same as
    // tests/integration_lifecycle.rs::test_milestone_dispute_and_resolution).
    client.set_global_admin(&admin, &admin);
    (env, client, admin, owner, reviewer, funder, tok, tok_client)
}

fn create_funded_submitted_voted(
    env: &Env,
    client: &StellarGrantsContractClient,
    owner: &Address,
    reviewer: &Address,
    funder: &Address,
    token: &Address,
    tok_admin: &token::StellarAssetClient,
) -> u64 {
    let mut revs = Vec::new(env);
    revs.push_back(reviewer.clone());

    let gid = client.grant_create(
        owner,
        &String::from_str(env, "G"),
        &String::from_str(env, "D"),
        token,
        &1000,
        &1000,
        &1,
        &revs,
    );
    tok_admin.mint(funder, &2000);
    client.grant_fund(&gid, funder, &1000);
    tok_admin.mint(reviewer, &1);
    client.stake_to_review(reviewer, &gid, &1);
    client.milestone_submit(
        &gid,
        &0,
        owner,
        &String::from_str(env, "MS"),
        &String::from_str(env, "proof"),
    );

    // `milestone_vote` now requires a satisfied acceptance-criteria checklist
    // (same pattern as tests/integration_lifecycle.rs::setup_checklist).
    let criteria = Vec::from_array(
        env,
        [AcceptanceCriteria {
            idx: 0,
            description: String::from_str(env, "Criteria 1"),
            is_required: true,
        }],
    );
    client.checklist_define_criteria(owner, &gid, &0, &criteria);
    let evidence = Vec::from_array(env, [Some(String::from_str(env, "https://evidence.com"))]);
    client.checklist_submit(owner, &gid, &0, &evidence);
    client.checklist_review_criterion(reviewer, &gid, &0, &0u32, &true);

    let now = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(now + COMMUNITY_REVIEW_PERIOD + 1);
    client.milestone_vote(&gid, &0, reviewer, &true, &None);
    gid
}

// ── Dispute resolution tests ─────────────────────────────────────────────────

#[test]
fn test_dispute_raise_and_resolve_for_contributor() {
    let (env, client, _admin, owner, reviewer, funder, tok, tok_adm) = make_env_client_token();

    let gid =
        create_funded_submitted_voted(&env, &client, &owner, &reviewer, &funder, &tok, &tok_adm);

    // Milestone should be approved after quorum
    let m = client.get_milestone(&gid, &0);
    assert_eq!(m.state, MilestoneState::Approved);

    // Raise a dispute
    client.dispute_raise(&gid, &0, &owner, &String::from_str(&env, "Quality concern"));

    let dispute = client.get_dispute_record(&gid, &0);
    assert!(dispute.is_some());
    assert_eq!(dispute.unwrap().status, DisputeStatus::Open);

    // Assign arbiter
    let arbiter = Address::generate(&env);
    client.dispute_assign_arbiter(&gid, &0, &_admin, &arbiter);

    // Arbiter votes in favor of contributor
    client.dispute_arbiter_vote(&gid, &0, &arbiter, &true);

    // Issue #977: a contributor win must move the disputed milestone's exact
    // amount from escrow to the grant owner and leave the funder untouched —
    // assert the real fund movement, not just the returned status enum.
    let tok_client = token::Client::new(&env, &tok);
    let milestone_amount = client.get_milestone(&gid, &0).amount;
    assert!(milestone_amount > 0);
    let owner_before = tok_client.balance(&owner);
    let funder_before = tok_client.balance(&funder);
    let escrow_before = client.get_grant(&gid).escrow_balance;

    let outcome = client.dispute_resolve(&gid, &0, &_admin);
    assert_eq!(outcome, DisputeStatus::ResolvedForContributor);

    assert_eq!(
        tok_client.balance(&owner) - owner_before,
        milestone_amount,
        "owner (contributor) receives exactly milestone.amount"
    );
    assert_eq!(
        tok_client.balance(&funder),
        funder_before,
        "funder balance unchanged on a contributor win"
    );
    assert_eq!(
        client.get_grant(&gid).escrow_balance,
        escrow_before - milestone_amount
    );
}

#[test]
fn test_dispute_raise_and_resolve_for_funder() {
    let (env, client, admin, owner, reviewer, funder, tok, tok_adm) = make_env_client_token();

    let gid =
        create_funded_submitted_voted(&env, &client, &owner, &reviewer, &funder, &tok, &tok_adm);

    client.dispute_raise(&gid, &0, &owner, &String::from_str(&env, "No show"));

    let arbiter = Address::generate(&env);
    client.dispute_assign_arbiter(&gid, &0, &admin, &arbiter);
    client.dispute_arbiter_vote(&gid, &0, &arbiter, &false);

    // Issue #977: a funder win must refund the disputed milestone's exact
    // amount from escrow back to the funder and leave the grant owner untouched.
    let tok_client = token::Client::new(&env, &tok);
    let milestone_amount = client.get_milestone(&gid, &0).amount;
    assert!(milestone_amount > 0);
    let owner_before = tok_client.balance(&owner);
    let funder_before = tok_client.balance(&funder);
    let escrow_before = client.get_grant(&gid).escrow_balance;

    let outcome = client.dispute_resolve(&gid, &0, &admin);
    assert_eq!(outcome, DisputeStatus::ResolvedForFunder);

    assert_eq!(
        tok_client.balance(&funder) - funder_before,
        milestone_amount,
        "funder is refunded exactly milestone.amount"
    );
    assert_eq!(
        tok_client.balance(&owner),
        owner_before,
        "owner balance unchanged on a funder win"
    );
    assert_eq!(
        client.get_grant(&gid).escrow_balance,
        escrow_before - milestone_amount
    );
}

#[test]
fn test_zero_amount_dispute_requires_no_special_balance() {
    let (env, client, _admin, owner, reviewer, funder, tok, tok_adm) = make_env_client_token();

    let gid =
        create_funded_submitted_voted(&env, &client, &owner, &reviewer, &funder, &tok, &tok_adm);

    client.dispute_raise(&gid, &0, &owner, &String::from_str(&env, "Dispute"));

    let m = client.get_milestone(&gid, &0);
    assert_eq!(m.state, MilestoneState::Approved);
}

#[test]
fn test_only_reviewer_or_owner_can_raise_dispute() {
    let (env, client, _admin, owner, reviewer, funder, tok, tok_adm) = make_env_client_token();

    let gid =
        create_funded_submitted_voted(&env, &client, &owner, &reviewer, &funder, &tok, &tok_adm);

    // A random address that is neither owner nor reviewer should fail
    let outsider = Address::generate(&env);
    let result =
        client.try_dispute_raise(&gid, &0, &outsider, &String::from_str(&env, "Unauthorized"));
    assert!(result.is_err());
}
