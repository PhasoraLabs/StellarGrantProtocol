use soroban_sdk::{Address, Env, Vec};

use crate::pagination;
use crate::storage::Storage;
use crate::types::{AuditAction, AuditEntry};

/// Append a new audit entry to the grant's log.
pub fn log(
    env: &Env,
    grant_id: u64,
    action: AuditAction,
    actor: &Address,
    milestone_idx: Option<u32>,
    amount: Option<i128>,
) {
    let entry = AuditEntry {
        action,
        actor: actor.clone(),
        grant_id,
        milestone_idx,
        amount,
        timestamp: env.ledger().timestamp(),
        ledger_sequence: env.ledger().sequence(),
    };
    Storage::append_audit_entry(env, grant_id, &entry);
}

/// Return the full audit log for a grant.
pub fn get_log(env: &Env, grant_id: u64) -> Vec<AuditEntry> {
    Storage::get_audit_log(env, grant_id)
}

/// Return the last N entries from the audit log, oldest of the page first.
pub fn get_recent(env: &Env, grant_id: u64, n: u32) -> Vec<AuditEntry> {
    let log = Storage::get_audit_log(env, grant_id);
    let len = log.len();
    if n == 0 || len == 0 {
        return Vec::new(env);
    }

    let start = len.saturating_sub(n);
    pagination::paginate(env, &log, start, n)
}

/// Return the count of audit entries for a grant.
pub fn log_length(env: &Env, grant_id: u64) -> u32 {
    Storage::get_audit_log(env, grant_id).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};

    fn set_ledger(env: &Env, sequence: u32, timestamp: u64) {
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp,
            protocol_version: 21,
            sequence_number: sequence,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 100_000,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 1_000_000,
        });
    }

    fn setup() -> (Env, Address, u64) {
        let env = Env::default();
        let actor = Address::generate(&env);
        let grant_id: u64 = 42;
        (env, actor, grant_id)
    }

    // ── AuditAction variant coverage ────────────────────────────────────

    #[test]
    fn log_grant_created() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        let entries = get_log(&env, grant_id);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries.get(0).unwrap().action, AuditAction::GrantCreated);
        assert_eq!(entries.get(0).unwrap().actor, actor);
    }

    #[test]
    fn log_grant_funded_with_amount() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantFunded,
            &actor,
            None,
            Some(5_000),
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::GrantFunded);
        assert_eq!(entry.amount, Some(5_000));
    }

    #[test]
    fn log_milestone_submitted_with_index() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::MilestoneSubmitted,
            &actor,
            Some(0),
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::MilestoneSubmitted);
        assert_eq!(entry.milestone_idx, Some(0));
    }

    #[test]
    fn log_milestone_approved() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::MilestoneApproved,
            &actor,
            Some(1),
            Some(2_500),
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::MilestoneApproved);
    }

    #[test]
    fn log_milestone_rejected() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::MilestoneRejected,
            &actor,
            Some(2),
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::MilestoneRejected);
    }

    #[test]
    fn log_milestone_paid() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::MilestonePaid,
            &actor,
            Some(0),
            Some(10_000),
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::MilestonePaid);
    }

    #[test]
    fn log_dispute_raised() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::DisputeRaised,
            &actor,
            Some(1),
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::DisputeRaised);
    }

    #[test]
    fn log_dispute_resolved() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::DisputeResolved,
            &actor,
            Some(1),
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::DisputeResolved);
    }

    #[test]
    fn log_grant_cancelled() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantCancelled,
            &actor,
            None,
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::GrantCancelled);
    }

    #[test]
    fn log_grant_completed() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantCompleted,
            &actor,
            None,
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::GrantCompleted);
    }

    #[test]
    fn log_admin_changed() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::AdminChanged,
            &actor,
            None,
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::AdminChanged);
    }

    #[test]
    fn log_contract_paused() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::ContractPaused,
            &actor,
            None,
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::ContractPaused);
    }

    #[test]
    fn log_contract_unpaused() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::ContractUnpaused,
            &actor,
            None,
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.action, AuditAction::ContractUnpaused);
    }

    // ── Pagination correctness ──────────────────────────────────────────

    #[test]
    fn get_recent_empty_log_returns_empty() {
        let (env, _actor, grant_id) = setup();
        let result = get_recent(&env, grant_id, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn get_recent_zero_n_returns_empty() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        let result = get_recent(&env, grant_id, 0);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn get_recent_n_greater_than_len_returns_all() {
        let (env, actor, grant_id) = setup();
        for i in 0..3u32 {
            log(
                &env,
                grant_id,
                AuditAction::MilestoneSubmitted,
                &actor,
                Some(i),
                None,
            );
        }
        let result = get_recent(&env, grant_id, 100);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn get_recent_exact_n_matches_log_length() {
        let (env, actor, grant_id) = setup();
        for i in 0..4u32 {
            log(
                &env,
                grant_id,
                AuditAction::MilestoneApproved,
                &actor,
                Some(i),
                None,
            );
        }
        let result = get_recent(&env, grant_id, 4);
        assert_eq!(result.len(), 4);
        assert_eq!(result.get(0).unwrap().milestone_idx, Some(0));
        assert_eq!(result.get(3).unwrap().milestone_idx, Some(3));
    }

    #[test]
    fn get_recent_returns_last_n_oldest_first() {
        let (env, actor, grant_id) = setup();
        for i in 0..5u32 {
            log(
                &env,
                grant_id,
                AuditAction::MilestoneSubmitted,
                &actor,
                Some(i),
                None,
            );
        }
        let result = get_recent(&env, grant_id, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(0).unwrap().milestone_idx, Some(2));
        assert_eq!(result.get(1).unwrap().milestone_idx, Some(3));
        assert_eq!(result.get(2).unwrap().milestone_idx, Some(4));
    }

    // ── Append-only invariant ────────────────────────────────────────────

    #[test]
    fn log_is_append_only_length_grows_monotonically() {
        let (env, actor, grant_id) = setup();
        let mut expected_len: u32 = 0;

        for action in [
            AuditAction::GrantCreated,
            AuditAction::GrantFunded,
            AuditAction::MilestoneSubmitted,
            AuditAction::MilestoneApproved,
            AuditAction::MilestonePaid,
        ] {
            log(&env, grant_id, action, &actor, None, None);
            expected_len += 1;
            let entries = get_log(&env, grant_id);
            assert_eq!(entries.len(), expected_len);
        }
    }

    #[test]
    fn log_entries_are_never_overwritten() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        log(&env, grant_id, AuditAction::GrantFunded, &actor, None, None);

        let entries = get_log(&env, grant_id);
        assert_eq!(entries.get(0).unwrap().action, AuditAction::GrantCreated);
        assert_eq!(entries.get(1).unwrap().action, AuditAction::GrantFunded);
    }

    #[test]
    fn log_entries_preserve_order_across_grants() {
        let (env, actor, _) = setup();
        let g1: u64 = 1;
        let g2: u64 = 2;

        log(&env, g1, AuditAction::GrantCreated, &actor, None, None);
        log(&env, g2, AuditAction::GrantCreated, &actor, None, None);
        log(&env, g1, AuditAction::GrantFunded, &actor, None, None);

        assert_eq!(get_log(&env, g1).len(), 2);
        assert_eq!(
            get_log(&env, g1).get(0).unwrap().action,
            AuditAction::GrantCreated
        );
        assert_eq!(
            get_log(&env, g1).get(1).unwrap().action,
            AuditAction::GrantFunded
        );

        assert_eq!(get_log(&env, g2).len(), 1);
        assert_eq!(
            get_log(&env, g2).get(0).unwrap().action,
            AuditAction::GrantCreated
        );
    }

    // ── log_length helper ────────────────────────────────────────────────

    #[test]
    fn log_length_empty_is_zero() {
        let (env, _actor, grant_id) = setup();
        assert_eq!(log_length(&env, grant_id), 0);
    }

    #[test]
    fn log_length_matches_get_log() {
        let (env, actor, grant_id) = setup();
        for i in 0..7u32 {
            log(
                &env,
                grant_id,
                AuditAction::MilestoneSubmitted,
                &actor,
                Some(i),
                None,
            );
        }
        assert_eq!(log_length(&env, grant_id), 7);
        assert_eq!(get_log(&env, grant_id).len(), 7);
    }

    // ── Metadata correctness ─────────────────────────────────────────────

    #[test]
    fn entry_records_grant_id() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        assert_eq!(get_log(&env, grant_id).get(0).unwrap().grant_id, grant_id);
    }

    #[test]
    fn entry_records_timestamp_and_ledger() {
        let (env, actor, grant_id) = setup();
        set_ledger(&env, 1000, 1_700_000_000);
        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        let entry = get_log(&env, grant_id).get(0).unwrap();
        assert_eq!(entry.timestamp, 1_700_000_000);
        assert_eq!(entry.ledger_sequence, 1000);
    }

    #[test]
    fn different_actors_tracked_correctly() {
        let (env, _, grant_id) = setup();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &alice,
            None,
            None,
        );
        log(&env, grant_id, AuditAction::GrantFunded, &bob, None, None);

        let entries = get_log(&env, grant_id);
        assert_eq!(entries.get(0).unwrap().actor, alice);
        assert_eq!(entries.get(1).unwrap().actor, bob);
    }
}
