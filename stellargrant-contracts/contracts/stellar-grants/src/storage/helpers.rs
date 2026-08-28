use super::DataKey;
use crate::types::{AuditEntry, Snapshot, SplitRecipient};
use soroban_sdk::{contracttype, Env, Vec};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrantKey {
    AuditLog(u64),
    AuditLogPageCount(u64),
    AuditLogPage(u64, u32),
}

pub const MAX_ENTRIES_PER_PAGE: u32 = 200;
pub const AUDIT_TTL_THRESHOLD: u32 = 172800;
pub const AUDIT_TTL_EXTEND_TO: u32 = 518400;

pub const SNAPSHOT_TTL_THRESHOLD: u32 = 172800;
pub const SNAPSHOT_TTL_EXTEND_TO: u32 = 518400;

pub fn append_audit_entry(env: &Env, grant_id: u64, entry: &AuditEntry) {
    let page_count_key = DataKey::GrantKey(GrantKey::AuditLogPageCount(grant_id));
    let mut page_num: u32 = env.storage().persistent().get(&page_count_key).unwrap_or(0);
    let page_key = DataKey::GrantKey(GrantKey::AuditLogPage(grant_id, page_num));
    let mut page: Vec<AuditEntry> = env
        .storage()
        .persistent()
        .get(&page_key)
        .unwrap_or_else(|| Vec::new(env));
    if page.len() >= MAX_ENTRIES_PER_PAGE {
        page_num += 1;
        env.storage().persistent().set(&page_count_key, &page_num);
        page = Vec::new(env);
    }
    page.push_back(entry.clone());
    let key = DataKey::GrantKey(GrantKey::AuditLogPage(grant_id, page_num));
    env.storage().persistent().set(&key, &page);
    env.storage()
        .persistent()
        .extend_ttl(&key, AUDIT_TTL_THRESHOLD, AUDIT_TTL_EXTEND_TO);
}

pub fn get_audit_log(env: &Env, grant_id: u64) -> Vec<AuditEntry> {
    let page_count_key = DataKey::GrantKey(GrantKey::AuditLogPageCount(grant_id));
    let page_count: u32 = env.storage().persistent().get(&page_count_key).unwrap_or(0);
    let mut full_log = Vec::new(env);
    for page_num in 0..=page_count {
        let page_key = DataKey::GrantKey(GrantKey::AuditLogPage(grant_id, page_num));
        if let Some(page) = env
            .storage()
            .persistent()
            .get::<_, Vec<AuditEntry>>(&page_key)
        {
            for entry in page.iter() {
                full_log.push_back(entry);
            }
        }
    }
    full_log
}

pub fn set_snapshot(env: &Env, grant_id: u64, snapshot_id: u32, snapshot: &Snapshot) {
    let key = DataKey::Snapshot(grant_id, snapshot_id);
    env.storage().persistent().set(&key, snapshot);
    env.storage()
        .persistent()
        .extend_ttl(&key, SNAPSHOT_TTL_THRESHOLD, SNAPSHOT_TTL_EXTEND_TO);
}

pub fn get_snapshot(env: &Env, grant_id: u64, snapshot_id: u32) -> Option<Snapshot> {
    let key = DataKey::Snapshot(grant_id, snapshot_id);
    env.storage().persistent().get(&key)
}

pub fn set_snapshot_list(env: &Env, grant_id: u64, snapshots: &Vec<u32>) {
    let key = DataKey::SnapshotList(grant_id);
    env.storage().persistent().set(&key, snapshots);
    env.storage()
        .persistent()
        .extend_ttl(&key, SNAPSHOT_TTL_THRESHOLD, SNAPSHOT_TTL_EXTEND_TO);
}

pub fn get_snapshot_list(env: &Env, grant_id: u64) -> Vec<u32> {
    let key = DataKey::SnapshotList(grant_id);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_split_recipients(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    recipients: &Vec<SplitRecipient>,
) {
    let key = DataKey::SplitRecipients(grant_id, milestone_idx);
    env.storage().persistent().set(&key, recipients);
}

pub fn get_split_recipients(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
) -> Option<Vec<SplitRecipient>> {
    let key = DataKey::SplitRecipients(grant_id, milestone_idx);
    env.storage().persistent().get(&key)
}
