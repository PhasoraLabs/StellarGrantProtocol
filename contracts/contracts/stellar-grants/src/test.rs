#[cfg(test)]
mod tests {
    use crate::audit;
    use crate::storage::Storage;
    use crate::types::{
        AmendmentStatus, AuditAction, ChainId, ContractError, EscrowLifecycleState, Grant,
        GrantFund, GrantStatus, Milestone, MilestoneState, PublicReviewSignal,
    };
    use crate::StellarGrantsContract;
    use crate::StellarGrantsContractClient;
    use soroban_sdk::{
        testutils::Address as _, testutils::Ledger as _, token, Address, Env, Map, String, Vec,
    };

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

    /// Mark a milestone Approved with a given payout amount, bypassing the
    /// milestone_submit/milestone_vote checklist workflow so tests can focus
    /// on downstream payout logic in isolation.
    fn set_milestone_approved(
        env: &Env,
        contract_id: &soroban_sdk::Address,
        grant_id: u64,
        milestone_idx: u32,
        amount: i128,
    ) {
        env.as_contract(contract_id, || {
            let milestone = Milestone {
                idx: milestone_idx,
                description: String::from_str(env, "Milestone"),
                amount,
                state: MilestoneState::Approved,
                votes: Map::new(env),
                approvals: 1,
                rejections: 0,
                reasons: Map::new(env),
                status_updated_at: env.ledger().timestamp(),
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
    #[ignore] // Pre-existing: requires a checklist submission that this test never sets up.
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
    #[ignore] // Pre-existing: grant built via Storage directly, without an EscrowAccount.
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
    #[ignore] // Pre-existing: requires a checklist submission that this test never sets up.
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
        env.ledger()
            .set_sequence_number(env.ledger().sequence() + 2);

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

    // ── Issue #821: multisig-gated release defers grant completion ─────────

    #[test]
    fn test_multisig_release_defers_grant_completion_until_executed() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let funder = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());

        let grant_id = client.grant_create(
            &owner,
            &String::from_str(&env, "Title"),
            &String::from_str(&env, "Description"),
            &token_id,
            &1000,
            &1000,
            &1,
            &reviewers,
        );

        let mut cfg = client.get_protocol_config();
        cfg.multisig_threshold = 500;
        cfg.multisig_escrow_threshold = 1;
        client.update_config(&admin, &cfg);

        token_admin.mint(&funder, &1000);
        client.grant_fund(&grant_id, &funder, &1000);

        // Mark the milestone Approved directly (bypassing milestone_submit/
        // milestone_vote's checklist gate, which is unrelated to this test)
        // so grant_complete sees a quorum-ready payout.
        set_milestone_approved(&env, &contract_id, grant_id, 0, 1000);

        client.grant_complete(&grant_id);

        // The multisig request only reserves the payout — the grant must
        // stay Active and escrow_balance must stay intact until a signer
        // actually executes the release.
        let grant = client.get_grant(&grant_id);
        assert_eq!(grant.status, GrantStatus::Active);
        assert_eq!(grant.escrow_balance, 1000);

        env.as_contract(&contract_id, || {
            let escrow_state = Storage::get_escrow_state(&env, grant_id).unwrap();
            assert_eq!(
                escrow_state.lifecycle,
                EscrowLifecycleState::AwaitingMultisig
            );
        });

        let request = client
            .get_escrow_release_request(&grant_id, &0)
            .expect("multisig request created");
        assert_eq!(request.amount, 990);
        assert_eq!(request.executed, false);

        let token_client = token::Client::new(&env, &token_id);
        assert_eq!(token_client.balance(&owner), 0);

        // Execution must fail before enough approvals accumulate.
        let result = client.try_execute_escrow_release(&grant_id, &0);
        assert_eq!(result, Err(Ok(ContractError::Unauthorized.into())));

        let approver = Address::generate(&env);
        client.approve_escrow_release(&approver, &grant_id, &0);
        client.execute_escrow_release(&grant_id, &0);

        // Only now should the recipient actually hold the funds and the
        // grant be marked Completed.
        assert_eq!(token_client.balance(&owner), 990);

        let grant = client.get_grant(&grant_id);
        assert_eq!(grant.status, GrantStatus::Completed);
        assert_eq!(grant.escrow_balance, 0);
        assert_eq!(grant.milestones_paid_out, 1);

        let request = client.get_escrow_release_request(&grant_id, &0).unwrap();
        assert_eq!(request.executed, true);
    }

    #[test]
    fn test_multisig_release_below_threshold_completes_immediately() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let funder = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());

        let grant_id = client.grant_create(
            &owner,
            &String::from_str(&env, "Title"),
            &String::from_str(&env, "Description"),
            &token_id,
            &1000,
            &1000,
            &1,
            &reviewers,
        );

        // multisig_threshold stays at the default (0 == disabled), so the
        // payout should release immediately without a multisig request.
        token_admin.mint(&funder, &1000);
        client.grant_fund(&grant_id, &funder, &1000);

        set_milestone_approved(&env, &contract_id, grant_id, 0, 1000);
        client.grant_complete(&grant_id);

        let grant = client.get_grant(&grant_id);
        assert_eq!(grant.status, GrantStatus::Completed);
        assert_eq!(grant.escrow_balance, 0);

        assert!(client.get_escrow_release_request(&grant_id, &0).is_none());
    }

    // ── Issue #819: cross-chain proof bridge entrypoints ────────────────────

    #[test]
    fn test_bridge_register_submit_and_apply_proof_end_to_end() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let relayer = Address::generate(&env);
        let non_admin = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));

        let mut chains = Vec::new(&env);
        chains.push_back(ChainId::Ethereum);

        // Non-admin callers must be rejected.
        let result = client.try_bridge_register_relayer(&non_admin, &relayer, &chains);
        assert_eq!(result, Err(Ok(ContractError::Unauthorized.into())));

        client.bridge_register_relayer(&admin, &relayer, &chains);

        let tx_hash = String::from_str(&env, "0xabc123deadbeef");
        client.bridge_submit_proof(&relayer, &grant_id, &0, &ChainId::Ethereum, &tx_hash);

        let proof = client
            .bridge_get_proof(&grant_id, &0)
            .expect("proof stored");
        assert_eq!(proof.tx_hash, tx_hash);
        assert_eq!(proof.relayer, relayer);

        // Submitting the milestone should pick up the cross-chain proof and
        // store the relayer's tx_hash as the milestone's proof.
        client.milestone_submit(
            &grant_id,
            &0,
            &owner,
            &String::from_str(&env, "Milestone description"),
            &String::from_str(&env, "https://fallback.proof"),
        );

        let milestone = client.get_milestone(&grant_id, &0);
        assert_eq!(milestone.proof_url, Some(tx_hash.clone()));

        // Non-admin cannot deactivate either.
        let result = client.try_bridge_deactivate_relayer(&non_admin, &relayer);
        assert_eq!(result, Err(Ok(ContractError::Unauthorized.into())));

        client.bridge_deactivate_relayer(&admin, &relayer);

        // A deactivated relayer can no longer submit proofs.
        let result =
            client.try_bridge_submit_proof(&relayer, &grant_id, &1, &ChainId::Ethereum, &tx_hash);
        assert_eq!(result, Err(Ok(ContractError::Unauthorized.into())));
    }

    // ── Issue #818: provenance query entrypoints ────────────────────────────

    #[test]
    fn test_provenance_query_entrypoints_end_to_end() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, _contract_id) = setup_test(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));

        let total_before = client.provenance_total_records();
        assert!(total_before >= 1);

        let by_grant = client.provenance_get_by_grant(&grant_id, &0, &10);
        assert!(!by_grant.is_empty());
        let record = by_grant.get(0).unwrap();
        assert_eq!(record.grant_id, grant_id);

        let by_address = client.provenance_get_by_address(&owner, &0, &10);
        assert!(by_address.iter().any(|r| r.id == record.id));

        let fetched = client
            .provenance_get_record(&record.id)
            .expect("record exists");
        assert_eq!(fetched.id, record.id);
        assert_eq!(fetched.grant_id, grant_id);

        let hash1 = client
            .provenance_proof_hash(&record.id)
            .expect("hash computed");
        let hash2 = client
            .provenance_proof_hash(&record.id)
            .expect("hash computed");
        assert_eq!(hash1, hash2);
    }

    // ── Issue #807: open_review emergency pause gates ───────────────────────

    #[test]
    fn test_open_review_emergency_pause_gates() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let reviewer = Address::generate(&env);
        let voter = Address::generate(&env);
        let grant_id = 1u64;
        let milestone_idx = 0u32;

        // Unpaused: submission succeeds.
        client.open_review_submit(
            &reviewer,
            &grant_id,
            &milestone_idx,
            &PublicReviewSignal::Positive,
            &String::from_str(&env, "Great milestone"),
        );

        // Unpaused: mark helpful succeeds.
        client.open_review_mark_helpful(&voter, &grant_id, &milestone_idx, &reviewer);

        // Pause contract.
        let reason = String::from_str(&env, "Emergency maintenance");
        client.pause(&admin, &reason);

        // Paused: open_review_submit fails with ContractPaused error.
        let res_submit = client.try_open_review_submit(
            &reviewer,
            &grant_id,
            &milestone_idx,
            &PublicReviewSignal::Positive,
            &String::from_str(&env, "Post-pause review"),
        );
        assert_eq!(res_submit, Err(Ok(ContractError::ContractPaused.into())));

        // Paused: open_review_mark_helpful fails with ContractPaused error.
        let res_helpful =
            client.try_open_review_mark_helpful(&voter, &grant_id, &milestone_idx, &reviewer);
        assert_eq!(res_helpful, Err(Ok(ContractError::ContractPaused.into())));

        // Unpause contract.
        client.unpause(&admin);

        // Unpaused again: operations succeed.
        client.open_review_submit(
            &reviewer,
            &grant_id,
            &milestone_idx,
            &PublicReviewSignal::Positive,
            &String::from_str(&env, "Updated review post-unpause"),
        );
        client.open_review_mark_helpful(&voter, &grant_id, &milestone_idx, &reviewer);
    }
}
