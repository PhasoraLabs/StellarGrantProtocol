use soroban_sdk::{token, Address, Env, String, Vec};

use crate::arbitration_pool;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, Dispute, DisputeStatus, Grant};

pub fn raise_dispute(
    env: &Env,
    grant: &Grant,
    milestone_idx: u32,
    caller: &Address,
    reason: String,
) -> Result<Dispute, ContractError> {
    if grant.status != crate::types::GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    let is_owner = grant.owner == *caller;
    let is_reviewer = grant.reviewers.contains(caller.clone());
    if !(is_owner || is_reviewer) {
        return Err(ContractError::Unauthorized);
    }

    if Storage::get_dispute(env, grant.id, milestone_idx).is_some() {
        return Err(ContractError::InvalidState);
    }

    crate::escrow::lock(env, grant.id)?;

    let dispute = Dispute {
        grant_id: grant.id,
        milestone_idx,
        raised_by: caller.clone(),
        reason,
        status: DisputeStatus::Open,
        arbiters: Vec::new(env),
        votes_contributor: 0,
        votes_funder: 0,
        raised_at: env.ledger().timestamp(),
        resolved_at: None,
    };

    Storage::set_dispute(env, grant.id, milestone_idx, &dispute);
    Events::emit_dispute_raised(env, grant.id, milestone_idx, caller.clone());
    Ok(dispute)
}

pub fn assign_arbiter(
    env: &Env,
    dispute: &mut Dispute,
    admin: &Address,
    arbiter: &Address,
) -> Result<(), ContractError> {
    if dispute.status != DisputeStatus::Open {
        return Err(ContractError::InvalidState);
    }
    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    if dispute.arbiters.contains(arbiter.clone()) {
        return Err(ContractError::AlreadyVoted);
    }
    dispute.arbiters.push_back(arbiter.clone());
    dispute.status = DisputeStatus::UnderReview;
    Storage::set_dispute(env, dispute.grant_id, dispute.milestone_idx, dispute);
    Events::emit_arbiter_assigned(
        env,
        dispute.grant_id,
        dispute.milestone_idx,
        arbiter.clone(),
    );
    Ok(())
}

/// Assign a randomized community-pool panel to an open dispute (Issue #573).
///
/// Used instead of the manual `assign_arbiter` flow when community/pool
/// arbitration is enabled. Delegates panel selection to the arbitration pool and
/// moves the dispute into `UnderReview`. Returns the created arbitration case id.
pub fn assign_pool_panel(
    env: &Env,
    dispute: &mut Dispute,
    dispute_id: u32,
    panel_size: u32,
) -> Result<u32, ContractError> {
    if dispute.status != DisputeStatus::Open {
        return Err(ContractError::InvalidState);
    }
    let case_id = arbitration_pool::assign_panel(env, dispute_id, panel_size)?;
    dispute.status = DisputeStatus::UnderReview;
    Storage::set_dispute(env, dispute.grant_id, dispute.milestone_idx, dispute);
    Ok(case_id)
}

pub fn arbiter_vote(
    env: &Env,
    dispute: &mut Dispute,
    arbiter: &Address,
    favor_contributor: bool,
) -> Result<(), ContractError> {
    if dispute.status != DisputeStatus::UnderReview {
        return Err(ContractError::InvalidState);
    }
    if !dispute.arbiters.contains(arbiter.clone()) {
        return Err(ContractError::Unauthorized);
    }
    if favor_contributor {
        dispute.votes_contributor = dispute.votes_contributor.saturating_add(1);
    } else {
        dispute.votes_funder = dispute.votes_funder.saturating_add(1);
    }
    Storage::set_dispute(env, dispute.grant_id, dispute.milestone_idx, dispute);
    Events::emit_arbiter_voted(
        env,
        dispute.grant_id,
        dispute.milestone_idx,
        arbiter.clone(),
        favor_contributor,
    );
    Ok(())
}

pub fn resolve_dispute(
    env: &Env,
    grant: &mut Grant,
    dispute: &mut Dispute,
) -> Result<DisputeStatus, ContractError> {
    if dispute.status != DisputeStatus::UnderReview {
        return Err(ContractError::InvalidState);
    }

    let total_votes = dispute
        .votes_contributor
        .saturating_add(dispute.votes_funder);
    if total_votes == 0 {
        return Err(ContractError::InvalidState);
    }

    let majority = total_votes / 2 + 1;
    let outcome = if dispute.votes_contributor >= majority {
        DisputeStatus::ResolvedForContributor
    } else if dispute.votes_funder >= majority {
        DisputeStatus::ResolvedForFunder
    } else {
        return Err(ContractError::QuorumNotReached);
    };

    let grant_id = dispute.grant_id;
    let milestone_idx = dispute.milestone_idx;

    crate::escrow::unlock(env, grant_id)?;

    if outcome == DisputeStatus::ResolvedForContributor {
        if let Some(milestone) = Storage::get_milestone(env, grant_id, milestone_idx) {
            crate::escrow::release(env, grant_id, &grant.owner, milestone.amount)?;
        }
    } else if let Some(milestone) = Storage::get_milestone(env, grant_id, milestone_idx) {
        if !grant.funders.is_empty() {
            crate::escrow::release_to_funders(env, grant_id, &grant.funders, milestone.amount)?;
        }
    }

    dispute.status = outcome.clone();
    dispute.resolved_at = Some(env.ledger().timestamp());
    Storage::set_dispute(env, grant_id, milestone_idx, dispute);

    // Reload grant to get updated escrow_balance from escrow operations
    if let Some(updated_grant) = Storage::get_grant(env, grant_id) {
        *grant = updated_grant;
    }

    let for_contributor = outcome == DisputeStatus::ResolvedForContributor;
    Events::emit_dispute_resolved(env, grant_id, milestone_idx, for_contributor);
    Ok(outcome)
}

pub fn cancel_dispute(
    env: &Env,
    dispute: &mut Dispute,
    caller: &Address,
) -> Result<(), ContractError> {
    if dispute.status != DisputeStatus::Open && dispute.status != DisputeStatus::UnderReview {
        return Err(ContractError::InvalidState);
    }
    if dispute.raised_by != *caller && Storage::get_global_admin(env) != Some(caller.clone()) {
        return Err(ContractError::Unauthorized);
    }
    dispute.status = DisputeStatus::Cancelled;
    dispute.resolved_at = Some(env.ledger().timestamp());
    Storage::set_dispute(env, dispute.grant_id, dispute.milestone_idx, dispute);
    Events::emit_dispute_cancelled(env, dispute.grant_id, dispute.milestone_idx, caller.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantFund, GrantStatus, Milestone, MilestoneState};
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token, Env, String, Vec};

    fn make_grant(env: &Env, owner: Address) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: String::from_str(env, "T"),
            description: String::from_str(env, "D"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 2,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        }
    }

    #[test]
    fn test_raise_dispute_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        let grant = make_grant(&env, owner);
        let reason = String::from_str(&env, "Proof is invalid");
        let result = raise_dispute(&env, &grant, 0, &stranger, reason);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_arbiter_quorum_not_reached_returns_error() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let grant = make_grant(&env, owner.clone());

        let mut dispute = Dispute {
            grant_id: 1,
            milestone_idx: 0,
            raised_by: owner.clone(),
            reason: String::from_str(&env, "reason"),
            status: DisputeStatus::UnderReview,
            arbiters: Vec::new(&env),
            votes_contributor: 1,
            votes_funder: 1,
            raised_at: 0,
            resolved_at: None,
        };

        let mut grant_mut = grant.clone();
        let result = resolve_dispute(&env, &mut grant_mut, &mut dispute);
        assert_eq!(result, Err(ContractError::QuorumNotReached));
    }

    #[test]
    fn test_resolve_dispute_wrong_status_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let grant = make_grant(&env, owner.clone());

        let mut dispute = Dispute {
            grant_id: 1,
            milestone_idx: 0,
            raised_by: owner.clone(),
            reason: String::from_str(&env, "reason"),
            status: DisputeStatus::Open,
            arbiters: Vec::new(&env),
            votes_contributor: 3,
            votes_funder: 0,
            raised_at: 0,
            resolved_at: None,
        };

        let mut grant_mut = grant.clone();
        let result = resolve_dispute(&env, &mut grant_mut, &mut dispute);
        assert_eq!(result, Err(ContractError::InvalidState));
    }

    #[test]
    fn test_arbiter_vote_on_wrong_status_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let arbiter = Address::generate(&env);
        let mut dispute = Dispute {
            grant_id: 1,
            milestone_idx: 0,
            raised_by: owner.clone(),
            reason: String::from_str(&env, "reason"),
            status: DisputeStatus::Open,
            arbiters: {
                let mut v = Vec::new(&env);
                v.push_back(arbiter.clone());
                v
            },
            votes_contributor: 0,
            votes_funder: 0,
            raised_at: 0,
            resolved_at: None,
        };
        let result = arbiter_vote(&env, &mut dispute, &arbiter, true);
        assert_eq!(result, Err(ContractError::InvalidState));
    }

    #[test]
    fn test_arbiter_vote_by_non_arbiter_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        let mut dispute = Dispute {
            grant_id: 1,
            milestone_idx: 0,
            raised_by: owner.clone(),
            reason: String::from_str(&env, "reason"),
            status: DisputeStatus::UnderReview,
            arbiters: Vec::new(&env),
            votes_contributor: 0,
            votes_funder: 0,
            raised_at: 0,
            resolved_at: None,
        };
        let result = arbiter_vote(&env, &mut dispute, &stranger, true);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    fn make_milestone(env: &Env, idx: u32, amount: i128) -> Milestone {
        Milestone {
            idx,
            description: String::from_str(env, "M"),
            amount,
            state: MilestoneState::Approved,
            votes: soroban_sdk::Map::new(env),
            approvals: 0,
            rejections: 0,
            reasons: soroban_sdk::Map::new(env),
            status_updated_at: 0,
            proof_url: None,
            submission_timestamp: 0,
            deadline: None,
            reviewer_count_snapshot: 0,
        }
    }

    /// Sets up a grant with a real `EscrowAccount` funded via the normal
    /// deposit flow (mirrors `grant.funders`), plus two milestone records.
    /// Returns the grant loaded back from storage (so `funders`/`escrow_balance`
    /// reflect the deposit).
    ///
    /// The deposit goes through the contract's `grant_fund` entrypoint
    /// (rather than calling `escrow::deposit` directly) because the token
    /// transfer needs `funder`'s auth tied to a root contract invocation.
    fn setup_funded_grant(
        env: &Env,
        client: &crate::StellarGrantsContractClient,
        contract_id: &Address,
        owner: &Address,
        funder: &Address,
        token_id: &Address,
        deposit_amount: i128,
    ) -> Grant {
        let mut grant = make_grant(env, owner.clone());
        grant.token = token_id.clone();
        grant.total_milestones = 2;

        env.as_contract(contract_id, || {
            Storage::set_grant(env, grant.id, &grant);
            crate::escrow::open(env, grant.id, owner, token_id).unwrap();
            Storage::set_milestone(env, grant.id, 0, &make_milestone(env, 0, 400));
            Storage::set_milestone(env, grant.id, 1, &make_milestone(env, 1, 300));
        });

        client.grant_fund(&grant.id, funder, &deposit_amount);

        env.as_contract(contract_id, || Storage::get_grant(env, grant.id).unwrap())
    }

    #[test]
    fn test_resolve_dispute_for_contributor_unlocks_escrow_and_pays_milestone() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let funder = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = asset.address();
        token::StellarAssetClient::new(&env, &token_id).mint(&funder, &10_000);
        let token_client = token::Client::new(&env, &token_id);

        let grant = setup_funded_grant(
            &env,
            &client,
            &contract_id,
            &owner,
            &funder,
            &token_id,
            1_000,
        );

        let mut dispute = env
            .as_contract(&contract_id, || {
                raise_dispute(&env, &grant, 0, &owner, String::from_str(&env, "bad proof"))
            })
            .unwrap();

        env.as_contract(&contract_id, || {
            assert!(crate::escrow::get_account(&env, grant.id).unwrap().locked);
        });

        dispute.status = DisputeStatus::UnderReview;
        dispute.votes_contributor = 1;
        dispute.votes_funder = 0;

        let mut grant_mut = grant.clone();
        let outcome = env
            .as_contract(&contract_id, || {
                resolve_dispute(&env, &mut grant_mut, &mut dispute)
            })
            .unwrap();

        assert_eq!(outcome, DisputeStatus::ResolvedForContributor);
        assert_eq!(dispute.status, DisputeStatus::ResolvedForContributor);
        assert_eq!(token_client.balance(&owner), 400);

        env.as_contract(&contract_id, || {
            let account = crate::escrow::get_account(&env, grant.id).unwrap();
            assert!(!account.locked);
            assert_eq!(account.balance, 600);

            // Escrow is usable for the next milestone's payout.
            crate::escrow::release(&env, grant.id, &owner, 300).unwrap();
        });
        assert_eq!(token_client.balance(&owner), 700);
    }

    #[test]
    fn test_resolve_dispute_for_funder_unlocks_escrow_and_releases_funds() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let client = crate::StellarGrantsContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let funder = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = asset.address();
        token::StellarAssetClient::new(&env, &token_id).mint(&funder, &10_000);
        let token_client = token::Client::new(&env, &token_id);

        let grant = setup_funded_grant(
            &env,
            &client,
            &contract_id,
            &owner,
            &funder,
            &token_id,
            1_000,
        );
        // The normal deposit flow mirrors the funder onto `grant.funders`.
        assert_eq!(grant.funders.len(), 1);

        let mut dispute = env
            .as_contract(&contract_id, || {
                raise_dispute(&env, &grant, 0, &owner, String::from_str(&env, "bad proof"))
            })
            .unwrap();

        dispute.status = DisputeStatus::UnderReview;
        dispute.votes_contributor = 0;
        dispute.votes_funder = 1;

        let mut grant_mut = grant.clone();
        let outcome = env
            .as_contract(&contract_id, || {
                resolve_dispute(&env, &mut grant_mut, &mut dispute)
            })
            .unwrap();

        assert_eq!(outcome, DisputeStatus::ResolvedForFunder);
        assert_eq!(token_client.balance(&funder), 10_000 - 1_000 + 400);

        env.as_contract(&contract_id, || {
            let account = crate::escrow::get_account(&env, grant.id).unwrap();
            assert!(!account.locked);
            assert_eq!(account.balance, 600);

            // Escrow is usable for the next milestone's payout.
            crate::escrow::release(&env, grant.id, &owner, 300).unwrap();
        });
        assert_eq!(token_client.balance(&owner), 300);
    }
}
