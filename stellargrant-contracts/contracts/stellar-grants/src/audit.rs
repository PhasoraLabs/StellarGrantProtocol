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

    let start = if len > n { len - n } else { 0 };
    pagination::paginate(env, &log, start, n)
}

/// Return the count of audit entries for a grant.
#[allow(dead_code)]
pub fn log_length(env: &Env, grant_id: u64) -> u32 {
    Storage::get_audit_log(env, grant_id).len()
}
