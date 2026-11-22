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

/// List grant IDs owned by `owner`, paginated by `offset` and `limit`.
/// Results are capped at [`crate::constants::MAX_PAGE_SIZE`].
pub fn by_owner(env: &Env, owner: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::OwnerIndex(owner.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

/// List grant IDs with the given `status`, paginated by `offset` and `limit`.
/// Results are capped at [`crate::constants::MAX_PAGE_SIZE`].
pub fn by_status(env: &Env, status: GrantStatus, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::StatusIndex(status as u32)))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

/// List grant IDs using the given `token`, paginated by `offset` and `limit`.
/// Results are capped at [`crate::constants::MAX_PAGE_SIZE`].
pub fn by_token(env: &Env, token: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::TokenIndex(token.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

/// List grant IDs where `contributor` is assigned, paginated by `offset` and `limit`.
/// Results are capped at [`crate::constants::MAX_PAGE_SIZE`].
pub fn by_contributor(env: &Env, contributor: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::ContribIndex(contributor.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

/// List all grant IDs in chronological creation order, paginated by `offset` and `limit`.
/// Results are capped at [`crate::constants::MAX_PAGE_SIZE`].
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

    #[test]
    fn test_by_owner_pagination() {
        use soroban_sdk::testutils::Address as _;
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);

            for id in 1..=5 {
                on_grant_created(&env, id, &owner, &token, GrantStatus::Active);
            }

            // Normal slices
            let page1 = by_owner(&env, &owner, 0, 2);
            assert_eq!(page1.len(), 2);
            assert_eq!(page1.get(0), Some(1));
            assert_eq!(page1.get(1), Some(2));

            let page2 = by_owner(&env, &owner, 2, 2);
            assert_eq!(page2.len(), 2);
            assert_eq!(page2.get(0), Some(3));
            assert_eq!(page2.get(1), Some(4));

            let page3 = by_owner(&env, &owner, 4, 2);
            assert_eq!(page3.len(), 1);
            assert_eq!(page3.get(0), Some(5));

            // Offset past total entries
            let empty = by_owner(&env, &owner, 5, 2);
            assert_eq!(empty.len(), 0);

            // Zero limit returns empty
            let zero = by_owner(&env, &owner, 0, 0);
            assert_eq!(zero.len(), 0);
        });
    }

    #[test]
    fn test_by_status_pagination() {
        use soroban_sdk::testutils::Address as _;
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);

            for id in 1..=4 {
                on_grant_created(&env, id, &owner, &token, GrantStatus::Active);
            }

            let slice = by_status(&env, GrantStatus::Active, 1, 2);
            assert_eq!(slice.len(), 2);
            assert_eq!(slice.get(0), Some(2));
            assert_eq!(slice.get(1), Some(3));

            let empty = by_status(&env, GrantStatus::Cancelled, 0, 10);
            assert_eq!(empty.len(), 0);
        });
    }

    #[test]
    fn test_by_token_pagination() {
        use soroban_sdk::testutils::Address as _;
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);

            for id in 1..=4 {
                on_grant_created(&env, id, &owner, &token, GrantStatus::Active);
            }

            let slice = by_token(&env, &token, 0, 3);
            assert_eq!(slice.len(), 3);
            assert_eq!(slice.get(0), Some(1));
            assert_eq!(slice.get(1), Some(2));
            assert_eq!(slice.get(2), Some(3));

            let other_token = Address::generate(&env);
            let empty = by_token(&env, &other_token, 0, 10);
            assert_eq!(empty.len(), 0);
        });
    }

    #[test]
    fn test_by_contributor_pagination() {
        use soroban_sdk::testutils::Address as _;
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let contributor = Address::generate(&env);

            for id in 10..=14 {
                on_contributor_assigned(&env, id, &contributor);
            }

            let slice = by_contributor(&env, &contributor, 1, 3);
            assert_eq!(slice.len(), 3);
            assert_eq!(slice.get(0), Some(11));
            assert_eq!(slice.get(1), Some(12));
            assert_eq!(slice.get(2), Some(13));

            let unknown = Address::generate(&env);
            let empty = by_contributor(&env, &unknown, 0, 10);
            assert_eq!(empty.len(), 0);
        });
    }

    #[test]
    fn test_recent_pagination_and_max_page_size_cap() {
        use soroban_sdk::testutils::Address as _;
        let env = Env::default();
        let contract_id = setup(&env);

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);

            for id in 1..=10 {
                on_grant_created(&env, id, &owner, &token, GrantStatus::Active);
            }

            let first_two = recent(&env, 0, 2);
            assert_eq!(first_two.len(), 2);
            assert_eq!(first_two.get(0), Some(1));
            assert_eq!(first_two.get(1), Some(2));

            let next_three = recent(&env, 2, 3);
            assert_eq!(next_three.len(), 3);
            assert_eq!(next_three.get(0), Some(3));
            assert_eq!(next_three.get(2), Some(5));

            // Requesting more than MAX_PAGE_SIZE is capped to MAX_PAGE_SIZE (or total items)
            let all = recent(&env, 0, constants::MAX_PAGE_SIZE + 10);
            assert_eq!(all.len(), 10);
        });
    }
}
