use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger as _},
    Address, Env, String, Vec,
};
use stellar_grants::{
    AcceptanceCriteria, ConditionType, ContractError, MilestoneState, ProtocolModule,
    ReleaseCondition, StellarGrantsContractClient,
};

const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

fn timestamp_condition(env: &Env, threshold: i128) -> ReleaseCondition {
    ReleaseCondition {
        condition_type: ConditionType::TimestampAfter,
        threshold,
        oracle_token: None,
        custom_contract: None,
        custom_fn_name: None,
        description: String::from_str(env, "after t"),
    }
}

/// Issue #895: a full pause -> unpause cycle must leave the protocol usable.
#[test]
fn test_unpause_reopens_every_module_and_grant_create_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = <Address as TestAddress>::generate(&env);
    let owner = <Address as TestAddress>::generate(&env);
    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    client.initialize(&admin);
    client.set_global_admin(&admin, &admin);

    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let mut reviewers = Vec::new(&env);
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    let create = |c: &StellarGrantsContractClient| {
        c.try_grant_create(
            &owner,
            &String::from_str(&env, "G"),
            &String::from_str(&env, "D"),
            &token_id,
            &100,
            &10,
            &3,
            &reviewers,
        )
    };

    client.pause(&admin, &String::from_str(&env, "incident"));
    assert!(create(&client).is_err());

    client.unpause(&admin);
    assert!(!client.is_paused());
    assert_eq!(client.breaker_tripped_modules().len(), 0);
    assert!(client.breaker_is_open(&ProtocolModule::Grants));
    assert!(client.breaker_is_open(&ProtocolModule::Oracle));
    assert!(create(&client).is_ok());
}

struct Fixture {
    env: Env,
    contract_id: Address,
    owner: Address,
    reviewers: Vec<Address>,
    grant_id: u64,
}

fn fixture() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(stellar_grants::StellarGrantsContract, ());
    let client = StellarGrantsContractClient::new(&env, &contract_id);
    let owner = <Address as TestAddress>::generate(&env);
    let admin = <Address as TestAddress>::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(admin).address();
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&contract_id, &1000);
    token_admin.mint(&owner, &1000);

    let mut reviewers = Vec::new(&env);
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));
    reviewers.push_back(<Address as TestAddress>::generate(&env));

    let grant_id = client.grant_create(
        &owner,
        &String::from_str(&env, "G"),
        &String::from_str(&env, "D"),
        &token_id,
        &100,
        &10,
        &3,
        &reviewers,
    );
    client.grant_fund(&grant_id, &owner, &100);
    client.milestone_submit(
        &grant_id,
        &0,
        &owner,
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "proof"),
    );

    // Approving a milestone requires an already-satisfied checklist and an
    // elapsed community review period, independent of release conditions.
    env.ledger().set_timestamp(COMMUNITY_REVIEW_PERIOD + 1);
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
    client.checklist_review_criterion(&reviewers.get(0).unwrap(), &grant_id, &0, &0, &true);

    Fixture {
        env,
        contract_id,
        owner,
        reviewers,
        grant_id,
    }
}

/// Issue #896: an unmet attached condition blocks approval; once met the same
/// milestone is approved through normal voting.
#[test]
fn test_unmet_release_condition_blocks_approval_until_met() {
    let f = fixture();
    let client = StellarGrantsContractClient::new(&f.env, &f.contract_id);
    let mut conditions = Vec::new(&f.env);
    conditions.push_back(timestamp_condition(&f.env, 10_000_000));
    client.conditional_attach_conditions(&f.owner, &f.grant_id, &0, &conditions);

    let r0 = f.reviewers.get(0).unwrap();
    let r1 = f.reviewers.get(1).unwrap();

    assert_eq!(
        client.try_milestone_vote(&f.grant_id, &0, &r0, &true, &None),
        Err(Ok(ContractError::ConditionCheckFailed))
    );
    assert_eq!(
        client.get_milestone(&f.grant_id, &0).state,
        MilestoneState::Submitted
    );

    // Rejecting is never gated by release conditions.
    assert!(client
        .try_milestone_vote(&f.grant_id, &0, &r1, &false, &None)
        .is_ok());

    f.env.ledger().set_timestamp(20_000_000);
    assert!(client.conditional_all_met(&f.grant_id, &0));
    assert!(!client.milestone_vote(&f.grant_id, &0, &r0, &true, &None));
    let r2 = f.reviewers.get(2).unwrap();
    assert!(client.milestone_vote(&f.grant_id, &0, &r2, &true, &None));
    assert_eq!(
        client.get_milestone(&f.grant_id, &0).state,
        MilestoneState::Approved
    );
}
