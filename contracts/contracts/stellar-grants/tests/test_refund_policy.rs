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
