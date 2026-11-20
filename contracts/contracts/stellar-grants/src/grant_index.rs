use soroban_sdk::{Address, Env, Vec};

use crate::constants;
use crate::events::Events;
use crate::storage::{DataKey, GrantKey};
use crate::types::GrantStatus;

/// Push `grant_id` onto the index list at `key`, capped at `cap` entries.
/// Returns `true` if the id is present in the list afterwards (either just
/// added or already there), `false` if the list was already at capacity and
/// the id had to be dropped.
///
/// Before issue #698, a cap hit was a silent no-op: `grant_create` still
/// succeeded, but the grant became permanently invisible to `by_owner`,
/// `by_status`, `by_token`, `recent`, and `data_export` for that index, with
/// no error or event to reveal it. Surfacing an `IndexCapReached` event here
/// at least makes the condition observable instead of a silent data loss.
fn push_to_index(env: &Env, key: &DataKey, grant_id: u64, cap: u32) -> bool {
    let mut list: Vec<u64> = env
        .storage()
        .persistent()
        .get(key)
        .unwrap_or_else(|| Vec::new(env));
    if list.contains(grant_id) {
        return true;
    }
    if list.len() >= cap {
        Events::emit_index_cap_reached(env, grant_id);
        return false;
    }
    list.push_back(grant_id);
    env.storage().persistent().set(key, &list);
    true
}

fn remove_from_index(env: &Env, key: &DataKey, grant_id: u64) {
    let mut list: Vec<u64> = env
        .storage()
        .persistent()
        .get(key)
        .unwrap_or_else(|| Vec::new(env));
    if let Some(pos) = (0..list.len()).find(|&i| list.get(i) == Some(grant_id)) {
        list.remove(pos);
        env.storage().persistent().set(key, &list);
    }
}

pub fn on_grant_created(
    env: &Env,
    grant_id: u64,
    owner: &Address,
    token: &Address,
    status: GrantStatus,
) {
    let cap = constants::MAX_INDEX_ENTRIES;
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::OwnerIndex(owner.clone())),
        grant_id,
        cap,
    );
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::StatusIndex(status as u32)),
        grant_id,
        cap,
    );
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::TokenIndex(token.clone())),
        grant_id,
        cap,
    );
    push_to_index(env, &DataKey::Grant(GrantKey::GlobalOrder), grant_id, cap);
}

pub fn on_status_changed(
    env: &Env,
    grant_id: u64,
    old_status: GrantStatus,
    new_status: GrantStatus,
) {
    if old_status != new_status {
        // Push to the new-status index first. If the index is full the
        // grant stays discoverable under its old status rather than being
        // orphaned in neither index.
        let pushed = push_to_index(
            env,
            &DataKey::Grant(GrantKey::StatusIndex(new_status as u32)),
            grant_id,
            constants::MAX_INDEX_ENTRIES,
        );
        if pushed {
            remove_from_index(
                env,
                &DataKey::Grant(GrantKey::StatusIndex(old_status as u32)),
                grant_id,
            );
        }
    }
}

pub fn on_contributor_assigned(env: &Env, grant_id: u64, contributor: &Address) {
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::ContribIndex(contributor.clone())),
        grant_id,
        constants::MAX_INDEX_ENTRIES,
    );
}

pub fn by_owner(env: &Env, owner: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::OwnerIndex(owner.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn by_status(env: &Env, status: GrantStatus, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::StatusIndex(status as u32)))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn by_token(env: &Env, token: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::TokenIndex(token.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn by_contributor(env: &Env, contributor: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::ContribIndex(contributor.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn recent(env: &Env, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::GlobalOrder))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn index_counts(env: &Env, owner: Option<&Address>) -> (u32, u32, u32) {
    let owned = if let Some(o) = owner {
        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::OwnerIndex(o.clone())))
            .unwrap_or_else(|| Vec::new(env));
        list.len()
    } else {
        0
    };
    let active: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::StatusIndex(
            GrantStatus::Active as u32,
        )))
        .unwrap_or_else(|| Vec::new(env));
    let contributed = if let Some(o) = owner {
        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::ContribIndex(o.clone())))
            .unwrap_or_else(|| Vec::new(env));
        list.len()
    } else {
        0
    };
    (owned, active.len(), contributed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::Events as _;
    use soroban_sdk::Env;
    extern crate alloc;
    use alloc::format;

    fn setup(env: &Env) -> Address {
        env.mock_all_auths();
        env.register(StellarGrantsContract, ())
    }

    #[test]
    fn test_push_to_index_respects_cap_and_surfaces_the_drop() {
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let key = DataKey::Grant(GrantKey::GlobalOrder);
            let cap = 3u32;

            assert!(push_to_index(&env, &key, 1, cap));
            assert!(push_to_index(&env, &key, 2, cap));
            assert!(push_to_index(&env, &key, 3, cap));

            let list: Vec<u64> = env.storage().persistent().get(&key).unwrap();
            assert_eq!(list.len(), 3);

            let accepted = push_to_index(&env, &key, 4, cap);
            assert!(!accepted, "push beyond the cap must report failure");

            let list: Vec<u64> = env.storage().persistent().get(&key).unwrap();
            assert_eq!(list.len(), 3, "list must not silently grow past the cap");
            assert!(
                !list.contains(4),
                "dropped id must not silently appear in the index"
            );

            let events = env.events().all();
            let mut found_cap_event = false;
            for e in events.events() {
                if format!("{:?}", e).contains("index_cap_reached") {
                    found_cap_event = true;
                }
            }
            assert!(
                found_cap_event,
                "IndexCapReached event must be emitted instead of silently dropping the entry"
            );
        });
    }

    #[test]
    fn test_push_to_index_dedupes_without_double_counting_against_cap() {
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let key = DataKey::Grant(GrantKey::GlobalOrder);
            let cap = 2u32;

            assert!(push_to_index(&env, &key, 1, cap));
            assert!(push_to_index(&env, &key, 2, cap));

            // Re-pushing an id already in a full list is a no-op success, not a
            // cap breach — it must not emit a spurious IndexCapReached event.
            assert!(push_to_index(&env, &key, 1, cap));

            let list: Vec<u64> = env.storage().persistent().get(&key).unwrap();
            assert_eq!(list.len(), 2);
        });
    }
}
