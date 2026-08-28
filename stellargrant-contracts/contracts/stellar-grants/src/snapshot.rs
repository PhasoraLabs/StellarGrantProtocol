use crate::storage::Storage;
use crate::types::{ContractError, Snapshot, SnapshotTrigger};
use soroban_sdk::{Address, Env};

pub fn capture(
    env: &Env,
    grant_id: u64,
    trigger: SnapshotTrigger,
    captured_by: &Address,
) -> Result<u32, ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    let is_related = grant.owner == *captured_by
        || grant.reviewers.contains(captured_by.clone())
        || Storage::get_global_admin(env) == Some(captured_by.clone());
    if !is_related {
        return Err(ContractError::Unauthorized);
    }

    let snapshot_id = Storage::get_snapshot_list(env, grant_id).len() + 1;

    let snapshot = Snapshot {
        snapshot_id,
        grant_id,
        trigger,
        captured_by: captured_by.clone(),
        timestamp: env.ledger().timestamp(),
        grant_state: grant,
    };

    Storage::set_snapshot(env, grant_id, snapshot_id, &snapshot);

    let mut list = Storage::get_snapshot_list(env, grant_id);
    list.push_back(snapshot_id);
    Storage::set_snapshot_list(env, grant_id, &list);

    Ok(snapshot_id)
}
