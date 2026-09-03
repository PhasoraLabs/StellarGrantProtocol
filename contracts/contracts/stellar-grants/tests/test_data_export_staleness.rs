/// Issue #817: data_export::set_last_updated was only ever called from its
/// own unit tests, so export_grants's last_updated_after filter, plus
/// last_global_update and state_fingerprint, never reflected real grant or
/// milestone activity. These tests drive a real grant through the public
/// contract entrypoints and confirm the staleness tracking now advances.
use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger as _},
    Address, Env, String, Vec,
};
use stellar_grants::{StellarGrantsContract, StellarGrantsContractClient};

fn setup() -> (Env, StellarGrantsContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);
    (env, client, owner, admin)
}

fn make_reviewers(env: &Env, count: u32) -> Vec<Address> {
    let mut reviewers = Vec::new(env);
    for _ in 0..count {
        reviewers.push_back(<Address as TestAddress>::generate(env));
    }
    reviewers
}

#[test]
fn test_milestone_submission_advances_last_updated_and_export_filter() {
    let (env, client, owner, admin) = setup();
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let reviewers = make_reviewers(&env, 1);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Export Grant"),
        &String::from_str(&env, "Testing staleness tracking"),
        &token_id,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&owner, &1000);
    client.grant_fund(&grant_id, &owner, &1000);

    // Record a baseline timestamp before any real mutation past creation.
    let baseline = client.last_global_update();
    let fingerprint_before = client.state_fingerprint();

    // Advance ledger time so the milestone submission timestamp is observably later.
    env.ledger().with_mut(|li| li.timestamp += 100);

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "Milestone 0"),
        &String::from_str(&env, "https://proof.example.com"),
    );

    // last_global_update must have advanced past the baseline.
    let after_submit = client.last_global_update();
    assert!(after_submit > baseline);

    // The fingerprint must change once a real milestone mutation happens.
    let fingerprint_after = client.state_fingerprint();
    assert_ne!(fingerprint_before, fingerprint_after);

    // export_grants with last_updated_after set to the baseline must now
    // include this grant, since it was updated after that timestamp.
    let page = client.export_grants(&0, &10, &Some(baseline));
    let found = page.items.iter().any(|g| g.id == grant_id);
    assert!(
        found,
        "grant should appear in incremental export after mutation"
    );

    // Filtering with a cutoff at or after the submission timestamp excludes it again.
    let page_after = client.export_grants(&0, &10, &Some(after_submit));
    let found_after = page_after.items.iter().any(|g| g.id == grant_id);
    assert!(
        !found_after,
        "grant should not appear once cutoff is at/after its last update"
    );
}

#[test]
fn test_grant_cancellation_advances_last_updated() {
    let (env, client, owner, admin) = setup();
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let reviewers = make_reviewers(&env, 2);

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Cancellation Grant"),
        &String::from_str(&env, "Testing staleness tracking"),
        &token_id,
        &1000,
        &1000,
        &1,
        &reviewers,
    );

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&owner, &1000);
    client.grant_fund(&grant_id, &owner, &1000);

    let before_cancel = client.last_global_update();
    env.ledger().with_mut(|li| li.timestamp += 100);

    client.grant_cancel(
        &grant_id,
        &owner,
        &String::from_str(&env, "No longer needed"),
    );

    let after_cancel = client.last_global_update();
    assert!(after_cancel > before_cancel);
}
