use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};
use stellar_grants::{RefundPolicy, RefundPolicyType, StellarGrantsContractClient};

#[test]
fn test_time_weighted_refund_policy_on_partial_cancel() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token);
    let token_client = token::Client::new(&env, &token);
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

    // `refund::set_policy` requires escrow_balance == 0, so this must happen
    // before the grant is funded.
    let policy = RefundPolicy {
        grant_id,
        policy_type: RefundPolicyType::TimeWeighted,
        penalty_bps: 0,
        grace_period_ledgers: 0,
        min_refund_pct_bps: 0,
    };
    client.refund_set_policy(&owner, &grant_id, &policy);

    let funder = Address::generate(&env);
    token_admin.mint(&funder, &1000);
    client.grant_fund(&grant_id, &funder, &1000);

    // Advance to the halfway point of the (total_milestones * 10_000)-second
    // time-weighted window so the refund split is a clean 50/50.
    let grant = client.get_grant(&grant_id);
    let start = grant.timestamp;
    env.ledger().with_mut(|li| li.timestamp = start + 5_000);

    let funder_before = token_client.balance(&funder);
    let owner_before = token_client.balance(&owner);

    client.grant_cancel(
        &grant_id,
        &owner,
        &String::from_str(&env, "no longer needed"),
    );

    let funder_refund = token_client.balance(&funder) - funder_before;
    let owner_compensation = token_client.balance(&owner) - owner_before;

    assert_eq!(funder_refund, 500);
    assert_eq!(owner_compensation, 500);
    // No double-payout: the two payouts must exactly cover the gross escrow.
    assert_eq!(funder_refund + owner_compensation, 1000);

    let cancelled_grant = client.get_grant(&grant_id);
    assert_eq!(cancelled_grant.escrow_balance, 0);
}

#[test]
fn test_cancel_without_policy_falls_back_to_full_refund() {
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
    let token_client = token::Client::new(&env, &token);
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

    let funder_before = token_client.balance(&funder);

    // No policy was ever set — cancellation must use the original flat
    // escrow::refund_all behavior, refunding the funder in full.
    client.grant_cancel(
        &grant_id,
        &owner,
        &String::from_str(&env, "no longer needed"),
    );

    assert_eq!(token_client.balance(&funder) - funder_before, 1000);
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #976: `test_refund_policy.rs` previously exercised only
// `RefundPolicyType::TimeWeighted` and the no-policy fallback. The tests below
// give each of the five `RefundPolicyType` variants dedicated end-to-end
// coverage through the real `grant_cancel` → `refund::execute_refund` path,
// asserting the exact split between the funder and the grant owner and that the
// two payouts always sum to the gross escrow (no double-payout, no leak).
// ─────────────────────────────────────────────────────────────────────────────

/// Set up a contract with a single-milestone grant of `1000`, apply `policy`,
/// fund it with `1000`, and return `(env, client, token_client, grant_id, owner,
/// funder)`. `refund_set_policy` requires `escrow_balance == 0`, so the policy is
/// applied before funding.
fn setup_funded_grant_with_policy(
    policy_type: RefundPolicyType,
    penalty_bps: u32,
    min_refund_pct_bps: u32,
) -> (
    Env,
    StellarGrantsContractClient<'static>,
    token::Client<'static>,
    u64,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr)
        .address();

    // Leak the env to `'static` so the clients can be returned (same pattern as
    // tests/test_reputation_and_dispute_fee.rs::make_env_client_token).
    let env_ref: &'static Env = unsafe { &*(&env as *const Env) };
    let token_admin = token::StellarAssetClient::new(env_ref, &token);
    let token_client = token::Client::new(env_ref, &token);
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(env_ref, &contract_id);
    client.initialize(&admin);

    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer);

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

    let policy = RefundPolicy {
        grant_id,
        policy_type,
        penalty_bps,
        grace_period_ledgers: 0,
        min_refund_pct_bps,
    };
    client.refund_set_policy(&owner, &grant_id, &policy);

    let funder = Address::generate(&env);
    token_admin.mint(&funder, &1000);
    client.grant_fund(&grant_id, &funder, &1000);

    (env, client, token_client, grant_id, owner, funder)
}

#[test]
fn test_full_refund_policy_returns_entire_escrow_to_funder() {
    let (env, client, token_client, grant_id, owner, funder) =
        setup_funded_grant_with_policy(RefundPolicyType::FullRefund, 0, 0);

    let funder_before = token_client.balance(&funder);
    let owner_before = token_client.balance(&owner);

    client.grant_cancel(&grant_id, &owner, &String::from_str(&env, "cancelled"));

    let funder_refund = token_client.balance(&funder) - funder_before;
    let owner_compensation = token_client.balance(&owner) - owner_before;

    assert_eq!(funder_refund, 1000);
    assert_eq!(owner_compensation, 0);
    assert_eq!(funder_refund + owner_compensation, 1000);
    assert_eq!(client.get_grant(&grant_id).escrow_balance, 0);
}

#[test]
fn test_proportional_to_remaining_refunds_unreleased_escrow() {
    // With zero milestones paid out, the entire escrow is still "unreleased",
    // so ProportionalToRemaining refunds the funder in full — gross * (N - 0) / N.
    // The partial-payout ratio is covered by the unit tests in src/refund.rs,
    // since no mainline entry point increments `milestones_paid_out` without
    // also completing (and thereby closing) the grant.
    let (env, client, token_client, grant_id, owner, funder) =
        setup_funded_grant_with_policy(RefundPolicyType::ProportionalToRemaining, 0, 0);

    let funder_before = token_client.balance(&funder);
    let owner_before = token_client.balance(&owner);

    client.grant_cancel(&grant_id, &owner, &String::from_str(&env, "cancelled"));

    let funder_refund = token_client.balance(&funder) - funder_before;
    let owner_compensation = token_client.balance(&owner) - owner_before;

    assert_eq!(funder_refund, 1000);
    assert_eq!(owner_compensation, 0);
    assert_eq!(funder_refund + owner_compensation, 1000);
    assert_eq!(client.get_grant(&grant_id).escrow_balance, 0);
}

#[test]
fn test_penalty_on_cancel_applies_penalty_bps_and_splits_remainder() {
    // penalty_bps = 2_000 (20%): the funder is refunded 80% and the configured
    // penalty (20%) goes to the grant owner as contributor compensation.
    let (env, client, token_client, grant_id, owner, funder) =
        setup_funded_grant_with_policy(RefundPolicyType::PenaltyOnCancel, 2_000, 0);

    let funder_before = token_client.balance(&funder);
    let owner_before = token_client.balance(&owner);

    client.grant_cancel(&grant_id, &owner, &String::from_str(&env, "cancelled"));

    let funder_refund = token_client.balance(&funder) - funder_before;
    let owner_compensation = token_client.balance(&owner) - owner_before;

    assert_eq!(funder_refund, 800);
    assert_eq!(owner_compensation, 200);
    assert_eq!(funder_refund + owner_compensation, 1000);
    assert_eq!(client.get_grant(&grant_id).escrow_balance, 0);
}

#[test]
fn test_no_refund_policy_sends_full_escrow_to_owner_and_zero_to_funder() {
    // NoRefund is the highest-risk branch: a funder cancelling under this policy
    // must receive exactly 0, with the whole escrow going to the grant owner —
    // no panic, no leak to the wrong party.
    let (env, client, token_client, grant_id, owner, funder) =
        setup_funded_grant_with_policy(RefundPolicyType::NoRefund, 0, 0);

    let funder_before = token_client.balance(&funder);
    let owner_before = token_client.balance(&owner);

    client.grant_cancel(&grant_id, &owner, &String::from_str(&env, "cancelled"));

    let funder_refund = token_client.balance(&funder) - funder_before;
    let owner_compensation = token_client.balance(&owner) - owner_before;

    assert_eq!(funder_refund, 0);
    assert_eq!(owner_compensation, 1000);
    assert_eq!(funder_refund + owner_compensation, 1000);
    assert_eq!(client.get_grant(&grant_id).escrow_balance, 0);
}

#[test]
fn test_no_refund_policy_with_min_refund_floor_still_pays_funder_the_floor() {
    // Even under NoRefund, an explicit `min_refund_pct_bps` floor must be
    // honored: with a 10% floor the funder gets 100 and the owner 900.
    let (env, client, token_client, grant_id, owner, funder) =
        setup_funded_grant_with_policy(RefundPolicyType::NoRefund, 0, 1_000);

    let funder_before = token_client.balance(&funder);
    let owner_before = token_client.balance(&owner);

    client.grant_cancel(&grant_id, &owner, &String::from_str(&env, "cancelled"));

    let funder_refund = token_client.balance(&funder) - funder_before;
    let owner_compensation = token_client.balance(&owner) - owner_before;

    assert_eq!(funder_refund, 100);
    assert_eq!(owner_compensation, 900);
    assert_eq!(funder_refund + owner_compensation, 1000);
    assert_eq!(client.get_grant(&grant_id).escrow_balance, 0);
}
