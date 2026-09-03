use soroban_sdk::testutils::Events;
use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger},
    token, Address, Env, String, Vec,
};
use stellar_grants::{
    AcceptanceCriteria, NotificationEvent, StellarGrantsContractClient, SubscriptionScope,
};

const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

fn count_notification_events(env: &Env) -> usize {
    let events = env.events().all();
    let mut count = 0;
    for e in events.events() {
        if format!("{:?}", e).contains("notification") {
            count += 1;
        }
    }
    count
}

/// Issue #699: `notification::emit_notification` existed but was never called
/// from anywhere in the crate, so a subscribed address never actually
/// received a notification for any grant/milestone/dispute lifecycle event.
#[test]
fn test_new_grant_notification_emitted_to_contributor_subscriber() {
    let env = Env::default();
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let subscriber = <Address as TestAddress>::generate(&env);
    let token_admin_addr = <Address as TestAddress>::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr.clone())
        .address();
    let reviewers = Vec::new(&env);
    env.mock_all_auths();

    client.subscribe(
        &subscriber,
        &NotificationEvent::NewGrant,
        &SubscriptionScope::PerContributor(owner.clone()),
    );
    assert!(client.is_subscribed(
        &subscriber,
        &NotificationEvent::NewGrant,
        &SubscriptionScope::PerContributor(owner.clone()),
    ));

    client.grant_create(
        &owner,
        &String::from_str(&env, "Notify Grant"),
        &String::from_str(&env, "Testing notifications"),
        &token,
        &100,
        &10,
        &1,
        &reviewers,
    );

    assert!(
        count_notification_events(&env) >= 1,
        "expected a notification event to be published on grant creation"
    );
}

/// Covers MilestoneSubmitted and MilestoneApproved: a subscriber watching a
/// specific grant must see both fire as the milestone moves through its
/// lifecycle.
#[test]
fn test_milestone_lifecycle_notifications_emitted_to_grant_subscriber() {
    let env = Env::default();
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let subscriber = <Address as TestAddress>::generate(&env);
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
        &String::from_str(&env, "Notify Grant"),
        &String::from_str(&env, "Testing notifications"),
        &token,
        &100,
        &10,
        &1,
        &reviewers,
    );

    client.subscribe(
        &subscriber,
        &NotificationEvent::MilestoneSubmitted,
        &SubscriptionScope::PerGrant(grant_id),
    );
    client.subscribe(
        &subscriber,
        &NotificationEvent::MilestoneApproved,
        &SubscriptionScope::PerGrant(grant_id),
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
    // `env.events().all()` only reflects the most recent top-level
    // invocation in this soroban-sdk version, so this check must happen
    // right after the call that publishes the event.
    assert!(
        count_notification_events(&env) >= 1,
        "expected a notification event on milestone submission"
    );

    let now = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(now + COMMUNITY_REVIEW_PERIOD + 1);

    // milestone_vote(approve=true) requires an already-satisfied checklist
    // (unrelated to issue #699) — attach and clear a single optional
    // criterion so `all_required_met` flips to true.
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

    assert!(
        count_notification_events(&env) >= 1,
        "expected a notification event on milestone approval"
    );

    assert!(client
        .get_subscribers(
            &NotificationEvent::MilestoneApproved,
            &SubscriptionScope::PerGrant(grant_id),
        )
        .contains(subscriber));
}

/// Covers DisputeRaised.
#[test]
fn test_dispute_raised_notification_emitted_to_grant_subscriber() {
    let env = Env::default();
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let subscriber = <Address as TestAddress>::generate(&env);
    let token_admin_addr = <Address as TestAddress>::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin_addr.clone())
        .address();
    let reviewers = Vec::new(&env);
    env.mock_all_auths();

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "Dispute Grant"),
        &String::from_str(&env, "Testing notifications"),
        &token,
        &100,
        &10,
        &1,
        &reviewers,
    );

    client.subscribe(
        &subscriber,
        &NotificationEvent::DisputeRaised,
        &SubscriptionScope::PerGrant(grant_id),
    );

    client.dispute_raise(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "milestone quality dispute"),
    );

    assert!(
        count_notification_events(&env) >= 1,
        "expected a notification event to be published on dispute_raise"
    );
}
