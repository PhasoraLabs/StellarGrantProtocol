/// Tests for dispute resolution flow (#514).
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};
use stellar_grants::{DisputeStatus, MilestoneState, StellarGrantsContractClient};

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

    // Resolve
    let outcome = client.dispute_resolve(&gid, &0, &_admin);
    assert_eq!(outcome, DisputeStatus::ResolvedForContributor);
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

    let outcome = client.dispute_resolve(&gid, &0, &admin);
    assert_eq!(outcome, DisputeStatus::ResolvedForFunder);
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
