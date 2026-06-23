#[cfg(test)]
mod tests {
    use crate::constants;
    use crate::storage::Storage;
    use crate::types::{
        BatchMilestoneVote, ContractError, Grant, GrantFund, GrantStatus, Milestone, MilestoneState,
    };
    use crate::StellarGrantsContract;
    use crate::StellarGrantsContractClient;
    use soroban_sdk::{testutils::Address as _, token, Address, Env, Map, String, Vec};

    fn setup_test(
        env: &Env,
    ) -> (
        StellarGrantsContractClient<'_>,
        Address,
        soroban_sdk::Address,
    ) {
        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        (client, admin, contract_id)
    }

    fn create_grant(
        env: &Env,
        contract_id: &soroban_sdk::Address,
        grant_id: u64,
        owner: Address,
        token: Address,
        reviewers: Vec<Address>,
    ) {
        env.as_contract(contract_id, || {
            let grant = Grant {
                id: grant_id,
                owner,
                title: String::from_str(env, "Title"),
                description: String::from_str(env, "Description"),
                token,
                status: GrantStatus::Active,
                total_amount: 1000,
                milestone_amount: 1000,
                reviewers,
                total_milestones: 1,
                milestones_paid_out: 0,
                escrow_balance: 1000,
                funders: Vec::new(env),
                reason: None,
                timestamp: env.ledger().timestamp(),
            };
            Storage::set_grant(env, grant_id, &grant);
        });
    }

    fn create_milestone(
        env: &Env,
        contract_id: &soroban_sdk::Address,
        grant_id: u64,
        milestone_idx: u32,
        state: MilestoneState,
    ) {
        env.as_contract(contract_id, || {
            let milestone = Milestone {
                idx: milestone_idx,
                description: String::from_str(env, "Description"),
                amount: 100,
                state,
                votes: Map::new(env),
                approvals: 0,
                rejections: 0,
                reasons: Map::new(env),
                status_updated_at: 0,
                proof_url: Some(String::from_str(env, "https://proof.url")),
                submission_timestamp: env.ledger().timestamp(),
            };
            Storage::set_milestone(env, grant_id, milestone_idx, &milestone);
        });
    }

    #[test]
    fn test_get_milestone_success() {
        let env = Env::default();
        let (client, _, contract_id) = setup_test(&env);
        let grant_id = 1;
        let milestone_idx = 0;
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);

        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        create_grant(&env, &contract_id, grant_id, owner, token, reviewers);
        create_milestone(
            &env,
            &contract_id,
            grant_id,
            milestone_idx,
            MilestoneState::Submitted,
        );

        let milestone = client.get_milestone(&grant_id, &milestone_idx);
        assert_eq!(milestone.state, MilestoneState::Submitted);
        assert_eq!(milestone.description, String::from_str(&env, "Description"));
    }

    #[test]
    fn test_get_milestone_grant_not_found() {
        let env = Env::default();
        let (client, _, _) = setup_test(&env);
        let result = client.try_get_milestone(&99, &0);
        assert_eq!(result, Err(Ok(ContractError::GrantNotFound.into())));
    }

    #[test]
    fn test_successful_vote() {
        let env = Env::default();
        let (client, _, contract_id) = setup_test(&env);
        let grant_id = 1;
        let milestone_idx = 0;
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);

        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        create_grant(&env, &contract_id, grant_id, owner, token, reviewers);
        create_milestone(
            &env,
            &contract_id,
            grant_id,
            milestone_idx,
            MilestoneState::Submitted,
        );

        env.mock_all_auths();
        let result = client.milestone_vote(&grant_id, &milestone_idx, &reviewer, &true, &None);

        assert_eq!(result, true); // Quorum reached (1/1)

        env.as_contract(&contract_id, || {
            let updated_milestone = Storage::get_milestone(&env, grant_id, milestone_idx).unwrap();
            assert_eq!(updated_milestone.approvals, 1);
            assert_eq!(updated_milestone.state, MilestoneState::Approved);
            assert!(updated_milestone.votes.get(reviewer).unwrap());
        });
    }

    #[test]
    fn test_grant_cancel_success_multiple_funders() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, contract_id) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let owner = Address::generate(&env);
        let funder1 = Address::generate(&env);
        let funder2 = Address::generate(&env);

        let total_funded = 1000i128;
        let fund1 = 600i128;
        let fund2 = 400i128;
        let remaining = 1000i128;
        let grant_id = 1u64;

        token_admin.mint(&contract_id, &remaining);

        let mut funders = Vec::new(&env);
        funders.push_back(GrantFund {
            funder: funder1.clone(),
            amount: fund1,
        });
        funders.push_back(GrantFund {
            funder: funder2.clone(),
            amount: fund2,
        });

        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(&env, "Title"),
            description: String::from_str(&env, "Description"),
            token: token_id.clone(),
            status: GrantStatus::Active,
            total_amount: total_funded,
            milestone_amount: 1000,
            reviewers: Vec::new(&env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: remaining,
            funders,
            reason: None,
            timestamp: env.ledger().timestamp(),
        };

        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, grant_id, &grant);
        });

        let reason = String::from_str(&env, "Project discontinued");
        client.grant_cancel(&grant_id, &owner, &reason);

        let token_client = token::Client::new(&env, &token_id);
        assert_eq!(token_client.balance(&funder1), 600);
        assert_eq!(token_client.balance(&funder2), 400);

        env.as_contract(&contract_id, || {
            let updated_grant = Storage::get_grant(&env, grant_id).unwrap();
            assert_eq!(updated_grant.status, GrantStatus::Cancelled);
        });
    }

    #[test]
    fn test_grant_cancel_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _, contract_id) = setup_test(&env);
        let owner = Address::generate(&env);
        let wrong_owner = Address::generate(&env);
        let token = Address::generate(&env);

        let grant_id = 1u64;
        create_grant(&env, &contract_id, grant_id, owner, token, Vec::new(&env));

        let reason = String::from_str(&env, "test");
        let result = client.try_grant_cancel(&grant_id, &wrong_owner, &reason);

        assert_eq!(result, Err(Ok(ContractError::Unauthorized.into())));
    }

    #[test]
    fn test_batch_vote_milestones_empty() {
        let env = Env::default();
        let (client, _, _) = setup_test(&env);
        let reviewer = Address::generate(&env);
        let votes = Vec::new(&env);

        env.mock_all_auths();
        let result = client.try_batch_vote_milestones(&reviewer, &votes);
        assert_eq!(result, Err(Ok(ContractError::BatchEmpty.into())));
    }

    #[test]
    fn test_batch_vote_milestones_oversized() {
        let env = Env::default();
        let (client, _, _) = setup_test(&env);
        let reviewer = Address::generate(&env);

        let mut votes = Vec::new(&env);
        for _ in 0..=constants::MAX_BATCH_SIZE {
            votes.push_back(BatchMilestoneVote {
                grant_id: 1,
                milestone_idx: 0,
                approve: true,
                reason: None,
            });
        }

        env.mock_all_auths();
        let result = client.try_batch_vote_milestones(&reviewer, &votes);
        assert_eq!(result, Err(Ok(ContractError::BatchTooLarge.into())));
    }

    #[test]
    fn test_batch_vote_milestones_all_success() {
        let env = Env::default();
        let (client, _, contract_id) = setup_test(&env);
        let grant_id = 1;
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);

        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        create_grant(&env, &contract_id, grant_id, owner, token, reviewers);
        create_milestone(&env, &contract_id, grant_id, 0, MilestoneState::Submitted);

        let mut votes = Vec::new(&env);
        votes.push_back(BatchMilestoneVote {
            grant_id,
            milestone_idx: 0,
            approve: true,
            reason: None,
        });

        env.mock_all_auths();
        let result = client.batch_vote_milestones(&reviewer, &votes);
        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 1);
        assert!(result.results.get(0).unwrap().success);
    }

    #[test]
    fn test_batch_vote_milestones_partial_failure() {
    fn test_grant_fund_zero_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _, contract_id) = setup_test(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let funder = Address::generate(&env);
        let grant_id = 1u64;

        create_grant(&env, &contract_id, grant_id, owner, token, Vec::new(&env));

        let result = client.try_grant_fund(&grant_id, &funder, &0);
        assert_eq!(result, Err(Ok(ContractError::ZeroAmount.into())));
    }

    #[test]
    fn test_milestone_index_out_of_bounds() {
        let env = Env::default();
        let (client, _, contract_id) = setup_test(&env);
        let grant_id = 1;
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);

        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        create_grant(&env, &contract_id, grant_id, owner, token, reviewers);
        create_milestone(&env, &contract_id, grant_id, 0, MilestoneState::Submitted);

        let mut votes = Vec::new(&env);
        votes.push_back(BatchMilestoneVote {
            grant_id,
            milestone_idx: 0,
            approve: true,
            reason: None,
        });
        votes.push_back(BatchMilestoneVote {
            grant_id: 99,
            milestone_idx: 0,
            approve: true,
            reason: None,
        });

        env.mock_all_auths();
        let result = client.batch_vote_milestones(&reviewer, &votes);
        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 1);
        assert!(result.results.get(0).unwrap().success);
        assert!(!result.results.get(1).unwrap().success);
        assert_eq!(
            result.results.get(1).unwrap().error_code,
            Some(ContractError::GrantNotFound as u32)
        );
    }

    #[test]
    fn test_batch_fund_grants_empty() {
        let env = Env::default();
        let (client, admin, _) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let funder = Address::generate(&env);
        let items = Vec::new(&env);

        env.mock_all_auths();
        let result = client.try_batch_fund_grants(&funder, &token_id, &items);
        assert_eq!(result, Err(Ok(ContractError::BatchEmpty.into())));
    }

    #[test]
    fn test_batch_fund_grants_partial_failure() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, contract_id) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let owner = Address::generate(&env);
        let funder = Address::generate(&env);
        let grant_id = 1u64;

        token_admin.mint(&funder, &500);

        create_grant(
            &env,
            &contract_id,
            grant_id,
            owner,
            token_id.clone(),
            Vec::new(&env),
        );

        let mut items = Vec::new(&env);
        items.push_back((grant_id, 100i128));
        items.push_back((99u64, 100i128));

        let result = client.batch_fund_grants(&funder, &token_id, &items);
        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 1);
        assert!(result.results.get(0).unwrap().success);
        assert!(!result.results.get(1).unwrap().success);
    }

    #[test]
    fn test_batch_cancel_grants_oversized() {
        let env = Env::default();
        let (client, _, _) = setup_test(&env);
        let caller = Address::generate(&env);
        let reason = String::from_str(&env, "cancel");

        let mut grant_ids = Vec::new(&env);
        for i in 0..=constants::MAX_BATCH_SIZE {
            grant_ids.push_back(i as u64);
        }

        env.mock_all_auths();
        let result = client.try_batch_cancel_grants(&caller, &grant_ids, &reason);
        assert_eq!(result, Err(Ok(ContractError::BatchTooLarge.into())));
    }
}
