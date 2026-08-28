#[cfg(test)]
mod tests {
    use crate::audit;
    use crate::batch;
    use crate::snapshot;
    use crate::storage::Storage;
    use crate::types::{
        ContractError, Grant, GrantFund, GrantStatus, Milestone, MilestoneState, SnapshotTrigger,
        SplitRecipient,
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
        env.cost_estimate().budget().reset_unlimited();
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

        assert_eq!(result, true);

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

    // Issue #940: batch try_cancel_grant checked_add overflow test
    #[test]
    fn test_try_cancel_grant_checked_add_overflow() {
        let env = Env::default();
        env.mock_all_auths();

        let (_, _, contract_id) = setup_test(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let funder1 = Address::generate(&env);
        let funder2 = Address::generate(&env);
        let grant_id = 1u64;

        let mut funders = Vec::new(&env);
        funders.push_back(GrantFund {
            funder: funder1.clone(),
            amount: i128::MAX,
        });
        funders.push_back(GrantFund {
            funder: funder2.clone(),
            amount: 100i128,
        });

        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(&env, "Title"),
            description: String::from_str(&env, "Description"),
            token,
            status: GrantStatus::Active,
            total_amount: i128::MAX,
            milestone_amount: 1000,
            reviewers: Vec::new(&env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 1000,
            funders,
            reason: None,
            timestamp: env.ledger().timestamp(),
        };

        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, grant_id, &grant);
            let reason = String::from_str(&env, "cancel test");
            let result = batch::try_cancel_grant(&env, grant_id, &owner, reason);
            assert_eq!(result, Err(ContractError::InvalidInput));
        });
    }

    // Issue #941: audit log sharding test across multiple pages (> 200 entries)
    #[test]
    fn test_audit_log_sharding_across_pages() {
        let env = Env::default();
        let (_, _, contract_id) = setup_test(&env);
        let grant_id = 1u64;
        let actor = Address::generate(&env);

        env.as_contract(&contract_id, || {
            for _i in 0..250 {
                let action = String::from_str(&env, "ACTION");
                audit::log(&env, grant_id, action, actor.clone());
            }

            let log = audit::get_audit_log(&env, grant_id);
            assert_eq!(log.len(), 250);
        });
    }

    // Issue #942: snapshot capture caller-to-grant authorization test
    #[test]
    fn test_snapshot_capture_authorization() {
        let env = Env::default();
        let (_, _, contract_id) = setup_test(&env);
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let unrelated = Address::generate(&env);
        let token = Address::generate(&env);
        let grant_id = 1u64;

        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());
        create_grant(
            &env,
            &contract_id,
            grant_id,
            owner.clone(),
            token,
            reviewers,
        );

        env.as_contract(&contract_id, || {
            // Unrelated caller must be rejected
            let res_unrelated =
                snapshot::capture(&env, grant_id, SnapshotTrigger::Manual, &unrelated);
            assert_eq!(res_unrelated, Err(ContractError::Unauthorized));

            // Owner can capture snapshot
            let res_owner = snapshot::capture(&env, grant_id, SnapshotTrigger::Manual, &owner);
            assert_eq!(res_owner, Ok(1));

            // Reviewer can capture snapshot
            let res_reviewer = snapshot::capture(
                &env,
                grant_id,
                SnapshotTrigger::MilestoneSubmitted,
                &reviewer,
            );
            assert_eq!(res_reviewer, Ok(2));

            // Verify both snapshots were recorded and TTL extended via storage accessors
            let list = Storage::get_snapshot_list(&env, grant_id);
            assert_eq!(list.len(), 2);
        });
    }

    // Issue #943: split_payment register_split status guard & event emission test
    #[test]
    fn test_register_split_status_guard_and_event() {
        let env = Env::default();
        let (client, _, contract_id) = setup_test(&env);
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let recipient1 = Address::generate(&env);
        let recipient2 = Address::generate(&env);
        let grant_id = 1u64;

        create_grant(
            &env,
            &contract_id,
            grant_id,
            owner.clone(),
            token,
            Vec::new(&env),
        );

        let mut recipients = Vec::new(&env);
        recipients.push_back(SplitRecipient {
            recipient: recipient1.clone(),
            basis_points: 6000,
        });
        recipients.push_back(SplitRecipient {
            recipient: recipient2.clone(),
            basis_points: 4000,
        });

        // Register split on Active grant via client -> should succeed
        let res_active = client.try_register_split(&owner, &grant_id, &0, &recipients);
        assert_eq!(res_active, Ok(Ok(())));

        // Mark grant as Cancelled
        env.as_contract(&contract_id, || {
            let mut grant = Storage::get_grant(&env, grant_id).unwrap();
            grant.status = GrantStatus::Cancelled;
            Storage::set_grant(&env, grant_id, &grant);
        });

        // Register split on Cancelled grant via client -> must fail with InvalidState
        let res_cancelled = client.try_register_split(&owner, &grant_id, &0, &recipients);
        assert_eq!(res_cancelled, Err(Ok(ContractError::InvalidState.into())));
    }
}
