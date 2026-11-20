#[cfg(test)]
mod tests {
    use crate::audit;
    use crate::storage::Storage;
    use crate::types::{
        AmendmentStatus, AuditAction, AutoApproveConfig, BatchMilestoneVote, ChainId,
        ContractError, EscrowLifecycleState, Grant, GrantFund, GrantStatus, Milestone,
        MilestoneState, PublicReviewSignal, VotingMechanism,
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
                reviewer_list_snapshot: Vec::new(env),
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
                reviewer_list_snapshot: soroban_sdk::Vec::new(env),
            };
            Storage::set_milestone(env, grant_id, milestone_idx, &milestone);
        });
    }

    mod rejection_tests {
        extern crate std;

        use super::*;
        use crate::{events::MilestoneRejected, metrics, reviewer_reward, reviewer_sla};
        use soroban_sdk::{events::Event, testutils::Events as _, xdr};

        fn reviewers(env: &Env) -> Vec<Address> {
            Vec::from_array(
                env,
                [
                    Address::generate(env),
                    Address::generate(env),
                    Address::generate(env),
                ],
            )
        }

        fn setup<'a>(
            env: &'a Env,
            reviewers: &Vec<Address>,
            token: &Address,
        ) -> (StellarGrantsContractClient<'a>, Address) {
            let (client, _, contract) = setup_test(env);
            create_grant(
                env,
                &contract,
                1,
                Address::generate(env),
                token.clone(),
                reviewers.clone(),
            );
            create_milestone(env, &contract, 1, 0, MilestoneState::Submitted);
            env.as_contract(&contract, || {
                let mut milestone = Storage::get_milestone(env, 1, 0).unwrap();
                milestone.reviewer_count_snapshot = reviewers.len();
                milestone.reviewer_list_snapshot = reviewers.clone();
                Storage::set_milestone(env, 1, 0, &milestone);
            });
            (client, contract)
        }

        fn event_bodies(env: &Env, contract: &Address) -> std::vec::Vec<xdr::ContractEventBody> {
            env.events()
                .all()
                .filter_by_contract(contract)
                .events()
                .iter()
                .map(|event| event.body.clone())
                .collect()
        }

        fn assert_rejection_paths_match(sla_deadline: u64) {
            let env = Env::default();
            env.mock_all_auths();
            env.ledger().set_timestamp(100);
            let reviewers = reviewers(&env);
            let token = Address::generate(&env);
            let (vote_client, vote_contract) = setup(&env, &reviewers, &token);
            let (reject_client, reject_contract) = setup(&env, &reviewers, &token);
            for contract in [&vote_contract, &reject_contract] {
                env.as_contract(contract, || {
                    for reviewer in reviewers.iter() {
                        reviewer_sla::register_sla(&env, &reviewer, (1, 0), sla_deadline);
                    }
                });
            }
            let reason = String::from_str(&env, "Required evidence is missing");
            for idx in 0..2 {
                let reviewer = reviewers.get(idx).unwrap();
                let expected_quorum = idx == 1;
                assert_eq!(
                    vote_client.milestone_vote(&1, &0, &reviewer, &false, &Some(reason.clone())),
                    expected_quorum
                );
                let vote_events = event_bodies(&env, &vote_contract);
                assert_eq!(
                    reject_client.milestone_reject(&1, &0, &reviewer, &reason),
                    expected_quorum
                );
                let mut reject_events = event_bodies(&env, &reject_contract);

                // The dedicated rejection API retains its original event in
                // addition to every event emitted by the shared voting path.
                let legacy_event = MilestoneRejected {
                    grant_id: 1,
                    milestone_idx: 0,
                    reviewer: reviewer.clone(),
                    reason: reason.clone(),
                    timestamp: 100,
                }
                .to_xdr(&env, &reject_contract);
                assert!(reject_events.contains(&legacy_event.body));
                let xdr::ContractEventBody::V0(legacy_body) = legacy_event.body;
                reject_events.retain(|body| {
                    let xdr::ContractEventBody::V0(body) = body;
                    body.topics != legacy_body.topics
                });
                assert_eq!(vote_events, reject_events);
                let notification_topic =
                    xdr::ScVal::Symbol(xdr::ScSymbol("notification".try_into().unwrap()));
                let notifications = vote_events
                    .iter()
                    .filter(|body| {
                        let xdr::ContractEventBody::V0(body) = body;
                        body.topics.first() == Some(&notification_topic)
                    })
                    .count();
                assert_eq!(notifications, usize::from(expected_quorum));

                let effects = |contract: &Address| {
                    env.as_contract(contract, || {
                        (
                            Storage::get_milestone(&env, 1, 0).unwrap(),
                            audit::get_audit_log(&env, 1),
                            metrics::get_metrics(&env),
                        )
                    })
                };
                let vote_effects = effects(&vote_contract);
                let reject_effects = effects(&reject_contract);
                assert_eq!(vote_effects, reject_effects);
                assert_eq!(
                    reject_effects.0.state,
                    if expected_quorum {
                        MilestoneState::Rejected
                    } else {
                        MilestoneState::Submitted
                    }
                );
                assert_eq!(reject_effects.1.len(), u32::from(expected_quorum));
                assert_eq!(
                    reject_effects.2.total_milestones_rejected,
                    u32::from(expected_quorum)
                );
                if expected_quorum {
                    let entry = reject_effects.1.get(0).unwrap();
                    assert_eq!(entry.action, AuditAction::MilestoneRejected);
                    assert_eq!(entry.actor, reviewer);
                    assert_eq!(entry.milestone_idx, Some(0));
                }

                let reviewer_effects = |contract: &Address| {
                    env.as_contract(contract, || {
                        let sla = reviewer_sla::get_sla(&env, &reviewer, (1, 0)).unwrap();
                        (
                            reviewer_reward::get_participation(&env, &reviewer, 1),
                            (sla.fulfilled, sla.breached),
                            Storage::get_reviewer_reputation(&env, reviewer.clone()),
                        )
                    })
                };
                let vote_effects = reviewer_effects(&vote_contract);
                let reject_effects = reviewer_effects(&reject_contract);
                assert_eq!(vote_effects, reject_effects);
                let breached = sla_deadline < 100;
                assert_eq!(reject_effects.1, (!breached, breached));
                if breached {
                    assert!(reject_effects.0.is_none());
                } else {
                    assert_eq!(reject_effects.0.unwrap().votes_cast, 1);
                }
            }
        }

        #[test]
        fn milestone_reject_matches_vote_before_and_after_quorum() {
            assert_rejection_paths_match(110);
        }

        #[test]
        fn milestone_reject_matches_vote_after_sla_deadline() {
            assert_rejection_paths_match(90);
        }

        #[test]
        fn milestone_reject_applies_shared_vote_validation() {
            let env = Env::default();
            env.mock_all_auths();
            let reviewers = reviewers(&env);
            let (client, contract) = setup(&env, &reviewers, &Address::generate(&env));
            let reviewer = reviewers.get(0).unwrap();
            let outsider = Address::generate(&env);
            let reason = String::from_str(&env, "Missing evidence");
            assert_eq!(
                client.try_milestone_reject(&1, &0, &outsider, &reason),
                Err(Ok(ContractError::Unauthorized))
            );
            // Adding a live reviewer must not bypass the submission snapshot.
            env.as_contract(&contract, || {
                let mut grant = Storage::get_grant(&env, 1).unwrap();
                grant.reviewers.push_back(outsider.clone());
                Storage::set_grant(&env, 1, &grant);
            });
            assert_eq!(
                client.try_milestone_reject(&1, &0, &outsider, &reason),
                Err(Ok(ContractError::Unauthorized))
            );
            let oversized = String::from_str(
                &env,
                &"x".repeat(crate::constants::MAX_BIO_LEN as usize + 1),
            );
            assert_eq!(
                client.try_milestone_reject(&1, &0, &reviewer, &oversized),
                Err(Ok(ContractError::InvalidInput))
            );
            assert!(!client.milestone_reject(&1, &0, &reviewer, &reason));
            assert_eq!(
                client.try_milestone_reject(&1, &0, &reviewer, &reason),
                Err(Ok(ContractError::AlreadyVoted))
            );
            env.as_contract(&contract, || {
                let mut grant = Storage::get_grant(&env, 1).unwrap();
                grant.status = GrantStatus::Cancelled;
                Storage::set_grant(&env, 1, &grant);
            });
            assert_eq!(
                client.try_milestone_reject(&1, &0, &reviewers.get(1).unwrap(), &reason),
                Err(Ok(ContractError::InvalidState))
            );
            env.as_contract(&contract, || {
                assert_eq!(
                    reviewer_reward::get_participation(&env, &reviewer, 1)
                        .unwrap()
                        .votes_cast,
                    1
                );
                assert_eq!(Storage::get_milestone(&env, 1, 0).unwrap().votes.len(), 1);
                assert_eq!(metrics::get_metrics(&env).total_milestones_rejected, 0);
            });
        }

        #[test]
        fn milestone_reject_requires_a_submitted_milestone() {
            let env = Env::default();
            env.mock_all_auths();
            let reviewers = reviewers(&env);
            let (client, contract) = setup(&env, &reviewers, &Address::generate(&env));
            create_milestone(&env, &contract, 1, 0, MilestoneState::Pending);
            assert_eq!(
                client.try_milestone_reject(
                    &1,
                    &0,
                    &reviewers.get(0).unwrap(),
                    &String::from_str(&env, "Missing evidence")
                ),
                Err(Ok(ContractError::MilestoneNotSubmitted))
            );
        }

        #[test]
        fn milestone_reject_preserves_pause_and_circuit_breaker_guards() {
            for guard in 0..3 {
                let env = Env::default();
                env.mock_all_auths();
                let reviewers = reviewers(&env);
                let (client, contract) = setup(&env, &reviewers, &Address::generate(&env));
                env.as_contract(&contract, || match guard {
                    0 => Storage::set_is_paused(&env, true),
                    1 => {
                        let owner = Storage::get_grant(&env, 1).unwrap().owner;
                        crate::grant_pause::pause(
                            &env,
                            &owner,
                            1,
                            String::from_str(&env, "Paused"),
                            None,
                        )
                        .unwrap();
                    }
                    _ => crate::circuit_breaker::trip_internal(
                        &env,
                        &reviewers.get(0).unwrap(),
                        crate::types::ProtocolModule::Grants,
                        String::from_str(&env, "Paused"),
                        None,
                    )
                    .unwrap(),
                });
                let expected = if guard == 2 {
                    ContractError::ModuleTripped
                } else {
                    ContractError::ContractPaused
                };
                assert_eq!(
                    client.try_milestone_reject(
                        &1,
                        &0,
                        &reviewers.get(0).unwrap(),
                        &String::from_str(&env, "Missing evidence")
                    ),
                    Err(Ok(expected))
                );
            }
        }
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

            let log = audit::get_log(&env, grant_id, 0, 50);
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

        let log = client.get_audit_log(&grant_id, &0, &50);
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

        let log = client.get_audit_log(&grant_id, &0, &50);
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

        // Issue #892: escrow_multisig::approve requires the caller be the
        // grant's owner, a registered reviewer, or the admin — use the
        // grant's own reviewer rather than an unrelated throwaway address.
        client.approve_escrow_release(&reviewer, &grant_id, &0);
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
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));
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

    // ── Issue #1024: auto-approved milestones get the manual approval effects ──

    #[test]
    fn test_auto_approve_applies_manual_approval_side_effects() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let grant_id = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));
        create_milestone(&env, &contract_id, grant_id, 0, MilestoneState::Submitted);

        // Majority approval with enough votes cast for the auto-approve gate.
        env.as_contract(&contract_id, || {
            let mut milestone = Storage::get_milestone(&env, grant_id, 0).unwrap();
            milestone.approvals = 2;
            milestone.rejections = 1;
            Storage::set_milestone(&env, grant_id, 0, &milestone);
        });

        client.auto_approve_set_config(
            &owner,
            &grant_id,
            &AutoApproveConfig {
                grant_id,
                enabled: true,
                grace_period_seconds: 0,
                min_votes_required: 1,
                set_by: owner.clone(),
                set_at: 0,
            },
        );

        let caller = Address::generate(&env);
        assert!(client.auto_approve_try(&caller, &grant_id, &0));

        // The auto-approved milestone must leave the same audit trail and mint
        // the same completion NFT a manual approval would.
        let log = client.get_audit_log(&grant_id, &0, &50);
        assert!(log
            .iter()
            .any(|entry| entry.action == AuditAction::MilestoneApproved));

        env.as_contract(&contract_id, || {
            assert!(Storage::get_milestone_nft(&env, grant_id, 0).is_some());
        });
    }

    // ── Issue #1026: quadratic voting respects milestone/grant state ─────────

    #[test]
    fn test_qv_vote_rejected_on_non_submitted_milestone() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());

        create_grant(&env, &contract_id, 1, owner, token, reviewers);
        create_milestone(&env, &contract_id, 1, 0, MilestoneState::Paid);
        env.as_contract(&contract_id, || {
            Storage::set_voting_mechanism(&env, 1, &VotingMechanism::Quadratic);
        });
        client.allocate_voice_credits(&admin, &reviewer, &1, &9);

        // A QV vote must not be able to re-finalize an already-Paid milestone.
        let result = client.try_milestone_vote(&1u64, &0u32, &reviewer, &false, &None);
        assert_eq!(result, Err(Ok(ContractError::MilestoneNotSubmitted.into())));
    }

    #[test]
    fn test_qv_vote_rejected_on_inactive_grant() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());

        create_grant(&env, &contract_id, 1, owner, token, reviewers);
        create_milestone(&env, &contract_id, 1, 0, MilestoneState::Submitted);
        env.as_contract(&contract_id, || {
            Storage::set_voting_mechanism(&env, 1, &VotingMechanism::Quadratic);
            let mut grant = Storage::get_grant(&env, 1).unwrap();
            grant.status = GrantStatus::Completed;
            Storage::set_grant(&env, 1, &grant);
        });
        client.allocate_voice_credits(&admin, &reviewer, &1, &9);

        let result = client.try_milestone_vote(&1u64, &0u32, &reviewer, &false, &None);
        assert_eq!(result, Err(Ok(ContractError::InvalidState.into())));
    }

    // ── Issue #1027: cancel_grant honours the emergency pause ───────────────

    #[test]
    fn test_cancel_grant_blocked_during_emergency_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _contract_id) = setup_test(&env);
        client.set_global_admin(&admin, &admin);

        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        // Unpaused: cancellation succeeds.
        let active_grant = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));
        client.cancel_grant(&active_grant, &owner, &String::from_str(&env, "cancel"));

        // Paused: cancellation is rejected by the emergency pause gate.
        let paused_grant = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));
        client.pause(&admin, &String::from_str(&env, "emergency"));

        let reason = String::from_str(&env, "cancel");
        let result = client.try_cancel_grant(&paused_grant, &owner, &reason);
        assert_eq!(result, Err(Ok(ContractError::ContractPaused.into())));
    }

    // ── Issue #890: cancel_grant must not drain escrow mid-dispute ──────────

    #[test]
    fn test_cancel_grant_blocked_while_dispute_open() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, contract_id) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let owner = Address::generate(&env);
        let funder = Address::generate(&env);
        token_admin.mint(&funder, &1000);

        let grant_id = create_client_grant(&env, &client, &owner, &token_id, Vec::new(&env));
        client.grant_fund(&grant_id, &funder, &1000);

        // Raise a dispute on milestone 0 — this locks escrow without changing
        // grant.status, which is the exact gap cancel_grant used to fall through.
        env.as_contract(&contract_id, || {
            let grant = Storage::get_grant(&env, grant_id).unwrap();
            crate::dispute::raise_dispute(
                &env,
                &grant,
                0,
                &owner,
                String::from_str(&env, "milestone proof looks fabricated"),
            )
            .unwrap();
        });

        let escrow_before = env.as_contract(&contract_id, || {
            Storage::get_escrow_account(&env, grant_id).unwrap()
        });
        assert_eq!(escrow_before.balance, 1000);
        assert!(escrow_before.locked);

        let reason = String::from_str(&env, "cancel");
        let result = client.try_cancel_grant(&grant_id, &owner, &reason);
        assert_eq!(result, Err(Ok(ContractError::EscrowLocked.into())));

        // Escrow must not have been drained, and the grant must still be Active.
        let escrow_after = env.as_contract(&contract_id, || {
            Storage::get_escrow_account(&env, grant_id).unwrap()
        });
        assert_eq!(escrow_after.balance, 1000);

        let grant_after = client.get_grant(&grant_id);
        assert_eq!(grant_after.status, GrantStatus::Active);
    }

    // ── Issue #894: batch.rs wired into the public contract interface ───────

    #[test]
    fn test_batch_vote_milestones_through_public_interface() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, contract_id) = setup_test(&env);

        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(reviewer.clone());

        create_grant(&env, &contract_id, 1, owner, token, reviewers);
        create_milestone(&env, &contract_id, 1, 0, MilestoneState::Submitted);

        let mut votes = Vec::new(&env);
        votes.push_back(BatchMilestoneVote {
            grant_id: 1,
            milestone_idx: 0,
            approve: true,
            reason: None,
        });

        let result = client.batch_vote_milestones(&reviewer, &votes);
        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);

        let milestone = client.get_milestone(&1u64, &0u32);
        assert_eq!(milestone.state, MilestoneState::Approved);
    }

    #[test]
    fn test_batch_fund_grants_through_public_interface() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _contract_id) = setup_test(&env);
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let owner = Address::generate(&env);
        let funder = Address::generate(&env);
        token_admin.mint(&funder, &500);

        let grant_id = create_client_grant(&env, &client, &owner, &token_id, Vec::new(&env));

        let mut items = Vec::new(&env);
        items.push_back((grant_id, 500i128));

        let result = client.batch_fund_grants(&funder, &token_id, &items);
        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);

        let grant = client.get_grant(&grant_id);
        assert_eq!(grant.escrow_balance, 500);
    }

    #[test]
    fn test_batch_cancel_grants_through_public_interface() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, _contract_id) = setup_test(&env);

        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        let grant_id1 = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));
        let grant_id2 = create_client_grant(&env, &client, &owner, &token, Vec::new(&env));

        let mut grant_ids = Vec::new(&env);
        grant_ids.push_back(grant_id1);
        grant_ids.push_back(grant_id2);

        let reason = String::from_str(&env, "no longer needed");
        let result = client.batch_cancel_grants(&owner, &grant_ids, &reason);
        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 0);

        assert_eq!(client.get_grant(&grant_id1).status, GrantStatus::Cancelled);
        assert_eq!(client.get_grant(&grant_id2).status, GrantStatus::Cancelled);
    }
}
