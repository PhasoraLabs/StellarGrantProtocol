use crate::storage::Storage;
use crate::types::{ContractError, GrantStatus, TransferProposal, TransferableRole};
use soroban_sdk::{Address, Env};

/// Propose a two-step ownership or reviewer-role transfer for a grant.
/// For Owner transfers, the caller must be the current owner.
/// For Reviewer transfers, the caller must be the owner OR the reviewer being replaced,
/// and `reviewer_to_replace` must currently be in the grant's reviewer list.
pub fn propose_transfer(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    new_holder: Address,
    role: TransferableRole,
    reviewer_to_replace: Option<Address>,
) -> Result<(), ContractError> {
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }

    match role {
        TransferableRole::Owner => {
            if grant.owner != *caller {
                return Err(ContractError::Unauthorized);
            }
        }
        TransferableRole::Reviewer => {
            let to_replace = reviewer_to_replace
                .as_ref()
                .ok_or(ContractError::InvalidInput)?;
            if !grant.reviewers.contains(to_replace.clone()) {
                return Err(ContractError::Unauthorized);
            }
            if grant.owner != *caller && to_replace != caller {
                return Err(ContractError::Unauthorized);
            }
        }
    }

    let proposal = TransferProposal {
        grant_id,
        current_holder: caller.clone(),
        proposed_new_holder: new_holder,
        role: role.clone(),
        reviewer_to_replace,
        proposed_at: env.ledger().timestamp(),
    };
    Storage::set_transfer_proposal(env, grant_id, &role, &proposal);
    Ok(())
}

/// Accept a pending transfer proposal. The caller must be the proposed new holder.
/// On acceptance the grant state is updated and the proposal is cleared.
pub fn accept_transfer(
    env: &Env,
    new_holder: &Address,
    grant_id: u64,
) -> Result<(), ContractError> {
    new_holder.require_auth();

    let proposal = Storage::get_transfer_proposal(env, grant_id, &TransferableRole::Owner)
        .or_else(|| Storage::get_transfer_proposal(env, grant_id, &TransferableRole::Reviewer))
        .ok_or(ContractError::InvalidState)?;

    if proposal.proposed_new_holder != *new_holder {
        return Err(ContractError::Unauthorized);
    }

    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    match proposal.role {
        TransferableRole::Owner => {
            grant.owner = new_holder.clone();
        }
        TransferableRole::Reviewer => {
            let to_replace = proposal
                .reviewer_to_replace
                .ok_or(ContractError::InvalidInput)?;
            for i in 0..grant.reviewers.len() {
                if grant.reviewers.get(i).unwrap() == to_replace {
                    grant.reviewers.set(i, new_holder.clone());
                    break;
                }
            }
        }
    }

    Storage::set_grant(env, grant_id, &grant);
    Storage::remove_transfer_proposal(env, grant_id, &proposal.role);
    Ok(())
}

pub fn get_transfer_proposal(env: &Env, grant_id: u64) -> Option<TransferProposal> {
    Storage::get_transfer_proposal(env, grant_id, &TransferableRole::Owner)
        .or_else(|| Storage::get_transfer_proposal(env, grant_id, &TransferableRole::Reviewer))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{Grant, GrantStatus};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{vec, Address, Env, String};

    fn create_test_grant(env: &Env, owner: &Address, reviewer: &Address) -> u64 {
        let grant_id = 1;
        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 100,
            reviewers: vec![env, reviewer.clone()],
            total_milestones: 10,
            milestones_paid_out: 0,
            escrow_balance: 1000,
            funders: vec![env],
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);
        grant_id
    }

    #[test]
    fn test_propose_and_accept_owner_transfer() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let grant_id = create_test_grant(&env, &owner, &reviewer);

        // Current owner proposes transfer
        let result = propose_transfer(
            &env,
            &owner,
            grant_id,
            new_owner.clone(),
            TransferableRole::Owner,
            None,
        );
        assert_eq!(result, Ok(()));

        // Check proposal exists
        let proposal = get_transfer_proposal(&env, grant_id).unwrap();
        assert_eq!(proposal.proposed_new_holder, new_owner);

        // New owner accepts
        let accept_res = accept_transfer(&env, &new_owner, grant_id);
        assert_eq!(accept_res, Ok(()));

        // Check grant is updated and proposal cleared
        let grant = Storage::get_grant(&env, grant_id).unwrap();
        assert_eq!(grant.owner, new_owner);
        assert!(get_transfer_proposal(&env, grant_id).is_none());
    }

    #[test]
    fn test_propose_transfer_unauthorized_caller() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let random_caller = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let grant_id = create_test_grant(&env, &owner, &reviewer);

        // Random caller cannot propose transfer
        let result = propose_transfer(
            &env,
            &random_caller,
            grant_id,
            new_owner.clone(),
            TransferableRole::Owner,
            None,
        );
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_accept_transfer_unauthorized_caller() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let random_caller = Address::generate(&env);
        let grant_id = create_test_grant(&env, &owner, &reviewer);

        propose_transfer(
            &env,
            &owner,
            grant_id,
            new_owner.clone(),
            TransferableRole::Owner,
            None,
        )
        .unwrap();

        // Random caller cannot accept transfer
        let accept_res = accept_transfer(&env, &random_caller, grant_id);
        assert_eq!(accept_res, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_propose_and_accept_reviewer_transfer() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let new_reviewer = Address::generate(&env);
        let grant_id = create_test_grant(&env, &owner, &reviewer);

        // Current reviewer proposes transfer to replace themselves
        let result = propose_transfer(
            &env,
            &reviewer,
            grant_id,
            new_reviewer.clone(),
            TransferableRole::Reviewer,
            Some(reviewer.clone()),
        );
        assert_eq!(result, Ok(()));

        // New reviewer accepts
        let accept_res = accept_transfer(&env, &new_reviewer, grant_id);
        assert_eq!(accept_res, Ok(()));

        // Check grant is updated
        let grant = Storage::get_grant(&env, grant_id).unwrap();
        assert_eq!(grant.reviewers.get(0).unwrap(), new_reviewer);
        assert!(get_transfer_proposal(&env, grant_id).is_none());
    }

    #[test]
    fn test_owner_and_reviewer_proposals_coexist() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let new_reviewer = Address::generate(&env);
        let grant_id = create_test_grant(&env, &owner, &reviewer);

        // Owner proposes ownership transfer
        let owner_result = propose_transfer(
            &env,
            &owner,
            grant_id,
            new_owner.clone(),
            TransferableRole::Owner,
            None,
        );
        assert_eq!(owner_result, Ok(()));

        // Reviewer proposes their replacement (should not overwrite owner's proposal)
        let reviewer_result = propose_transfer(
            &env,
            &reviewer,
            grant_id,
            new_reviewer.clone(),
            TransferableRole::Reviewer,
            Some(reviewer.clone()),
        );
        assert_eq!(reviewer_result, Ok(()));

        // Check both proposals exist independently
        let owner_proposal =
            Storage::get_transfer_proposal(&env, grant_id, &TransferableRole::Owner);
        assert!(owner_proposal.is_some());
        assert_eq!(owner_proposal.unwrap().proposed_new_holder, new_owner);

        let reviewer_proposal =
            Storage::get_transfer_proposal(&env, grant_id, &TransferableRole::Reviewer);
        assert!(reviewer_proposal.is_some());
        assert_eq!(reviewer_proposal.unwrap().proposed_new_holder, new_reviewer);

        // Accept owner transfer
        accept_transfer(&env, &new_owner, grant_id).unwrap();
        let grant = Storage::get_grant(&env, grant_id).unwrap();
        assert_eq!(grant.owner, new_owner);

        // Reviewer proposal should still exist
        let reviewer_proposal =
            Storage::get_transfer_proposal(&env, grant_id, &TransferableRole::Reviewer);
        assert!(reviewer_proposal.is_some());
    }
}
