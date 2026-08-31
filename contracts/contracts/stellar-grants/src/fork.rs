use soroban_sdk::{Address, Env, Map, String, Vec};

use crate::constants;
use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ForkRecord, GrantStatus, Milestone, MilestoneState};

pub fn fork_grant(
    env: &Env,
    caller: &Address,
    original_grant_id: u64,
    new_title: String,
    new_description: String,
    new_total_amount: i128,
    new_token: &Address,
    inherit_reviewers: bool,
    inherit_milestones: bool,
) -> Result<u64, ContractError> {
    let original =
        Storage::get_grant(env, original_grant_id).ok_or(ContractError::GrantNotFound)?;

    if original.status == GrantStatus::Cancelled {
        return Err(ContractError::InvalidState);
    }

    let depth = fork_depth(env, original_grant_id);
    if depth >= constants::MAX_FORK_DEPTH {
        return Err(ContractError::InvalidInput);
    }

    let reviewers = if inherit_reviewers {
        original.reviewers.clone()
    } else {
        Vec::new(env)
    };

    let new_grant_id = crate::internal_grant_create(
        env,
        caller,
        new_title,
        new_description,
        new_token,
        new_total_amount,
        original.milestone_amount,
        original.total_milestones,
        reviewers,
    )?;

    let mut inherited_fields = Vec::new(env);
    let mut overridden_fields = Vec::new(env);
    if inherit_reviewers {
        inherited_fields.push_back(String::from_str(env, "reviewers"));
    }
    if inherit_milestones {
        let mut copied_any = false;
        for idx in 0..original.total_milestones {
            if let Some(orig_milestone) = Storage::get_milestone(env, original_grant_id, idx) {
                let new_milestone = Milestone {
                    idx,
                    description: orig_milestone.description.clone(),
                    amount: original.milestone_amount,
                    state: MilestoneState::Pending,
                    votes: Map::new(env),
                    approvals: 0,
                    rejections: 0,
                    reasons: Map::new(env),
                    status_updated_at: 0,
                    proof_url: None,
                    submission_timestamp: 0,
                    deadline: None,
                    reviewer_count_snapshot: 0,
                };
                Storage::set_milestone(env, new_grant_id, idx, &new_milestone);
                copied_any = true;
            }
        }
        if copied_any {
            inherited_fields.push_back(String::from_str(env, "milestones"));
        }
    }
    overridden_fields.push_back(String::from_str(env, "title"));
    overridden_fields.push_back(String::from_str(env, "description"));
    overridden_fields.push_back(String::from_str(env, "total_amount"));
    overridden_fields.push_back(String::from_str(env, "token"));

    let record = ForkRecord {
        original_grant_id,
        forked_grant_id: new_grant_id,
        forked_by: caller.clone(),
        forked_at: env.ledger().timestamp(),
        inherited_fields,
        overridden_fields,
    };

    Storage::set_fork_record(env, new_grant_id, &record);

    let mut children: Vec<u64> = Storage::get_fork_children(env, original_grant_id);
    if !children.contains(new_grant_id) {
        children.push_back(new_grant_id);
        Storage::set_fork_children(env, original_grant_id, &children);
    }

    Events::emit_grant_forked(env, original_grant_id, new_grant_id);

    Ok(new_grant_id)
}

pub fn get_fork_record(env: &Env, grant_id: u64) -> Option<ForkRecord> {
    Storage::get_fork_record(env, grant_id)
}

pub fn get_forks(env: &Env, original_grant_id: u64) -> Vec<u64> {
    Storage::get_fork_children(env, original_grant_id)
}

pub fn fork_depth(env: &Env, grant_id: u64) -> u32 {
    let mut depth = 0u32;
    let mut current = grant_id;
    loop {
        if let Some(record) = Storage::get_fork_record(env, current) {
            current = record.original_grant_id;
            depth += 1;
        } else {
            break;
        }
    }
    depth
}

pub fn is_descendant(env: &Env, ancestor_id: u64, descendant_id: u64) -> bool {
    let mut current = descendant_id;
    loop {
        if let Some(record) = Storage::get_fork_record(env, current) {
            if record.original_grant_id == ancestor_id {
                return true;
            }
            current = record.original_grant_id;
        } else {
            return false;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::constants::MAX_FORK_DEPTH;
    use crate::storage::Storage;
    use crate::types::{Grant, GrantStatus};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{vec, Address, Env, String};

    fn setup_grant(env: &Env, id: u64, owner: &Address) {
        let grant = Grant {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "Original Grant"),
            description: String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 100,
            reviewers: vec![env],
            total_milestones: 10,
            milestones_paid_out: 0,
            escrow_balance: 1000,
            funders: vec![env],
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, id, &grant);
    }

    fn setup_milestone(env: &Env, grant_id: u64, idx: u32, description: &str) {
        let milestone = Milestone {
            idx,
            description: String::from_str(env, description),
            amount: 100,
            state: MilestoneState::Approved,
            votes: Map::new(env),
            approvals: 1,
            rejections: 0,
            reasons: Map::new(env),
            status_updated_at: 0,
            proof_url: Some(String::from_str(env, "https://proof")),
            submission_timestamp: env.ledger().timestamp(),
            deadline: None,
            reviewer_count_snapshot: 1,
        };
        Storage::set_milestone(env, grant_id, idx, &milestone);
    }

    /// #875: inherit_milestones = true must genuinely copy each submitted
    /// milestone's description onto the new grant (with approval state
    /// reset), and only then may ForkRecord.inherited_fields claim
    /// "milestones" was inherited.
    #[test]
    fn test_fork_grant_copies_milestone_data_when_inherited() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let caller = Address::generate(&env);
        let token = Address::generate(&env);
        setup_grant(&env, 1, &owner);
        setup_milestone(&env, 1, 0, "Design doc");
        setup_milestone(&env, 1, 1, "MVP implementation");

        let new_grant_id = fork_grant(
            &env,
            &caller,
            1,
            String::from_str(&env, "Forked Grant"),
            String::from_str(&env, "Forked Desc"),
            2000,
            &token,
            false,
            true,
        )
        .unwrap();

        let record = get_fork_record(&env, new_grant_id).unwrap();
        assert!(record
            .inherited_fields
            .contains(String::from_str(&env, "milestones")));

        let copied_0 = Storage::get_milestone(&env, new_grant_id, 0).unwrap();
        assert_eq!(copied_0.description, String::from_str(&env, "Design doc"));
        assert_eq!(copied_0.state, MilestoneState::Pending);
        assert_eq!(copied_0.approvals, 0);
        assert!(copied_0.proof_url.is_none());

        let copied_1 = Storage::get_milestone(&env, new_grant_id, 1).unwrap();
        assert_eq!(
            copied_1.description,
            String::from_str(&env, "MVP implementation")
        );
        assert_eq!(copied_1.state, MilestoneState::Pending);
    }

    #[test]
    fn test_fork_grant_no_milestones_copied_does_not_claim_inherited() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let caller = Address::generate(&env);
        let token = Address::generate(&env);
        // No milestones ever submitted on the original grant.
        setup_grant(&env, 1, &owner);

        let new_grant_id = fork_grant(
            &env,
            &caller,
            1,
            String::from_str(&env, "Forked Grant"),
            String::from_str(&env, "Forked Desc"),
            2000,
            &token,
            false,
            true,
        )
        .unwrap();

        let record = get_fork_record(&env, new_grant_id).unwrap();
        assert!(!record
            .inherited_fields
            .contains(String::from_str(&env, "milestones")));
    }

    #[test]
    fn test_fork_grant() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let caller = Address::generate(&env);
        let token = Address::generate(&env);
        setup_grant(&env, 1, &owner);

        let new_grant_id = fork_grant(
            &env,
            &caller,
            1,
            String::from_str(&env, "Forked Grant"),
            String::from_str(&env, "Forked Desc"),
            2000,
            &token,
            true,
            true,
        )
        .unwrap();

        // Verify fork record
        let record = get_fork_record(&env, new_grant_id).unwrap();
        assert_eq!(record.original_grant_id, 1);
        assert_eq!(record.forked_by, caller);

        // Verify children
        let children = get_forks(&env, 1);
        assert_eq!(children.len(), 1);
        assert_eq!(children.get(0).unwrap(), new_grant_id);

        // Verify depth and descendant
        assert_eq!(fork_depth(&env, new_grant_id), 1);
        assert!(is_descendant(&env, 1, new_grant_id));
    }

    #[test]
    fn test_fork_depth_limit() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        setup_grant(&env, 1, &owner);
        let mut current_id = 1;

        // Reach max depth
        for _ in 0..MAX_FORK_DEPTH {
            let caller = Address::generate(&env);
            current_id = fork_grant(
                &env,
                &caller,
                current_id,
                String::from_str(&env, "Fork"),
                String::from_str(&env, "Desc"),
                100,
                &token,
                false,
                false,
            )
            .unwrap();
        }

        // Exceed max depth
        let caller = Address::generate(&env);
        let result = fork_grant(
            &env,
            &caller,
            current_id,
            String::from_str(&env, "Fork"),
            String::from_str(&env, "Desc"),
            100,
            &token,
            false,
            false,
        );
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_fork_multiple_children() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        setup_grant(&env, 1, &owner);

        let caller1 = Address::generate(&env);
        let child1 = fork_grant(
            &env,
            &caller1,
            1,
            String::from_str(&env, "Fork 1"),
            String::from_str(&env, "Desc"),
            100,
            &token,
            false,
            false,
        )
        .unwrap();

        let caller2 = Address::generate(&env);
        let child2 = fork_grant(
            &env,
            &caller2,
            1,
            String::from_str(&env, "Fork 2"),
            String::from_str(&env, "Desc"),
            100,
            &token,
            false,
            false,
        )
        .unwrap();

        let children = get_forks(&env, 1);
        assert_eq!(children.len(), 2);
        assert_eq!(children.get(0).unwrap(), child1);
        assert_eq!(children.get(1).unwrap(), child2);
    }
}
