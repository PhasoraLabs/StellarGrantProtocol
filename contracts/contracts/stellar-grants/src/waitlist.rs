use crate::events::Events;
use crate::rate_limit;
use crate::storage::Storage;
use crate::types::{ContractError, RateLimitAction, WaitlistConfig, WaitlistEntry};
use soroban_sdk::{Address, Env, String, Vec};

/// Configure the waitlist for a grant. Owner only.
pub fn configure(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    config: WaitlistConfig,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;

    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    Storage::set_waitlist_config(env, grant_id, &config);
    Events::emit_waitlist_configured(env, grant_id, config.max_waitlist_size, config.auto_promote);
    Ok(())
}

/// Join the waitlist for a grant. Returns the position (1-indexed).
pub fn join(env: &Env, applicant: &Address, grant_id: u64) -> Result<u32, ContractError> {
    applicant.require_auth();
    rate_limit::check_and_increment(env, applicant, RateLimitAction::WaitlistJoin)?;

    let config = Storage::get_waitlist_config(env, grant_id).ok_or(ContractError::InvalidInput)?;

    let mut entries = Storage::get_waitlist_entries(env, grant_id);

    // Check if already on waitlist
    for entry in entries.iter() {
        if entry.applicant == *applicant {
            return Err(ContractError::AlreadyOnWaitlist);
        }
    }

    // Check if waitlist is full
    if entries.len() >= config.max_waitlist_size {
        return Err(ContractError::WaitlistFull);
    }

    // Get applicant's reputation score
    let profile = Storage::get_contributor(env, applicant.clone())
        .ok_or(ContractError::ContributorNotFound)?;
    let reputation_snapshot = profile.reputation_score as u32;

    let joined_at = env.ledger().timestamp();
    let position;

    if config.rank_by_reputation {
        // Insert in sorted order by reputation (highest first)
        let mut insert_idx = entries.len();
        for idx in 0..entries.len() {
            let entry = entries.get(idx).unwrap();
            if reputation_snapshot > entry.reputation_snapshot {
                insert_idx = idx;
                break;
            }
        }

        let new_entry = WaitlistEntry {
            applicant: applicant.clone(),
            grant_id,
            joined_at,
            reputation_snapshot,
            position: insert_idx + 1,
            promoted: false,
            promoted_at: None,
        };

        entries.insert(insert_idx, new_entry);
        position = insert_idx + 1;

        // Re-index all entries after insertion point
        for idx in (insert_idx + 1)..entries.len() {
            let mut entry = entries.get(idx).unwrap();
            entry.position = idx + 1;
            entries.set(idx, entry);
        }
    } else {
        // FIFO: append to end
        let new_entry = WaitlistEntry {
            applicant: applicant.clone(),
            grant_id,
            joined_at,
            reputation_snapshot,
            position: (entries.len() + 1) as u32,
            promoted: false,
            promoted_at: None,
        };

        entries.push_back(new_entry);
        position = entries.len() as u32;
    }

    Storage::set_waitlist_entries(env, grant_id, &entries);
    Events::emit_waitlist_joined(env, grant_id, applicant.clone(), position);

    Ok(position)
}

/// Leave the waitlist voluntarily.
pub fn leave(env: &Env, applicant: &Address, grant_id: u64) -> Result<(), ContractError> {
    applicant.require_auth();

    let mut entries = Storage::get_waitlist_entries(env, grant_id);

    let mut found_idx = None;
    for idx in 0..entries.len() {
        let entry = entries.get(idx).unwrap();
        if entry.applicant == *applicant {
            found_idx = Some(idx);
            break;
        }
    }

    let idx = found_idx.ok_or(ContractError::NotOnWaitlist)?;

    // Remove the entry
    entries.remove(idx);

    // Re-index remaining entries
    for i in idx..entries.len() {
        let mut entry = entries.get(i).unwrap();
        entry.position = i + 1;
        entries.set(i, entry);
    }

    Storage::set_waitlist_entries(env, grant_id, &entries);
    Events::emit_waitlist_left(env, grant_id, applicant.clone());

    Ok(())
}

/// Promote the top-ranked entry. Called when a slot opens.
/// Returns the promoted address if successful, None if waitlist is empty.
pub fn promote_next(
    env: &Env,
    caller: &Address,
    grant_id: u64,
) -> Result<Option<Address>, ContractError> {
    caller.require_auth();
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }

    let config = match Storage::get_waitlist_config(env, grant_id) {
        Some(c) => c,
        None => return Ok(None),
    };
    if !config.auto_promote {
        return Ok(None);
    }

    let promoted_count = Storage::get_waitlist_promoted_count(env, grant_id);
    if promoted_count >= config.max_slots {
        return Err(ContractError::WaitlistFull);
    }

    let mut entries = Storage::get_waitlist_entries(env, grant_id);

    if entries.is_empty() {
        return Ok(None);
    }

    // Get the first (highest-ranked) entry
    let promoted_entry = entries.get(0).ok_or(ContractError::InvalidState)?.clone();

    // Mark as promoted
    let mut first = entries.get(0).unwrap();
    first.promoted = true;
    first.promoted_at = Some(env.ledger().timestamp());
    entries.set(0, first);

    // Remove from waitlist
    entries.remove(0);

    // Re-index remaining entries
    for i in 0..entries.len() {
        let mut entry = entries.get(i).unwrap();
        entry.position = i + 1;
        entries.set(i, entry);
    }

    Storage::set_waitlist_entries(env, grant_id, &entries);
    Storage::set_waitlist_promoted_count(env, grant_id, promoted_count + 1);
    Events::emit_waitlist_promoted(env, grant_id, promoted_entry.applicant.clone(), 1);

    Ok(Some(promoted_entry.applicant))
}

/// Return all entries, sorted by reputation (or FIFO).
pub fn get_waitlist(env: &Env, grant_id: u64) -> Vec<WaitlistEntry> {
    Storage::get_waitlist_entries(env, grant_id)
}

/// Return an applicant's current position (1-indexed).
pub fn position_of(env: &Env, applicant: &Address, grant_id: u64) -> Option<u32> {
    let entries = Storage::get_waitlist_entries(env, grant_id);
    for entry in entries.iter() {
        if entry.applicant == *applicant {
            return Some(entry.position);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::{Address as _, Events, Ledger as _};
    use soroban_sdk::{Address, String};
    use std::boxed::Box;

    /// Register the contract and return its address so tests can wrap
    /// storage-touching calls in `env.as_contract(&contract_id, || { ... })`
    /// — required because this soroban-sdk version rejects storage access
    /// (and thus `require_auth()`) outside of a contract execution context.
    ///
    /// Under `mock_all_auths()`, calling `require_auth()` twice for the same
    /// address within one `as_contract` block trips "frame is already
    /// authorized" (direct Rust calls don't create the separate invocation
    /// frames a real dispatched contract call would). When a test needs the
    /// same address to go through a second authorized call (e.g. `join`
    /// then `leave` for the same applicant), give that second call its own
    /// `as_contract` block.
    fn register(env: &Env) -> Address {
        env.register(StellarGrantsContract, ())
    }

    fn setup_grant(env: &Env, grant_id: u64, owner: &Address) {
        let grant = crate::types::Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Test"),
            token: Address::generate(env),
            status: crate::types::GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 0,
            reviewers: Vec::new(env),
            total_milestones: 0,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);
    }

    fn setup_contributor(env: &Env, address: &Address, reputation_score: u64) {
        let profile = crate::types::ContributorProfile {
            contributor: address.clone(),
            name: String::from_str(env, "Applicant"),
            reputation_score,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            bio: String::from_str(env, ""),
            skills: Vec::new(env),
            github_url: String::from_str(env, ""),
            registration_timestamp: 0,
            grants_count: 0,
            last_action_at: 0,
        };
        Storage::set_contributor(env, address.clone(), &profile);
    }

    #[test]
    fn test_join_waitlist_reputation_ranked() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);
        let applicant3 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: true,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();

            // Reputations: applicant1 (500), applicant2 (800), applicant3 (600)
            setup_contributor(&env, &applicant1, 500);
            setup_contributor(&env, &applicant2, 800);
            setup_contributor(&env, &applicant3, 600);

            join(&env, &applicant1, grant_id).unwrap();
            join(&env, &applicant2, grant_id).unwrap();
            join(&env, &applicant3, grant_id).unwrap();

            let waitlist = get_waitlist(&env, grant_id);
            assert_eq!(waitlist.len(), 3);

            // Verify order: highest reputation first (800, 600, 500)
            assert_eq!(waitlist.get(0).unwrap().applicant, applicant2);
            assert_eq!(waitlist.get(0).unwrap().position, 1);
            assert_eq!(waitlist.get(1).unwrap().applicant, applicant3);
            assert_eq!(waitlist.get(1).unwrap().position, 2);
            assert_eq!(waitlist.get(2).unwrap().applicant, applicant1);
            assert_eq!(waitlist.get(2).unwrap().position, 3);
        });
    }

    #[test]
    fn test_join_waitlist_fifo() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);
        let applicant3 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();

            setup_contributor(&env, &applicant1, 500);
            setup_contributor(&env, &applicant2, 500);
            setup_contributor(&env, &applicant3, 500);

            join(&env, &applicant1, grant_id).unwrap();
            join(&env, &applicant2, grant_id).unwrap();
            join(&env, &applicant3, grant_id).unwrap();

            let waitlist = get_waitlist(&env, grant_id);
            assert_eq!(waitlist.len(), 3);

            // Verify FIFO order
            assert_eq!(waitlist.get(0).unwrap().applicant, applicant1);
            assert_eq!(waitlist.get(1).unwrap().applicant, applicant2);
            assert_eq!(waitlist.get(2).unwrap().applicant, applicant3);
        });
    }

    #[test]
    fn test_leave_waitlist() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);
        let applicant3 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();

            setup_contributor(&env, &applicant1, 500);
            setup_contributor(&env, &applicant2, 500);
            setup_contributor(&env, &applicant3, 500);

            join(&env, &applicant1, grant_id).unwrap();
            join(&env, &applicant2, grant_id).unwrap();
            join(&env, &applicant3, grant_id).unwrap();
        });

        // leave() re-authorizes applicant2, which under mock_all_auths must
        // happen in its own invocation frame (see `register`'s doc comment).
        env.as_contract(&contract_id, || {
            // Leave from middle
            leave(&env, &applicant2, grant_id).unwrap();

            let waitlist = get_waitlist(&env, grant_id);
            assert_eq!(waitlist.len(), 2);
            assert_eq!(waitlist.get(0).unwrap().applicant, applicant1);
            assert_eq!(waitlist.get(0).unwrap().position, 1);
            assert_eq!(waitlist.get(1).unwrap().applicant, applicant3);
            assert_eq!(waitlist.get(1).unwrap().position, 2);
        });
    }

    #[test]
    fn test_waitlist_full() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);
        let applicant3 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 2,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();

            setup_contributor(&env, &applicant1, 500);
            setup_contributor(&env, &applicant2, 500);
            setup_contributor(&env, &applicant3, 500);

            join(&env, &applicant1, grant_id).unwrap();
            join(&env, &applicant2, grant_id).unwrap();

            // Third applicant should fail
            let result = join(&env, &applicant3, grant_id);
            assert_eq!(result, Err(ContractError::WaitlistFull));
        });
    }

    #[test]
    fn test_promote_next() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();

            setup_contributor(&env, &applicant1, 500);
            setup_contributor(&env, &applicant2, 500);

            join(&env, &applicant1, grant_id).unwrap();
            join(&env, &applicant2, grant_id).unwrap();

            // Promote first
            let promoted = promote_next(&env, &owner, grant_id).unwrap();
            assert_eq!(promoted, Some(applicant1.clone()));

            let waitlist = get_waitlist(&env, grant_id);
            assert_eq!(waitlist.len(), 1);
            assert_eq!(waitlist.get(0).unwrap().applicant, applicant2);
            assert_eq!(waitlist.get(0).unwrap().position, 1);
        });
    }

    #[test]
    fn test_promote_next_requires_owner() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        let applicant1 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();
            setup_contributor(&env, &applicant1, 500);
            join(&env, &applicant1, grant_id).unwrap();

            let result = promote_next(&env, &stranger, grant_id);
            assert_eq!(result, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn test_promote_next_stops_at_max_slots() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);
        let applicant3 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();
            setup_contributor(&env, &applicant1, 500);
            setup_contributor(&env, &applicant2, 500);
            setup_contributor(&env, &applicant3, 500);
            join(&env, &applicant1, grant_id).unwrap();
            join(&env, &applicant2, grant_id).unwrap();
            join(&env, &applicant3, grant_id).unwrap();

            promote_next(&env, &owner, grant_id).unwrap();
            promote_next(&env, &owner, grant_id).unwrap();

            let result = promote_next(&env, &owner, grant_id);
            assert_eq!(result, Err(ContractError::WaitlistFull));
        });
    }

    #[test]
    fn test_position_of() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);

        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();

            setup_contributor(&env, &applicant1, 500);
            setup_contributor(&env, &applicant2, 500);

            join(&env, &applicant1, grant_id).unwrap();
            join(&env, &applicant2, grant_id).unwrap();

            assert_eq!(position_of(&env, &applicant1, grant_id), Some(1));
            assert_eq!(position_of(&env, &applicant2, grant_id), Some(2));
        });
    }

    #[test]
    fn test_configure_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config.clone()).unwrap();
        });

        // Verify the event was emitted
        let events = env.events().all();
        assert!(!events.events().is_empty(), "At least one event should be emitted");
    }

    /// `require_auth()` fails by panicking (a host trap), not by returning
    /// an `Err`, so an unauthorized call must be caught with
    /// `catch_unwind` rather than matched on a `Result`.
    fn panics(f: impl FnOnce() + std::panic::UnwindSafe) -> bool {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(f);
        std::panic::set_hook(prev_hook);
        result.is_err()
    }

    #[test]
    fn test_configure_requires_owner_auth() {
        // A stranger cannot reconfigure another owner's waitlist merely by
        // passing the owner's address as a parameter: without mock_all_auths,
        // owner.require_auth() has nothing authorizing it and the call panics.
        let env = Env::default();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
        });

        assert!(panics(std::panic::AssertUnwindSafe(|| {
            env.as_contract(&contract_id, || {
                configure(&env, &owner, grant_id, config).unwrap();
            });
        })));
    }

    #[test]
    fn test_join_requires_applicant_auth() {
        // A third party cannot enroll an arbitrary address onto a waitlist:
        // join() requires the applicant's own authorization, which is absent
        // here since mock_all_auths was never called.
        let env = Env::default();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant = Address::generate(&env);
        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            Storage::set_waitlist_config(&env, grant_id, &config);
            setup_contributor(&env, &applicant, 500);
        });

        assert!(panics(std::panic::AssertUnwindSafe(|| {
            env.as_contract(&contract_id, || {
                join(&env, &applicant, grant_id).unwrap();
            });
        })));
    }

    #[test]
    fn test_leave_requires_applicant_auth() {
        // A third party cannot remove an arbitrary address from a waitlist:
        // leave() requires the applicant's own authorization. Seed the
        // waitlist entry directly via storage (bypassing join(), which would
        // itself need mocked auth) so the only auth check under test is
        // leave()'s own applicant.require_auth().
        let env = Env::default();
        let contract_id = register(&env);

        let applicant = Address::generate(&env);
        let grant_id = 1;

        env.as_contract(&contract_id, || {
            let mut entries = Vec::new(&env);
            entries.push_back(WaitlistEntry {
                applicant: applicant.clone(),
                grant_id,
                joined_at: 0,
                reputation_snapshot: 500,
                position: 1,
                promoted: false,
                promoted_at: None,
            });
            Storage::set_waitlist_entries(&env, grant_id, &entries);
        });

        assert!(panics(std::panic::AssertUnwindSafe(|| {
            env.as_contract(&contract_id, || {
                leave(&env, &applicant, grant_id).unwrap();
            });
        })));
    }

    #[test]
    fn test_join_waitlist_rate_limited() {
        // Repeated automated joins from the same applicant address across
        // many grants are throttled once RATE_LIMIT_WAITLIST_JOIN_MAX is hit
        // within the rate limit window, rather than succeeding indefinitely.
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant = Address::generate(&env);

        env.as_contract(&contract_id, || {
            setup_contributor(&env, &applicant, 500);
        });

        let max = crate::constants::RATE_LIMIT_WAITLIST_JOIN_MAX;

        for grant_id in 1..=max as u64 {
            env.as_contract(&contract_id, || {
                setup_grant(&env, grant_id, &owner);
                let config = WaitlistConfig {
                    grant_id,
                    max_slots: 2,
                    max_waitlist_size: 10,
                    rank_by_reputation: false,
                    auto_promote: true,
                };
                configure(&env, &owner, grant_id, config).unwrap();
                join(&env, &applicant, grant_id).unwrap();
            });
        }

        // One more join (a new grant) from the same applicant within the
        // same window must be throttled rather than silently succeeding.
        let extra_grant_id = max as u64 + 1;
        env.as_contract(&contract_id, || {
            setup_grant(&env, extra_grant_id, &owner);
            let config = WaitlistConfig {
                grant_id: extra_grant_id,
                max_slots: 2,
                max_waitlist_size: 10,
                rank_by_reputation: false,
                auto_promote: true,
            };
            configure(&env, &owner, extra_grant_id, config).unwrap();

            let result = join(&env, &applicant, extra_grant_id);
            assert_eq!(result, Err(ContractError::InvalidInput));
        });
    }

    #[test]
    fn test_join_waitlist_legitimate_infrequent_use_unaffected() {
        // A real applicant joining a single grant's waitlist once (well
        // under the rate limit) is unaffected by the throttling.
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = register(&env);

        let owner = Address::generate(&env);
        let applicant = Address::generate(&env);
        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: false,
            auto_promote: true,
        };

        env.as_contract(&contract_id, || {
            setup_grant(&env, grant_id, &owner);
            configure(&env, &owner, grant_id, config).unwrap();
            setup_contributor(&env, &applicant, 500);

            let position = join(&env, &applicant, grant_id).unwrap();
            assert_eq!(position, 1);
        });
    }
}
