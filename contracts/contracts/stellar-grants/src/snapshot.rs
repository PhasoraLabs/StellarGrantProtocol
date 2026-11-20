use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

use crate::rate_limit;
use crate::storage::helpers::{PERSISTENT_TTL_EXTEND_TO, PERSISTENT_TTL_THRESHOLD};
use crate::storage::Storage;
use crate::types::{
    ContractError, MilestoneState, RateLimitAction, SnapshotTrigger, StateSnapshot,
};

#[contracttype]
pub enum SnapshotKey {
    Counter(u64),
    One(u64, u32),
    List(u64),
}

fn bump(env: &Env, key: &SnapshotKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

fn snapshot_ids(env: &Env, grant_id: u64) -> Vec<u32> {
    let key = SnapshotKey::List(grant_id);
    let ids = env.storage().persistent().get(&key);
    if ids.is_some() {
        bump(env, &key);
    }
    ids.unwrap_or_else(|| Vec::new(env))
}

fn next_id(env: &Env, grant_id: u64) -> u32 {
    let key = SnapshotKey::Counter(grant_id);
    let mut id: u32 = env.storage().persistent().get(&key).unwrap_or(0);
    id = id.saturating_add(1);
    env.storage().persistent().set(&key, &id);
    bump(env, &key);
    id
}

pub fn capture(
    env: &Env,
    grant_id: u64,
    trigger: SnapshotTrigger,
    captured_by: &Address,
) -> Result<u32, ContractError> {
    rate_limit::check_and_increment(env, captured_by, RateLimitAction::SnapshotCapture)?;

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    let is_related = grant.owner == *captured_by
        || grant.reviewers.contains(captured_by.clone())
        || Storage::get_global_admin(env) == Some(captured_by.clone());
    if !is_related {
        return Err(ContractError::Unauthorized);
    }

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

    let key = SnapshotKey::One(grant_id, id);
    env.storage().persistent().set(&key, &snapshot);
    bump(env, &key);
    let mut ids = snapshot_ids(env, grant_id);
    ids.push_back(id);
    let key = SnapshotKey::List(grant_id);
    env.storage().persistent().set(&key, &ids);
    bump(env, &key);

    Ok(id)
}

pub fn get_snapshot(
    env: &Env,
    grant_id: u64,
    snapshot_id: u32,
) -> Result<StateSnapshot, ContractError> {
    let key = SnapshotKey::One(grant_id, snapshot_id);
    let snapshot = env.storage().persistent().get(&key);
    if snapshot.is_some() {
        bump(env, &key);
    }
    snapshot.ok_or(ContractError::InvalidState)
}

pub fn list_snapshots(env: &Env, grant_id: u64, offset: u32, limit: u32) -> Vec<StateSnapshot> {
    let ids = snapshot_ids(env, grant_id);
    let page = crate::pagination::paginate(env, &ids, offset, limit);
    let mut out = Vec::new(env);
    for id in page.iter() {
        if let Ok(s) = get_snapshot(env, grant_id, id) {
            out.push_back(s);
        }
    }
    out
}

pub fn list_snapshots_all(env: &Env, grant_id: u64) -> Vec<StateSnapshot> {
    let ids = snapshot_ids(env, grant_id);
    let mut out = Vec::new(env);
    for id in ids.iter() {
        if let Ok(s) = get_snapshot(env, grant_id, id) {
            out.push_back(s);
        }
    }
    out
}

pub fn latest_snapshot(env: &Env, grant_id: u64) -> Option<StateSnapshot> {
    let list = list_snapshots_all(env, grant_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantStatus};
    use soroban_sdk::{
        testutils::{storage::Persistent as _, Address as _, Ledger},
        String,
    };

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.ledger().with_mut(|ledger| {
            ledger.max_entry_ttl = PERSISTENT_TTL_EXTEND_TO * 2;
        });
        let contract = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract, || {
            // Use the admin exemption so the fixture isolates snapshot retention.
            Storage::set_global_admin(&env, &owner);
            env.storage().persistent().extend_ttl(
                &crate::storage::DataKey::GlobalAdmin,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
            Storage::set_grant(
                &env,
                1,
                &Grant {
                    id: 1,
                    owner: owner.clone(),
                    title: String::from_str(&env, "Snapshot retention"),
                    description: String::from_str(&env, "Dispute evidence"),
                    token: Address::generate(&env),
                    status: GrantStatus::Active,
                    total_amount: 100,
                    milestone_amount: 100,
                    reviewers: Vec::new(&env),
                    total_milestones: 1,
                    milestones_paid_out: 0,
                    escrow_balance: 100,
                    funders: Vec::new(&env),
                    reason: None,
                    timestamp: 0,
                    require_compliance: None,
                },
            );
        });
        (env, contract, owner)
    }

    fn decay_ttl(env: &Env) {
        env.ledger().with_mut(|ledger| {
            ledger.sequence_number += PERSISTENT_TTL_EXTEND_TO - PERSISTENT_TTL_THRESHOLD / 2;
        });
    }

    #[test]
    fn capture_extends_all_written_keys_and_preserves_sequence() {
        let (env, contract, owner) = setup();
        env.as_contract(&contract, || {
            assert_eq!(
                capture(&env, 1, SnapshotTrigger::AdminRequest, &owner),
                Ok(1)
            );
            for key in [
                SnapshotKey::One(1, 1),
                SnapshotKey::List(1),
                SnapshotKey::Counter(1),
            ] {
                assert_eq!(
                    env.storage().persistent().get_ttl(&key),
                    PERSISTENT_TTL_EXTEND_TO
                );
            }
        });

        decay_ttl(&env);
        env.as_contract(&contract, || {
            for key in [SnapshotKey::List(1), SnapshotKey::Counter(1)] {
                assert!(env.storage().persistent().get_ttl(&key) < PERSISTENT_TTL_THRESHOLD);
            }
            assert_eq!(
                capture(&env, 1, SnapshotTrigger::AdminRequest, &owner),
                Ok(2)
            );
            for key in [
                SnapshotKey::One(1, 2),
                SnapshotKey::List(1),
                SnapshotKey::Counter(1),
            ] {
                assert_eq!(
                    env.storage().persistent().get_ttl(&key),
                    PERSISTENT_TTL_EXTEND_TO
                );
            }
            assert_eq!(list_snapshots_all(&env, 1).len(), 2);
        });
    }

    #[test]
    fn get_snapshot_renews_existing_entry() {
        let (env, contract, owner) = setup();
        let snapshot = env.as_contract(&contract, || {
            capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();
            get_snapshot(&env, 1, 1).unwrap()
        });
        decay_ttl(&env);
        env.as_contract(&contract, || {
            assert!(
                env.storage().persistent().get_ttl(&SnapshotKey::One(1, 1))
                    < PERSISTENT_TTL_THRESHOLD
            );
            assert_eq!(get_snapshot(&env, 1, 1), Ok(snapshot));
            assert_eq!(
                env.storage().persistent().get_ttl(&SnapshotKey::One(1, 1)),
                PERSISTENT_TTL_EXTEND_TO
            );
        });
    }

    #[test]
    fn snapshot_lists_renew_index_and_returned_entries() {
        for paginated in [true, false] {
            let (env, contract, owner) = setup();
            env.as_contract(&contract, || {
                capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();
            });
            decay_ttl(&env);
            env.as_contract(&contract, || {
                for key in [SnapshotKey::List(1), SnapshotKey::One(1, 1)] {
                    assert!(env.storage().persistent().get_ttl(&key) < PERSISTENT_TTL_THRESHOLD);
                }
                let snapshots = if paginated {
                    list_snapshots(&env, 1, 0, 10)
                } else {
                    list_snapshots_all(&env, 1)
                };
                assert_eq!(snapshots.len(), 1);
                assert_eq!(snapshots.get(0).unwrap().id, 1);
                for key in [SnapshotKey::List(1), SnapshotKey::One(1, 1)] {
                    assert_eq!(
                        env.storage().persistent().get_ttl(&key),
                        PERSISTENT_TTL_EXTEND_TO
                    );
                }
            });
        }
    }

    #[test]
    fn missing_snapshots_keep_existing_empty_and_error_results() {
        let (env, contract, _) = setup();
        env.as_contract(&contract, || {
            assert_eq!(get_snapshot(&env, 1, 1), Err(ContractError::InvalidState));
            assert!(list_snapshots(&env, 1, 0, 10).is_empty());
            assert!(list_snapshots_all(&env, 1).is_empty());
            assert_eq!(latest_snapshot(&env, 1), None);
            assert!(!env.storage().persistent().has(&SnapshotKey::List(1)));
        });
    }
}
