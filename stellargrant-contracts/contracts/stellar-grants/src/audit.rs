use crate::storage::Storage;
use crate::types::AuditEntry;
use soroban_sdk::{Address, Env, String, Vec};

pub fn log(env: &Env, grant_id: u64, action: String, actor: Address) {
    let entry = AuditEntry {
        grant_id,
        action,
        actor,
        timestamp: env.ledger().timestamp(),
    };
    Storage::append_audit_entry(env, grant_id, &entry);
}

pub fn get_audit_log(env: &Env, grant_id: u64) -> Vec<AuditEntry> {
    Storage::get_audit_log(env, grant_id)
}
