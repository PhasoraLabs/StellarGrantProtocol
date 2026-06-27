#[cfg(test)]
mod tests {
    use crate::audit;
    use crate::storage::Storage;
    use crate::types::{
        AmendmentStatus, AuditAction, ContractError, Grant, GrantFund, GrantStatus, Milestone,
        MilestoneState,
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
                require_compliance: None,
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
                deadline: None,
                reviewer_count_snapshot: 1,
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
            require_compliance: None,
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
    fn test_audit_log_grows_on_actions() {
        let env = Env::default();
        let (_, _, contract_id) = setup_test(&env);
        let grant_id = 1u64;
        let actor = Address::generate(&env);

        env.as_contract(&contract_id, || {
            audit::log(
                &env,
                grant_id,
                AuditAction::GrantCreated,
                &actor,
                None,
                Some(1000),
            );
            audit::log(
                &env,
                grant_id,
                AuditAction::GrantFunded,
                &actor,
                None,
                Some(500),
            );
            audit::log(
                &env,
                grant_id,
                AuditAction::MilestoneSubmitted,
                &actor,
                Some(0),
                Some(100),
            );

            assert_eq!(audit::log_length(&env, grant_id), 3);
        });
    }

    #[test]
    fn test_audit_get_log_returns_all_entries() {
        let env = Env::default();
        let (_, _, contract_id) = setup_test(&env);
        let grant_id = 1u64;
        let actor = Address::generate(&env);

        env.as_contract(&contract_id, || {
            audit::log(
                &env,
                grant_id,
                AuditAction::GrantCreated,
                &actor,
                None,
                None,
            );
            audit::log(
                &env,
                grant_id,
                AuditAction::GrantFunded,
                &actor,
                None,
                Some(100),
            );
            audit::log(
                &env,
                grant_id,
                AuditAction::MilestoneSubmitted,
                &actor,
                Some(0),
                None,
            );

            let log = audit::get_log(&env, grant_id);
            assert_eq!(log.len(), 3);
            assert_eq!(log.get(0).unwrap().action, AuditAction::GrantCreated);
            assert_eq!(log.get(1).unwrap().action, AuditAction::GrantFunded);
            assert_eq!(log.get(2).unwrap().action, AuditAction::MilestoneSubmitted);
        });
    }

    #[test]
    fn test_audit_get_recent_respects_limit() {
        let env = Env::default();
        let (_, _, contract_id) = setup_test(&env);
        let grant_id = 1u64;
        let actor = Address::generate(&env);

        env.as_contract(&contract_id, || {
            audit::log(
                &env,
                grant_id,
                AuditAction::GrantCreated,
                &actor,
                None,
                None,
            );
            audit::log(&env, grant_id, AuditAction::GrantFunded, &actor, None, None);
            audit::log(
                &env,
                grant_id,
                AuditAction::MilestoneSubmitted,
                &actor,
                Some(0),
                None,
            );
            audit::log(
                &env,
                grant_id,
                AuditAction::MilestoneApproved,
                &actor,
                Some(0),
                None,
            );
            audit::log(
                &env,
                grant_id,
                AuditAction::GrantCancelled,
                &actor,
                None,
                None,
            );

            let recent = audit::get_recent(&env, grant_id, 3);
            assert_eq!(recent.len(), 3);
            assert_eq!(
                recent.get(0).unwrap().action,
                AuditAction::MilestoneSubmitted
            );
            assert_eq!(
                recent.get(1).unwrap().action,
                AuditAction::MilestoneApproved
            );
            assert_eq!(recent.get(2).unwrap().action, AuditAction::GrantCancelled);
        });
    }

    #[test]
    fn test_fund_batch_empty_returns_error() {
        let env = Env::default();
        let (client, _, _) = setup_test(&env);
        let funder = Address::generate(&env);
        let items: Vec<(u64, i128)> = Vec::new(&env);

        env.mock_all_auths();
        let result = client.try_fund_batch(&funder, &items);
        assert_eq!(result, Err(Ok(ContractError::BatchEmpty.into())));
    }

    #[test]
    fn test_grant_create_appends_audit_entry() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, _) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let owner = Address::generate(&env);

        let grant_id = client.grant_create(
            &owner,
            &String::from_str(&env, "Title"),
            &String::from_str(&env, "Description"),
            &token_id,
            &1000,
            &100,
            &10,
            &Vec::new(&env),
        );

        let log = client.get_audit_log(&grant_id);
        assert_eq!(log.len(), 1);
        assert_eq!(log.get(0).unwrap().action, AuditAction::GrantCreated);
        assert_eq!(log.get(0).unwrap().actor, owner);
        assert_eq!(log.get(0).unwrap().amount, Some(1000));
    }

    #[test]
    fn test_milestone_vote_approved_appends_audit_entry() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _, contract_id) = setup_test(&env);
        let grant_id = 1;
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);

        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        create_grant(&env, &contract_id, grant_id, owner, token, reviewers);
        create_milestone(&env, &contract_id, grant_id, 0, MilestoneState::Submitted);

        client.milestone_vote(&grant_id, &0, &reviewer, &true, &None);

        let log = client.get_audit_log(&grant_id);
        assert_eq!(log.len(), 1);
        assert_eq!(log.get(0).unwrap().action, AuditAction::MilestoneApproved);
    }

    fn create_client_grant(
        env: &Env,
        client: &StellarGrantsContractClient<'_>,
        owner: &Address,
        token: &Address,
        reviewers: Vec<Address>,
    ) -> u64 {
        client.grant_create(
            owner,
            &String::from_str(env, "Title"),
            &String::from_str(env, "Description"),
            token,
            &1000,
            &500,
            &2,
            &reviewers,
        )
    }

    #[test]
    fn test_syndicate_two_members_split_50_50() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);
        let owner = Address::generate(&env);
        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token_id, Vec::new(&env));

        token_admin.mint(&member1, &500);
        token_admin.mint(&member2, &500);

        client.form_syndicate(&owner, &grant_id, &1000, &100, &5, &10);
        client.join_syndicate(&member1, &grant_id, &500);
        client.join_syndicate(&member2, &grant_id, &500);
        client.close_syndicate(&owner, &grant_id);

        let first = client.get_syndicate_member(&grant_id, &member1).unwrap();
        let second = client.get_syndicate_member(&grant_id, &member2).unwrap();
        assert_eq!(first.share_bps, 5000);
        assert_eq!(second.share_bps, 5000);
        assert_eq!(client.get_grant(&grant_id).escrow_balance, 1000);
    }

    #[test]
    fn test_syndicate_partial_funding_cannot_close() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);
        let owner = Address::generate(&env);
        let member = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token_id, Vec::new(&env));

        token_admin.mint(&member, &400);
        client.form_syndicate(&owner, &grant_id, &1000, &100, &5, &10);
        client.join_syndicate(&member, &grant_id, &400);

        let result = client.try_close_syndicate(&owner, &grant_id);
        assert_eq!(result, Err(Ok(ContractError::InvalidInput.into())));
    }

    #[test]
    fn test_syndicate_deadline_passed_member_can_withdraw() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);
        let owner = Address::generate(&env);
        let member = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token_id, Vec::new(&env));

        token_admin.mint(&member, &400);
        client.form_syndicate(&owner, &grant_id, &1000, &100, &5, &1);
        client.join_syndicate(&member, &grant_id, &400);
        env.ledger().set_sequence_number(env.ledger().sequence() + 2);

        assert_eq!(client.withdraw_syndicate(&member, &grant_id), 400);
        let token_client = token::Client::new(&env, &token_id);
        assert_eq!(token_client.balance(&member), 400);
    }

    #[test]
    fn test_versioning_create_propose_approve_apply_v2_exists() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_test(&env);
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let token = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        let grant_id = create_client_grant(&env, &client, &owner, &token, reviewers);

        let mut fields = Vec::new(&env);
        fields.push_back(String::from_str(&env, "title"));
        let mut values = Vec::new(&env);
        values.push_back(String::from_str(&env, "Updated Title"));

        let version = client.propose_amendment(
            &owner,
            &grant_id,
            &fields,
            &values,
            &String::from_str(&env, "Scope refined"),
        );
        assert_eq!(
            client.vote_amendment(&reviewer, &grant_id, &version, &true),
            AmendmentStatus::Approved
        );
        let v2 = client.apply_amendment(&grant_id, &version);
        assert_eq!(v2.version, 2);
        assert_eq!(v2.title, String::from_str(&env, "Updated Title"));
        assert_eq!(client.current_version(&grant_id), 2);
        assert_eq!(client.amendment_history(&grant_id).len(), 1);
    }

    #[test]
    fn test_versioning_rejected_amendment_no_new_version() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_test(&env);
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let token = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        let grant_id = create_client_grant(&env, &client, &owner, &token, reviewers);

        let mut fields = Vec::new(&env);
        fields.push_back(String::from_str(&env, "title"));
        let mut values = Vec::new(&env);
        values.push_back(String::from_str(&env, "Rejected Title"));

        let version = client.propose_amendment(
            &owner,
            &grant_id,
            &fields,
            &values,
            &String::from_str(&env, "Nope"),
        );
        assert_eq!(
            client.vote_amendment(&reviewer, &grant_id, &version, &false),
            AmendmentStatus::Rejected
        );
        let result = client.try_apply_amendment(&grant_id, &version);
        assert_eq!(result, Err(Ok(ContractError::InvalidState.into())));
        assert_eq!(client.current_version(&grant_id), 1);
    }

    #[test]
    fn test_versioning_get_v1_returns_original_spec() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_test(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));

        let v1 = client.get_version(&grant_id, &1).unwrap();
        assert_eq!(v1.version, 1);
        assert_eq!(v1.title, String::from_str(&env, "Title"));
        assert_eq!(v1.description, String::from_str(&env, "Description"));
        assert_eq!(v1.total_amount, 1000);
        assert_eq!(v1.total_milestones, 2);
    }
}
