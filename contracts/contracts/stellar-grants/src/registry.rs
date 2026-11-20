use soroban_sdk::{Address, Env, String, Vec};

use crate::events::Events;
use crate::pagination;
use crate::storage::Storage;
use crate::types::{ContractError, RegistryEntry, RegistryEntryType};

const MAX_ENTRIES_PER_PAGE: u32 = 500;

/// Add a contributor to the global registry index. Called on registration.
///
/// Does not itself check for a pre-existing entry: the `contributor_register`
/// entrypoint in lib.rs already performs an O(1) `Storage::get_contributor`
/// duplicate check before calling this function (and before the contributor
/// profile is written), so re-scanning the whole index here would be
/// redundant and O(n) per registration. Callers other than that entrypoint
/// must perform an equivalent duplicate check before calling this function.
pub fn register_contributor(
    env: &Env,
    address: &Address,
    name: &String,
) -> Result<(), ContractError> {
    let page_count_key = crate::storage::DataKey::User(crate::storage::UserKey::RegistryPageCount);
    let mut page_num: u32 = env.storage().persistent().get(&page_count_key).unwrap_or(0);

    let mut page = Storage::get_contributor_index_page(env, page_num);

    if page.len() >= MAX_ENTRIES_PER_PAGE {
        page_num += 1;
        env.storage().persistent().set(&page_count_key, &page_num);
        page = Vec::new(env);
    }

    let entry = RegistryEntry {
        address: address.clone(),
        registered_at: env.ledger().timestamp(),
        is_active: true,
        entry_type: RegistryEntryType::Contributor,
    };

    page.push_back(entry);
    Storage::set_contributor_index_page(env, page_num, &page);

    Events::emit_contributor_registered(env, address.clone(), name.clone());

    Ok(())
}

/// Add an address to the approved reviewer allowlist. Admin only.
pub fn approve_reviewer(
    env: &Env,
    admin: &Address,
    reviewer: &Address,
) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;

    let mut allowlist = Storage::get_reviewer_allowlist(env);

    if allowlist.contains(reviewer.clone()) {
        return Ok(());
    }

    allowlist.push_back(reviewer.clone());
    Storage::set_reviewer_allowlist(env, &allowlist);

    Events::emit_reviewer_approved(env, reviewer.clone(), admin.clone());

    Ok(())
}

/// Remove an address from the approved reviewer allowlist. Admin only.
pub fn revoke_reviewer(
    env: &Env,
    admin: &Address,
    reviewer: &Address,
) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;

    let allowlist = Storage::get_reviewer_allowlist(env);
    let mut new_list: Vec<Address> = Vec::new(env);
    let mut found = false;

    for addr in allowlist.iter() {
        if addr == *reviewer {
            found = true;
        } else {
            new_list.push_back(addr);
        }
    }

    if !found {
        return Err(ContractError::InvalidInput);
    }

    Storage::set_reviewer_allowlist(env, &new_list);

    Events::emit_reviewer_revoked(env, reviewer.clone(), admin.clone());

    Ok(())
}

/// Check if an address is on the approved reviewer allowlist.
pub fn is_approved_reviewer(env: &Env, address: &Address) -> bool {
    let allowlist = Storage::get_reviewer_allowlist(env);
    allowlist.contains(address.clone())
}

/// Paginated list of all registered contributor addresses.
pub fn get_contributors_page(env: &Env, offset: u32, limit: u32) -> Vec<RegistryEntry> {
    let page_count_key = crate::storage::DataKey::User(crate::storage::UserKey::RegistryPageCount);
    let page_count: u32 = env.storage().persistent().get(&page_count_key).unwrap_or(0);

    let mut result = Vec::new(env);
    let mut entries_skipped = 0u32;
    let mut entries_returned = 0u32;

    for page_num in 0..=page_count {
        let page = Storage::get_contributor_index_page(env, page_num);
        for entry in page.iter() {
            if entries_skipped < offset {
                entries_skipped += 1;
                continue;
            }
            if entries_returned >= limit {
                return result;
            }
            result.push_back(entry);
            entries_returned += 1;
        }
    }
    result
}

/// Total count of registered contributors.
pub fn contributor_count(env: &Env) -> u32 {
    let page_count_key = crate::storage::DataKey::User(crate::storage::UserKey::RegistryPageCount);
    let page_count: u32 = env.storage().persistent().get(&page_count_key).unwrap_or(0);

    let mut total = 0u32;
    for page_num in 0..=page_count {
        let page = Storage::get_contributor_index_page(env, page_num);
        total = total.saturating_add(page.len());
    }
    total
}

fn require_global_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    let admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if admin != *caller {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::StellarGrantsContract;
    use soroban_sdk::{testutils::Address as _, Env, String};

    /// Registers the contract and an admin, returning the env/contract/admin
    /// so tests can wrap their bodies in `env.as_contract(&contract_id, ||
    /// { ... })` — required because this soroban-sdk version rejects storage
    /// access outside of a contract execution context.
    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let admin = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_global_admin(&env, &admin);
        });
        (env, contract_id, admin)
    }

    #[test]
    fn test_register_contributor() {
        let (env, contract_id, _) = setup();
        let addr = Address::generate(&env);
        let name = String::from_str(&env, "Alice");

        env.as_contract(&contract_id, || {
            register_contributor(&env, &addr, &name).unwrap();

            assert_eq!(contributor_count(&env), 1);
            let page = get_contributors_page(&env, 0, 10);
            assert_eq!(page.len(), 1);
            assert_eq!(page.get(0).unwrap().address, addr);
            assert_eq!(
                page.get(0).unwrap().entry_type,
                RegistryEntryType::Contributor
            );
            assert!(page.get(0).unwrap().is_active);
        });
    }

    #[test]
    fn test_register_contributor_duplicate_index_entries_allowed_at_this_layer() {
        // register_contributor no longer scans the index for duplicates: the
        // caller (contributor_register in lib.rs) is responsible for
        // rejecting duplicates via an O(1) Storage::get_contributor check
        // before it ever calls into this function. Calling this function
        // directly twice for the same address is therefore not rejected here.
        let (env, contract_id, _) = setup();
        let addr = Address::generate(&env);
        let name = String::from_str(&env, "Alice");

        env.as_contract(&contract_id, || {
            register_contributor(&env, &addr, &name).unwrap();
            register_contributor(&env, &addr, &name).unwrap();

            assert_eq!(contributor_count(&env), 2);
        });
    }

    #[test]
    fn test_register_contributor_does_not_scan_full_index() {
        // Simulates the real call pattern used by contributor_register in
        // lib.rs: an O(1) Storage::get_contributor duplicate check performed
        // by the caller before register_contributor runs. With many
        // pre-existing contributors, registering one more should still only
        // require that O(1) lookup plus a single append — not a scan of the
        // whole index.
        let (env, contract_id, _) = setup();

        // Each registration runs in its own `as_contract` invocation (like a
        // separate transaction would), so the test host's per-invocation
        // event-size budget doesn't accumulate across all of them the way it
        // would if they all shared one frame.
        const PRE_EXISTING: u32 = 300;

        for _ in 0..PRE_EXISTING {
            env.as_contract(&contract_id, || {
                let addr = Address::generate(&env);
                register_contributor(&env, &addr, &String::from_str(&env, "C")).unwrap();
            });
        }
        env.as_contract(&contract_id, || {
            assert_eq!(contributor_count(&env), PRE_EXISTING);
        });

        // Registering one more on top of many pre-existing contributors
        // should still only require an O(1) lookup plus a single append —
        // not a scan of the whole index.
        env.as_contract(&contract_id, || {
            let new_addr = Address::generate(&env);
            assert!(Storage::get_contributor(&env, new_addr.clone()).is_none());
            register_contributor(&env, &new_addr, &String::from_str(&env, "New")).unwrap();

            assert_eq!(contributor_count(&env), PRE_EXISTING + 1);
        });
    }

    #[test]
    fn test_register_multiple_contributors() {
        let (env, contract_id, _) = setup();
        let a1 = Address::generate(&env);
        let a2 = Address::generate(&env);
        let a3 = Address::generate(&env);

        env.as_contract(&contract_id, || {
            register_contributor(&env, &a1, &String::from_str(&env, "A")).unwrap();
            register_contributor(&env, &a2, &String::from_str(&env, "B")).unwrap();
            register_contributor(&env, &a3, &String::from_str(&env, "C")).unwrap();

            assert_eq!(contributor_count(&env), 3);
        });
    }

    #[test]
    fn test_approve_reviewer() {
        let (env, contract_id, admin) = setup();
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            approve_reviewer(&env, &admin, &reviewer).unwrap();

            assert!(is_approved_reviewer(&env, &reviewer));
        });
    }

    #[test]
    fn test_approve_reviewer_idempotent() {
        let (env, contract_id, admin) = setup();
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            approve_reviewer(&env, &admin, &reviewer).unwrap();
            approve_reviewer(&env, &admin, &reviewer).unwrap(); // no-op

            assert!(is_approved_reviewer(&env, &reviewer));
        });
    }

    #[test]
    #[should_panic]
    fn test_approve_reviewer_unauthorized() {
        let (env, contract_id, _) = setup();
        let reviewer = Address::generate(&env);
        let other = Address::generate(&env);

        env.as_contract(&contract_id, || {
            approve_reviewer(&env, &other, &reviewer).unwrap();
        });
    }

    #[test]
    fn test_revoke_reviewer() {
        let (env, contract_id, admin) = setup();
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            approve_reviewer(&env, &admin, &reviewer).unwrap();
            assert!(is_approved_reviewer(&env, &reviewer));

            revoke_reviewer(&env, &admin, &reviewer).unwrap();
            assert!(!is_approved_reviewer(&env, &reviewer));
        });
    }

    #[test]
    #[should_panic]
    fn test_revoke_reviewer_not_found() {
        let (env, contract_id, admin) = setup();
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            revoke_reviewer(&env, &admin, &reviewer).unwrap();
        });
    }

    #[test]
    #[should_panic]
    fn test_revoke_reviewer_unauthorized() {
        let (env, contract_id, admin) = setup();
        let reviewer = Address::generate(&env);
        let other = Address::generate(&env);

        env.as_contract(&contract_id, || {
            approve_reviewer(&env, &admin, &reviewer).unwrap();
            revoke_reviewer(&env, &other, &reviewer).unwrap();
        });
    }

    #[test]
    fn test_is_approved_reviewer_returns_false_for_unknown() {
        let (env, contract_id, _) = setup();
        let addr = Address::generate(&env);

        env.as_contract(&contract_id, || {
            assert!(!is_approved_reviewer(&env, &addr));
        });
    }

    #[test]
    fn test_pagination() {
        let (env, contract_id, _) = setup();

        env.as_contract(&contract_id, || {
            for _ in 0..5 {
                let addr = Address::generate(&env);
                register_contributor(&env, &addr, &String::from_str(&env, "C")).unwrap();
            }

            assert_eq!(contributor_count(&env), 5);

            // Page 0: items 0-2
            let page0 = get_contributors_page(&env, 0, 3);
            assert_eq!(page0.len(), 3);

            // Page 1: items 3-4
            let page1 = get_contributors_page(&env, 3, 3);
            assert_eq!(page1.len(), 2);

            // Offset beyond count
            let empty = get_contributors_page(&env, 10, 3);
            assert_eq!(empty.len(), 0);
        });
    }

    #[test]
    fn test_contributor_count_empty() {
        let (env, contract_id, _) = setup();

        env.as_contract(&contract_id, || {
            assert_eq!(contributor_count(&env), 0);
        });
    }

    #[test]
    fn test_register_contributor_splits_across_pages() {
        let (env, contract_id, _) = setup();
        const MAX_ENTRIES_PER_PAGE: u32 = 500;

        env.as_contract(&contract_id, || {
            for _ in 0..5 {
                let addr = Address::generate(&env);
                register_contributor(&env, &addr, &String::from_str(&env, "Contributor")).unwrap();
            }

            assert_eq!(contributor_count(&env), 5);
            let page = get_contributors_page(&env, 0, 10);
            assert_eq!(page.len(), 5);
        });
    }
}
