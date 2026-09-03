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

pub fn get_audit_log(env: &Env, grant_id: u64) -> Vec<AuditEntry> {
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
        let e = entries.get(0).unwrap();
        assert_eq!(e.grant_id, grant_id);
        assert_eq!(e.action, AuditAction::GrantCreated);
        assert_eq!(e.actor, actor);
        assert_eq!(e.milestone_idx, None);
        assert_eq!(e.amount, None);
    }

    #[test]
    fn log_grant_funded() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantFunded,
            &actor,
            None,
            Some(5_000),
        );

        let entries = get_log(&env, grant_id);
        assert_eq!(entries.len(), 1);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::GrantFunded);
        assert_eq!(e.amount, Some(5_000));
    }

    #[test]
    fn log_milestone_submitted() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::MilestoneSubmitted,
            &actor,
            Some(2),
            None,
        );

        let entries = get_log(&env, grant_id);
        assert_eq!(entries.len(), 1);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::MilestoneSubmitted);
        assert_eq!(e.milestone_idx, Some(2));
    }

    #[test]
    fn log_milestone_approved() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::MilestoneApproved,
            &actor,
            Some(0),
            Some(1_000),
        );

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::MilestoneApproved);
        assert_eq!(e.milestone_idx, Some(0));
        assert_eq!(e.amount, Some(1_000));
    }

    #[test]
    fn log_milestone_rejected() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::MilestoneRejected,
            &actor,
            Some(1),
            None,
        );

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::MilestoneRejected);
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
            Some(10_000),
        );

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::GrantCancelled);
        assert_eq!(e.amount, Some(10_000));
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

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::GrantCompleted);
    }

    #[test]
    fn log_dispute_raised() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::DisputeRaised,
            &actor,
            Some(0),
            None,
        );

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::DisputeRaised);
    }

    #[test]
    fn log_dispute_resolved() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::DisputeResolved,
            &actor,
            Some(0),
            Some(2_500),
        );

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::DisputeResolved);
    }

    #[test]
    fn log_split_registered() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::SplitRegistered,
            &actor,
            Some(1),
            None,
        );

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::SplitRegistered);
    }

    #[test]
    fn log_snapshot_captured() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::SnapshotCaptured,
            &actor,
            None,
            None,
        );

        let entries = get_log(&env, grant_id);
        let e = entries.get(0).unwrap();
        assert_eq!(e.action, AuditAction::SnapshotCaptured);
    }

    // ── Metadata verification ──────────────────────────────────────────

    #[test]
    fn audit_metadata_stored_correctly() {
        let (env, actor, grant_id) = setup();
        set_ledger(&env, 12345, 999_888_777);

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

        let entry = entries.get(0).unwrap();
        assert_eq!(entry.grant_id, grant_id);
        assert_eq!(entry.action, AuditAction::GrantCreated);
        assert_eq!(entry.actor, actor);
        assert_eq!(entry.timestamp, 999_888_777);
        assert_eq!(entry.ledger_sequence, 12345);
    }

    // ── Append multiple / log length ────────────────────────────────────

    #[test]
    fn append_multiple_entries_increases_log_length() {
        let (env, actor, grant_id) = setup();

        assert_eq!(log_length(&env, grant_id), 0);

        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        assert_eq!(log_length(&env, grant_id), 1);

        log(
            &env,
            grant_id,
            AuditAction::GrantFunded,
            &actor,
            None,
            Some(1_000),
        );
        assert_eq!(log_length(&env, grant_id), 2);

        log(
            &env,
            grant_id,
            AuditAction::MilestoneSubmitted,
            &actor,
            Some(0),
            None,
        );
        assert_eq!(log_length(&env, grant_id), 3);

        let entries = get_log(&env, grant_id);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries.get(0).unwrap().action, AuditAction::GrantCreated);
        assert_eq!(entries.get(1).unwrap().action, AuditAction::GrantFunded);
        assert_eq!(
            entries.get(2).unwrap().action,
            AuditAction::MilestoneSubmitted
        );
    }

    // ── get_recent tests ────────────────────────────────────────────────

    #[test]
    fn get_recent_empty_log() {
        let (env, _, grant_id) = setup();
        let recent = get_recent(&env, grant_id, 5);
        assert_eq!(recent.len(), 0);
    }

    #[test]
    fn get_recent_zero_n() {
        let (env, actor, grant_id) = setup();
        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        let recent = get_recent(&env, grant_id, 0);
        assert_eq!(recent.len(), 0);
    }

    #[test]
    fn get_recent_fewer_entries_than_n() {
        let (env, actor, grant_id) = setup();

        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        log(
            &env,
            grant_id,
            AuditAction::GrantFunded,
            &actor,
            None,
            Some(500),
        );

        let recent = get_recent(&env, grant_id, 10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent.get(0).unwrap().action, AuditAction::GrantCreated);
        assert_eq!(recent.get(1).unwrap().action, AuditAction::GrantFunded);
    }

    #[test]
    fn get_recent_more_entries_than_n() {
        let (env, actor, grant_id) = setup();

        log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &actor,
            None,
            None,
        );
        log(
            &env,
            grant_id,
            AuditAction::GrantFunded,
            &actor,
            None,
            Some(100),
        );
        log(
            &env,
            grant_id,
            AuditAction::MilestoneSubmitted,
            &actor,
            Some(0),
            None,
        );
        log(
            &env,
            grant_id,
            AuditAction::MilestoneApproved,
            &actor,
            Some(0),
            Some(500),
        );

        // Asking for the 2 most recent out of 4 entries
        let recent = get_recent(&env, grant_id, 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(
            recent.get(0).unwrap().action,
            AuditAction::MilestoneSubmitted
        );
        assert_eq!(
            recent.get(1).unwrap().action,
            AuditAction::MilestoneApproved
        );
    }
}
