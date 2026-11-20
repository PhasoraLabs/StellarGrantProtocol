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

    /// Mutate the stored grant the way a fund/payout/cancel would, without
    /// touching the ledger, so tests can place a state change in the exact
    /// ledger a snapshot is taken in.
    fn change_grant(env: &Env, f: impl FnOnce(&mut Grant)) {
        let mut grant = Storage::get_grant(env, 1).unwrap();
        f(&mut grant);
        Storage::set_grant(env, 1, &grant);
    }

    fn set_ledger(env: &Env, sequence: u32, timestamp: u64) {
        env.ledger().with_mut(|ledger| {
            ledger.sequence_number = sequence;
            ledger.timestamp = timestamp;
        });
    }

    // Snapshots read grant state at the moment `capture` runs. Within one
    // ledger the contract has no ledger-level ordering to fall back on, so
    // call order is the only boundary: a change made before `capture` is in
    // the snapshot, a change made after it is not.

    /// A balance change applied *before* `capture` in the same ledger is
    /// visible in the snapshot ("after" semantics).
    #[test]
    fn snapshot_includes_balance_change_made_earlier_in_same_ledger() {
        let (env, contract, owner) = setup();
        set_ledger(&env, 50, 1_000);
        env.as_contract(&contract, || {
            change_grant(&env, |g| g.escrow_balance = 40);

            let id = capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();
            let snapshot = get_snapshot(&env, 1, id).unwrap();

            assert_eq!(snapshot.escrow_balance, 40);
            assert_eq!(snapshot.captured_at_ledger, 50);
            assert_eq!(snapshot.captured_at, 1_000);
        });
    }

    /// A balance change applied *after* `capture` in the same ledger is not
    /// retroactively written into the snapshot ("before" semantics).
    #[test]
    fn snapshot_excludes_balance_change_made_later_in_same_ledger() {
        let (env, contract, owner) = setup();
        set_ledger(&env, 50, 1_000);
        env.as_contract(&contract, || {
            let id = capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();

            change_grant(&env, |g| g.escrow_balance = 40);

            let snapshot = get_snapshot(&env, 1, id).unwrap();
            assert_eq!(snapshot.escrow_balance, 100);
            assert_eq!(Storage::get_grant(&env, 1).unwrap().escrow_balance, 40);
        });
    }

    /// Two snapshots straddling a change in one ledger share a ledger
    /// number and timestamp but still record different state, and the diff
    /// reports exactly the fields that moved.
    #[test]
    fn same_ledger_snapshots_straddle_a_change_and_diff_reports_it() {
        let (env, contract, owner) = setup();
        set_ledger(&env, 50, 1_000);
        env.as_contract(&contract, || {
            let before = capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();
            change_grant(&env, |g| {
                g.escrow_balance = 40;
                g.milestones_paid_out = 1;
                g.status = GrantStatus::Completed;
            });
            let after = capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();

            let a = get_snapshot(&env, 1, before).unwrap();
            let b = get_snapshot(&env, 1, after).unwrap();
            assert_eq!(a.captured_at_ledger, b.captured_at_ledger);
            assert_eq!(a.captured_at, b.captured_at);
            assert_eq!((a.escrow_balance, a.milestones_paid_out), (100, 0));
            assert_eq!((b.escrow_balance, b.milestones_paid_out), (40, 1));
            assert_eq!(a.grant_status, GrantStatus::Active);
            assert_eq!(b.grant_status, GrantStatus::Completed);

            let changes = diff_snapshots(&env, 1, before, after);
            assert_eq!(changes.len(), 3);
            assert!(changes.contains(Symbol::new(&env, "escrow_balance")));
            assert!(changes.contains(Symbol::new(&env, "milestones_paid_out")));
            assert!(changes.contains(Symbol::new(&env, "grant_status")));
            // Insertion order is preserved by `latest_snapshot`.
            assert_eq!(latest_snapshot(&env, 1).unwrap().id, after);
        });
    }

    /// Re-capturing with no intervening change in the same ledger yields an
    /// identical state and an empty diff, and ids still advance.
    #[test]
    fn same_ledger_snapshots_without_a_change_have_an_empty_diff() {
        let (env, contract, owner) = setup();
        set_ledger(&env, 50, 1_000);
        env.as_contract(&contract, || {
            let first = capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();
            let second = capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap();

            assert_eq!((first, second), (1, 2));
            assert!(diff_snapshots(&env, 1, first, second).is_empty());
        });
    }

    /// Ledger metadata is stamped at capture time: a change made on the next
    /// ledger shows up only in a snapshot taken on that ledger.
    #[test]
    fn snapshot_ledger_boundary_separates_pre_and_post_change_state() {
        let (env, contract, owner) = setup();
        set_ledger(&env, 50, 1_000);
        let before = env.as_contract(&contract, || {
            capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap()
        });

        set_ledger(&env, 51, 1_005);
        let after = env.as_contract(&contract, || {
            change_grant(&env, |g| g.escrow_balance = 40);
            capture(&env, 1, SnapshotTrigger::AdminRequest, &owner).unwrap()
        });

        env.as_contract(&contract, || {
            let a = get_snapshot(&env, 1, before).unwrap();
            let b = get_snapshot(&env, 1, after).unwrap();
            assert_eq!((a.captured_at_ledger, a.captured_at), (50, 1_000));
            assert_eq!((b.captured_at_ledger, b.captured_at), (51, 1_005));
            assert_eq!(a.escrow_balance, 100);
            assert_eq!(b.escrow_balance, 40);
        });
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
