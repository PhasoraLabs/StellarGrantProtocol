use soroban_sdk::testutils::Events;
use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger},
    token, Address, Env, String, Vec,
};
use stellar_grants::{AcceptanceCriteria, StellarGrantsContractClient};

const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

/// Issue #696: a milestone paid out through the normal `grant_complete` ->
/// `finalize_grant_release` flow must transition to `MilestoneState::Paid`,
/// not stay stuck at `Approved` — otherwise the contributor's portfolio
/// earnings and the grant's paid-out totals both silently report zero.
#[test]
fn test_paid_milestone_reflects_in_portfolio_earnings() {
    let env = Env::default();
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let token_admin_addr = <Address as TestAddress>::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token);
    let mut reviewers = Vec::new(&env);
    let reviewer = <Address as TestAddress>::generate(&env);
    reviewers.push_back(reviewer.clone());
    env.mock_all_auths();

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Paid Milestone Grant"),
        &String::from_str(&env, "Testing milestone payout state"),
        &token,
        &100,
        &100,
        &1,
        &reviewers,
    );

    let funder = <Address as TestAddress>::generate(&env);
    token_admin.mint(&funder, &100);
    client.grant_fund(&grant_id, &funder, &100);

    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "proof"),
    );

    let now = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(now + COMMUNITY_REVIEW_PERIOD + 1);

    // milestone_vote(approve=true) requires an already-satisfied checklist
    // (unrelated to issue #696) — a milestone with no checklist attached
    // otherwise can never be approved at all. Attach a single optional
    // criterion and review it so `all_required_met` flips to true.
    let mut criteria = Vec::new(&env);
    criteria.push_back(AcceptanceCriteria {
        idx: 0,
        description: String::from_str(&env, "Basic check"),
        is_required: false,
    });
    client.checklist_define_criteria(&owner, &grant_id, &0, &criteria);
    let mut evidence = Vec::new(&env);
    evidence.push_back(None);
    client.checklist_submit(&owner, &grant_id, &0, &evidence);
    client.checklist_review_criterion(&reviewer, &grant_id, &0, &0, &true);

    client.milestone_vote(&grant_id, &0, &reviewer, &true, &None);

    // Before grant_complete runs the real payout, portfolio earnings must
    // still be zero — Approved alone does not mean paid.
    let earnings_before = client.portfolio_earnings_by_token(&owner);
    assert_eq!(
        earnings_before.len(),
        0,
        "earnings should be zero before payout actually executes"
    );

    client.grant_complete(&grant_id);

    // `env.events().all()` only reflects the most recent top-level
    // invocation in this soroban-sdk version, so the emitted-event check
    // must happen right after the call that publishes it and before any
    // further client calls.
    let events = env.events().all();
    let mut found_milestone_paid = false;
    for e in events.events() {
        let s = format!("{:?}", e);
        if s.contains("milestone_paid") {
            found_milestone_paid = true;
        }
    }
    assert!(found_milestone_paid, "milestone_paid event not found");

    let milestones = client.export_milestones(&grant_id);
    let milestone = milestones.get(0).unwrap();
    assert_eq!(
        milestone.state,
        stellar_grants::MilestoneState::Paid,
        "milestone must transition to Paid once the real payout succeeds"
    );

    let earnings_after = client.portfolio_earnings_by_token(&owner);
    assert_eq!(earnings_after.len(), 1);
    let (earned_token, amount) = earnings_after.get(0).unwrap();
    assert_eq!(earned_token, token);
    assert_eq!(amount, 100);

    // The paid-out total exported for the grant must reflect the payout too.
    let exported = client.export_grants(&0, &10, &None);
    let exported_grant = exported
        .items
        .iter()
        .find(|g| g.id == grant_id)
        .expect("grant should be present in export");
    assert_eq!(exported_grant.paid_out, 100);
}
