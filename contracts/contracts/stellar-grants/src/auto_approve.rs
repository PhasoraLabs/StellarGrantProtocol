use soroban_sdk::{Address, Env, Map, Vec};

use crate::errors::ContractError;
use crate::governance;
use crate::governance::VoteResult;
use crate::storage::Storage;
use crate::types::{AutoApproveConfig, AutoApproveRecord, MilestoneState};

/// Configure auto-approve for a grant. Owner only.
pub fn set_config(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    config: AutoApproveConfig,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    if config.grace_period_seconds == 0 && config.min_votes_required == 0 {
        return Err(ContractError::InvalidInput);
    }

    Storage::set_auto_approve_config(env, grant_id, &config);
    Ok(())
}

/// Attempt auto-approve for a milestone. Anyone may call; enforces all conditions.
pub fn try_auto_approve(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<bool, ContractError> {
    let config = Storage::get_auto_approve_config(env, grant_id)
        .ok_or(ContractError::AutoApproveNotEnabled)?;

    if !config.enabled {
        return Err(ContractError::AutoApproveNotEnabled);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;

    if milestone.state != MilestoneState::Submitted {
        return Ok(false);
    }

    if let Some(existing) = Storage::get_auto_approve_record(env, grant_id, milestone_idx) {
        let _ = existing;
        return Ok(false);
    }

    let now = env.ledger().timestamp();
    let submission_time = milestone.submission_timestamp;
    let deadline = milestone.deadline.unwrap_or(0);

    let effective_deadline = if deadline > 0 {
        deadline
    } else {
        submission_time
    };
    let grace_end = effective_deadline.saturating_add(config.grace_period_seconds);

    if now < grace_end {
        return Err(ContractError::AutoApproveGracePeriodNotPassed);
    }

    let votes_cast = milestone.approvals + milestone.rejections;
    if votes_cast < config.min_votes_required {
        return Err(ContractError::AutoApproveInsufficientVotes);
    }

    let mut grant = Storage::get_grant_v(env, grant_id);
    let mut milestone = Storage::get_milestone_v(env, grant_id, milestone_idx);

    let approved = milestone.approvals > 0 && milestone.approvals > milestone.rejections;
    let vote_result = VoteResult {
        approved,
        quorum_reached: true,
        approval_pct: 100,
    };

    governance::finalize_milestone(&mut milestone, &vote_result);
    Storage::set_milestone(env, grant_id, milestone_idx, &milestone);

    let record = AutoApproveRecord {
        grant_id,
        milestone_idx,
        triggered_by: caller.clone(),
        triggered_at: now,
        votes_at_trigger: votes_cast,
    };
    Storage::set_auto_approve_record(env, grant_id, milestone_idx, &record);

    crate::events::Events::milestone_status_changed(
        env,
        grant_id,
        milestone_idx,
        MilestoneState::Approved,
    );

    // Issue #1024: an auto-approved milestone is a legitimate approval and must
    // trigger the same notification, reputation, audit, metrics, hook, NFT,
    // portfolio, and badge side effects as one approved via `milestone_vote`.
    crate::apply_milestone_approval_side_effects(env, &grant, &milestone, caller);

    Ok(true)
}

/// Return whether auto-approve conditions are currently met for a milestone.
pub fn can_auto_approve(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    let config = match Storage::get_auto_approve_config(env, grant_id) {
        Some(c) if c.enabled => c,
        _ => return false,
    };

    let grant = match Storage::get_grant(env, grant_id) {
        Some(g) => g,
        None => return false,
    };

    if milestone_idx >= grant.total_milestones {
        return false;
    }

    let milestone = match Storage::get_milestone(env, grant_id, milestone_idx) {
        Some(m) => m,
        None => return false,
    };

    if milestone.state != MilestoneState::Submitted {
        return false;
    }

    if Storage::get_auto_approve_record(env, grant_id, milestone_idx).is_some() {
        return false;
    }

    let now = env.ledger().timestamp();
    let submission_time = milestone.submission_timestamp;
    let deadline = milestone.deadline.unwrap_or(0);
    let effective_deadline = if deadline > 0 {
        deadline
    } else {
        submission_time
    };
    let grace_end = effective_deadline.saturating_add(config.grace_period_seconds);

    if now < grace_end {
        return false;
    }

    let votes_cast = milestone.approvals + milestone.rejections;
    votes_cast >= config.min_votes_required
}

/// Return the auto-approve config for a grant.
pub fn get_config(env: &Env, grant_id: u64) -> Option<AutoApproveConfig> {
    Storage::get_auto_approve_config(env, grant_id)
}

/// Return the auto-approve record if it was triggered.
pub fn get_record(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<AutoApproveRecord> {
    Storage::get_auto_approve_record(env, grant_id, milestone_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{AutoApproveConfig, Grant, GrantStatus, Milestone, MilestoneState};
    use soroban_sdk::testutils::{Address as _, Ledger as _};

    fn setup() -> (Env, Address, u64) {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let grant_id = 1u64;

        env.as_contract(&contract_id, || {
            let grant = Grant {
                id: grant_id,
                owner: owner.clone(),
                title: soroban_sdk::String::from_str(&env, "Test Grant"),
                description: soroban_sdk::String::from_str(&env, "Desc"),
                total_amount: 1_000_000,
                status: GrantStatus::Active,
                total_amount: 1_000_000,
                milestone_amount: 500_000,
                reviewers: soroban_sdk::Vec::new(&env),
                total_milestones: 2,
                milestone_amount: 500_000,
                reviewers: Vec::new(&env),
                milestones_paid_out: 0,
                escrow_balance: 0,
                funders: Vec::new(&env),
                reason: None,
                timestamp: env.ledger().timestamp(),
                require_compliance: None,
                token: Address::generate(&env),
            };
            Storage::set_grant(&env, grant_id, &grant);

            let milestone = Milestone {
                idx: 0,
                description: soroban_sdk::String::from_str(&env, "M1"),
                amount: 500_000,
                state: MilestoneState::Submitted,
                votes: Map::new(&env),
                approvals: 0,
                rejections: 0,
                reasons: Map::new(&env),
                status_updated_at: env.ledger().timestamp(),
                proof_url: None,
                submission_timestamp: 1000,
                deadline: None,
                reviewer_count_snapshot: 0,
            };
            Storage::set_milestone(&env, grant_id, 0, &milestone);
        });

        (env, owner, grant_id)
    }

    #[test]
    fn test_set_and_get_config() {
        let (env, owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());

        let config = AutoApproveConfig {
            grant_id,
            enabled: true,
            grace_period_seconds: 3600,
            min_votes_required: 3,
            set_by: owner.clone(),
            set_at: env.ledger().timestamp(),
        };

        env.as_contract(&contract_id, || {
            let result = set_config(&env, &owner, grant_id, config.clone());
            assert_eq!(result, Ok(()));

            let stored = get_config(&env, grant_id).unwrap();
            assert_eq!(stored.enabled, true);
            assert_eq!(stored.grace_period_seconds, 3600);
            assert_eq!(stored.min_votes_required, 3);
        });
    }

    #[test]
    fn test_set_config_rejects_zero_params() {
        let (env, owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());

        let config = AutoApproveConfig {
            grant_id,
            enabled: true,
            grace_period_seconds: 0,
            min_votes_required: 0,
            set_by: owner.clone(),
            set_at: env.ledger().timestamp(),
        };

        env.as_contract(&contract_id, || {
            let result = set_config(&env, &owner, grant_id, config);
            assert_eq!(result, Err(ContractError::InvalidInput));
        });
    }

    #[test]
    fn test_try_auto_approve_not_enabled() {
        let (env, _owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let caller = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let result = try_auto_approve(&env, &caller, grant_id, 0);
            assert_eq!(result, Err(ContractError::AutoApproveNotEnabled));
        });
    }

    #[test]
    fn test_try_auto_approve_grace_period_not_passed() {
        let (env, owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let caller = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let config = AutoApproveConfig {
                grant_id,
                enabled: true,
                grace_period_seconds: 3600,
                min_votes_required: 1,
                set_by: owner.clone(),
                set_at: env.ledger().timestamp(),
            };
            set_config(&env, &owner, grant_id, config).unwrap();

            // Set enough votes but don't advance time past grace period
            let mut milestone = Storage::get_milestone_v(&env, grant_id, 0);
            milestone.approvals = 2;
            milestone.rejections = 0;
            Storage::set_milestone(&env, grant_id, 0, &milestone);

            let result = try_auto_approve(&env, &caller, grant_id, 0);
            assert_eq!(result, Err(ContractError::AutoApproveGracePeriodNotPassed));
        });
    }

    #[test]
    fn test_try_auto_approve_insufficient_votes() {
        let (env, owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let caller = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let config = AutoApproveConfig {
                grant_id,
                enabled: true,
                grace_period_seconds: 0,
                min_votes_required: 5,
                set_by: owner.clone(),
                set_at: env.ledger().timestamp(),
            };
            set_config(&env, &owner, grant_id, config).unwrap();

            // Only 2 votes cast, need 5
            let mut milestone = Storage::get_milestone_v(&env, grant_id, 0);
            milestone.approvals = 1;
            milestone.rejections = 1;
            Storage::set_milestone(&env, grant_id, 0, &milestone);

            // Advance time past grace period
            let mut ledger = env.ledger().get();
            ledger.timestamp = 5000;
            env.ledger().set(ledger);

            let result = try_auto_approve(&env, &caller, grant_id, 0);
            assert_eq!(result, Err(ContractError::AutoApproveInsufficientVotes));
        });
    }

    #[test]
    fn test_try_auto_approve_success() {
        let (env, owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let caller = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let config = AutoApproveConfig {
                grant_id,
                enabled: true,
                grace_period_seconds: 0,
                min_votes_required: 2,
                set_by: owner.clone(),
                set_at: env.ledger().timestamp(),
            };
            set_config(&env, &owner, grant_id, config).unwrap();

            // 3 approvals, 1 rejection — majority approves
            let mut milestone = Storage::get_milestone_v(&env, grant_id, 0);
            milestone.approvals = 3;
            milestone.rejections = 1;
            milestone.submission_timestamp = 1000;
            Storage::set_milestone(&env, grant_id, 0, &milestone);

            let result = try_auto_approve(&env, &caller, grant_id, 0);
            assert_eq!(result, Ok(true));

            // Record should exist
            let record = get_record(&env, grant_id, 0).unwrap();
            assert_eq!(record.triggered_by, caller);
            assert_eq!(record.votes_at_trigger, 4);
        });
    }

    #[test]
    fn test_try_auto_approve_rejects_already_triggered() {
        let (env, owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let caller = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let config = AutoApproveConfig {
                grant_id,
                enabled: true,
                grace_period_seconds: 0,
                min_votes_required: 1,
                set_by: owner.clone(),
                set_at: env.ledger().timestamp(),
            };
            set_config(&env, &owner, grant_id, config).unwrap();

            let mut milestone = Storage::get_milestone_v(&env, grant_id, 0);
            milestone.approvals = 2;
            milestone.rejections = 0;
            Storage::set_milestone(&env, grant_id, 0, &milestone);

            // First call succeeds
            let result = try_auto_approve(&env, &caller, grant_id, 0);
            assert_eq!(result, Ok(true));

            // Second call returns false (already triggered)
            let result2 = try_auto_approve(&env, &caller, grant_id, 0);
            assert_eq!(result2, Ok(false));
        });
    }

    #[test]
    fn test_can_auto_approve() {
        let (env, owner, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            assert!(!can_auto_approve(&env, grant_id, 0));

            let config = AutoApproveConfig {
                grant_id,
                enabled: true,
                grace_period_seconds: 0,
                min_votes_required: 1,
                set_by: owner.clone(),
                set_at: env.ledger().timestamp(),
            };
            set_config(&env, &owner, grant_id, config).unwrap();

            let mut milestone = Storage::get_milestone_v(&env, grant_id, 0);
            milestone.approvals = 1;
            Storage::set_milestone(&env, grant_id, 0, &milestone);

            assert!(can_auto_approve(&env, grant_id, 0));
        });
    }
}
