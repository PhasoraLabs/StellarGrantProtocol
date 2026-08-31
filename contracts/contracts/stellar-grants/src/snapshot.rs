use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

use crate::types::{ContractError, MilestoneState, SnapshotTrigger, StateSnapshot};
use crate::Storage;

#[contracttype]
pub enum SnapshotKey {
    Counter(u64),
    One(u64, u32),
    List(u64),
}

fn next_id(env: &Env, grant_id: u64) -> u32 {
    let key = SnapshotKey::Counter(grant_id);
    let mut id: u32 = env.storage().persistent().get(&key).unwrap_or(0);
    id = id.saturating_add(1);
    env.storage().persistent().set(&key, &id);
    id
}

pub fn capture(
    env: &Env,
    grant_id: u64,
    trigger: SnapshotTrigger,
    captured_by: &Address,
) -> Result<u32, ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let id = next_id(env, grant_id);
    let mut states: Vec<MilestoneState> = Vec::new(env);
    for idx in 0..grant.total_milestones {
        let state = Storage::get_milestone(env, grant_id, idx)
            .map(|m| m.state)
            .unwrap_or(MilestoneState::Pending);
        states.push_back(state);
    }
    let snapshot = StateSnapshot {
        id,
        grant_id,
        trigger,
        grant_status: grant.status,
        escrow_balance: grant.escrow_balance,
        milestones_paid_out: grant.milestones_paid_out,
        total_milestones: grant.total_milestones,
        milestone_states: states,
        captured_at: env.ledger().timestamp(),
        captured_at_ledger: env.ledger().sequence(),
        captured_by: captured_by.clone(),
    };
    env.storage()
        .persistent()
        .set(&SnapshotKey::One(grant_id, id), &snapshot);
    let mut ids: Vec<u32> = env
        .storage()
        .persistent()
        .get(&SnapshotKey::List(grant_id))
        .unwrap_or_else(|| Vec::new(env));
    ids.push_back(id);
    env.storage()
        .persistent()
        .set(&SnapshotKey::List(grant_id), &ids);
    Ok(id)
}

pub fn get_snapshot(
    env: &Env,
    grant_id: u64,
    snapshot_id: u32,
) -> Result<StateSnapshot, ContractError> {
    env.storage()
        .persistent()
        .get(&SnapshotKey::One(grant_id, snapshot_id))
        .ok_or(ContractError::InvalidState)
}

pub fn list_snapshots(env: &Env, grant_id: u64) -> Vec<StateSnapshot> {
    let ids: Vec<u32> = env
        .storage()
        .persistent()
        .get(&SnapshotKey::List(grant_id))
        .unwrap_or_else(|| Vec::new(env));
    let mut out = Vec::new(env);
    for id in ids.iter() {
        if let Ok(s) = get_snapshot(env, grant_id, id) {
            out.push_back(s);
        }
    }
    out
}

pub fn latest_snapshot(env: &Env, grant_id: u64) -> Option<StateSnapshot> {
    let list = list_snapshots(env, grant_id);
    if list.is_empty() {
        None
    } else {
        list.get(list.len() - 1)
    }
}

pub fn diff_snapshots(env: &Env, grant_id: u64, a_id: u32, b_id: u32) -> Vec<Symbol> {
    let mut changes = Vec::new(env);
    let a = match get_snapshot(env, grant_id, a_id) {
        Ok(s) => s,
        Err(_) => return changes,
    };
    let b = match get_snapshot(env, grant_id, b_id) {
        Ok(s) => s,
        Err(_) => return changes,
    };
    if a.grant_status != b.grant_status {
        changes.push_back(Symbol::new(env, "grant_status"));
    }
    if a.escrow_balance != b.escrow_balance {
        changes.push_back(Symbol::new(env, "escrow_balance"));
    }
    if a.milestones_paid_out != b.milestones_paid_out {
        changes.push_back(Symbol::new(env, "milestones_paid_out"));
    }
    if a.total_milestones != b.total_milestones {
        changes.push_back(Symbol::new(env, "total_milestones"));
    }
    if a.milestone_states != b.milestone_states {
        changes.push_back(Symbol::new(env, "milestone_states"));
    }
    changes
}
