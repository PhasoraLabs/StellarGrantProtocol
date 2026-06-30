use soroban_sdk::{Address, Env, String, Vec};
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, WaitlistConfig, WaitlistEntry};

/// Configure the waitlist for a grant. Owner only.
pub fn configure(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    config: WaitlistConfig,
) -> Result<(), ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    Storage::set_waitlist_config(env, grant_id, &config);
    Ok(())
}

/// Join the waitlist for a grant. Returns the position (1-indexed).
pub fn join(env: &Env, applicant: &Address, grant_id: u64) -> Result<u32, ContractError> {
    let config = Storage::get_waitlist_config(env, grant_id)
        .ok_or(ContractError::InvalidInput)?;

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
    let profile = Storage::get_contributor(env, applicant)
        .ok_or(ContractError::ContributorNotFound)?;
    let reputation_snapshot = profile.reputation_score as u32;

    let joined_at = env.ledger().timestamp();
    let position;

    if config.rank_by_reputation {
        // Insert in sorted order by reputation (highest first)
        let mut insert_idx = entries.len();
        for (idx, entry) in entries.iter().enumerate() {
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
            position: (insert_idx + 1) as u32,
            promoted: false,
            promoted_at: None,
        };

        entries.insert(insert_idx as u32, new_entry);
        position = (insert_idx + 1) as u32;

        // Re-index all entries after insertion point
        for idx in (insert_idx + 1)..entries.len() {
            entries[idx].position = (idx + 1) as u32;
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
    let mut entries = Storage::get_waitlist_entries(env, grant_id);

    let mut found_idx = None;
    for (idx, entry) in entries.iter().enumerate() {
        if entry.applicant == *applicant {
            found_idx = Some(idx);
            break;
        }
    }

    let idx = found_idx.ok_or(ContractError::NotOnWaitlist)?;

    // Remove the entry
    entries.remove(idx as u32);

    // Re-index remaining entries
    for i in idx..entries.len() {
        entries[i].position = (i + 1) as u32;
    }

    Storage::set_waitlist_entries(env, grant_id, &entries);
    Events::emit_waitlist_left(env, grant_id, applicant.clone());

    Ok(())
}

/// Promote the top-ranked entry. Called when a slot opens.
/// Returns the promoted address if successful, None if waitlist is empty.
pub fn promote_next(env: &Env, grant_id: u64) -> Option<Address> {
    let config = Storage::get_waitlist_config(env, grant_id)?;
    if !config.auto_promote {
        return None;
    }

    let mut entries = Storage::get_waitlist_entries(env, grant_id);
    
    if entries.is_empty() {
        return None;
    }

    // Get the first (highest-ranked) entry
    let promoted_entry = entries.get(0)?.clone();

    // Mark as promoted
    entries[0].promoted = true;
    entries[0].promoted_at = Some(env.ledger().timestamp());

    // Remove from waitlist
    entries.remove(0);

    // Re-index remaining entries
    for i in 0..entries.len() {
        entries[i].position = (i + 1) as u32;
    }

    Storage::set_waitlist_entries(env, grant_id, &entries);
    Events::emit_waitlist_promoted(env, grant_id, promoted_entry.applicant.clone(), 1);

    Some(promoted_entry.applicant)
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
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::{Address, String};

    #[test]
    fn test_join_waitlist_reputation_ranked() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let applicant1 = Address::generate(&env);
        let applicant2 = Address::generate(&env);
        let applicant3 = Address::generate(&env);

        // Setup grant
        let grant_id = 1;
        let config = WaitlistConfig {
            grant_id,
            max_slots: 2,
            max_waitlist_size: 10,
            rank_by_reputation: true,
            auto_promote: true,
        };

        // Configure waitlist
        configure(&env, &owner, grant_id, config.clone()).unwrap();

        // Create contributor profiles with different reputations
        let mut profile1 = crate::types::ContributorProfile {
            contributor: applicant1.clone(),
            name: String::from_str(&env, "Applicant1"),
            reputation_score: 500,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant1.clone(), &mut profile1);

        let mut profile2 = crate::types::ContributorProfile {
            contributor: applicant2.clone(),
            name: String::from_str(&env, "Applicant2"),
            reputation_score: 800,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant2.clone(), &mut profile2);

        let mut profile3 = crate::types::ContributorProfile {
            contributor: applicant3.clone(),
            name: String::from_str(&env, "Applicant3"),
            reputation_score: 600,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant3.clone(), &mut profile3);

        // Join in order: applicant1 (500), applicant2 (800), applicant3 (600)
        join(&env, &applicant1, grant_id).unwrap();
        join(&env, &applicant2, grant_id).unwrap();
        join(&env, &applicant3, grant_id).unwrap();

        let waitlist = get_waitlist(&env, grant_id);
        assert_eq!(waitlist.len(), 3);

        // Verify order: highest reputation first (800, 600, 500)
        assert_eq!(waitlist[0].applicant, applicant2);
        assert_eq!(waitlist[0].position, 1);
        assert_eq!(waitlist[1].applicant, applicant3);
        assert_eq!(waitlist[1].position, 2);
        assert_eq!(waitlist[2].applicant, applicant1);
        assert_eq!(waitlist[2].position, 3);
    }

    #[test]
    fn test_join_waitlist_fifo() {
        let env = Env::default();
        env.mock_all_auths();

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

        configure(&env, &owner, grant_id, config.clone()).unwrap();

        let mut profile = crate::types::ContributorProfile {
            contributor: applicant1.clone(),
            name: String::from_str(&env, "Applicant1"),
            reputation_score: 500,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant1.clone(), &mut profile);

        profile.contributor = applicant2.clone();
        Storage::set_contributor(&env, applicant2.clone(), &mut profile);

        profile.contributor = applicant3.clone();
        Storage::set_contributor(&env, applicant3.clone(), &mut profile);

        // Join in order
        join(&env, &applicant1, grant_id).unwrap();
        join(&env, &applicant2, grant_id).unwrap();
        join(&env, &applicant3, grant_id).unwrap();

        let waitlist = get_waitlist(&env, grant_id);
        assert_eq!(waitlist.len(), 3);

        // Verify FIFO order
        assert_eq!(waitlist[0].applicant, applicant1);
        assert_eq!(waitlist[1].applicant, applicant2);
        assert_eq!(waitlist[2].applicant, applicant3);
    }

    #[test]
    fn test_leave_waitlist() {
        let env = Env::default();
        env.mock_all_auths();

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

        configure(&env, &owner, grant_id, config.clone()).unwrap();

        let mut profile = crate::types::ContributorProfile {
            contributor: applicant1.clone(),
            name: String::from_str(&env, "Applicant1"),
            reputation_score: 500,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant1.clone(), &mut profile);

        profile.contributor = applicant2.clone();
        Storage::set_contributor(&env, applicant2.clone(), &mut profile);

        profile.contributor = applicant3.clone();
        Storage::set_contributor(&env, applicant3.clone(), &mut profile);

        join(&env, &applicant1, grant_id).unwrap();
        join(&env, &applicant2, grant_id).unwrap();
        join(&env, &applicant3, grant_id).unwrap();

        // Leave from middle
        leave(&env, &applicant2, grant_id).unwrap();

        let waitlist = get_waitlist(&env, grant_id);
        assert_eq!(waitlist.len(), 2);
        assert_eq!(waitlist[0].applicant, applicant1);
        assert_eq!(waitlist[0].position, 1);
        assert_eq!(waitlist[1].applicant, applicant3);
        assert_eq!(waitlist[1].position, 2);
    }

    #[test]
    fn test_waitlist_full() {
        let env = Env::default();
        env.mock_all_auths();

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

        configure(&env, &owner, grant_id, config.clone()).unwrap();

        let mut profile = crate::types::ContributorProfile {
            contributor: applicant1.clone(),
            name: String::from_str(&env, "Applicant1"),
            reputation_score: 500,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant1.clone(), &mut profile);

        profile.contributor = applicant2.clone();
        Storage::set_contributor(&env, applicant2.clone(), &mut profile);

        profile.contributor = applicant3.clone();
        Storage::set_contributor(&env, applicant3.clone(), &mut profile);

        join(&env, &applicant1, grant_id).unwrap();
        join(&env, &applicant2, grant_id).unwrap();

        // Third applicant should fail
        let result = join(&env, &applicant3, grant_id);
        assert_eq!(result, Err(ContractError::WaitlistFull));
    }

    #[test]
    fn test_promote_next() {
        let env = Env::default();
        env.mock_all_auths();

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

        configure(&env, &owner, grant_id, config.clone()).unwrap();

        let mut profile = crate::types::ContributorProfile {
            contributor: applicant1.clone(),
            name: String::from_str(&env, "Applicant1"),
            reputation_score: 500,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant1.clone(), &mut profile);

        profile.contributor = applicant2.clone();
        Storage::set_contributor(&env, applicant2.clone(), &mut profile);

        join(&env, &applicant1, grant_id).unwrap();
        join(&env, &applicant2, grant_id).unwrap();

        // Promote first
        let promoted = promote_next(&env, grant_id);
        assert_eq!(promoted, Some(applicant1.clone()));

        let waitlist = get_waitlist(&env, grant_id);
        assert_eq!(waitlist.len(), 1);
        assert_eq!(waitlist[0].applicant, applicant2);
        assert_eq!(waitlist[0].position, 1);
    }

    #[test]
    fn test_position_of() {
        let env = Env::default();
        env.mock_all_auths();

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

        configure(&env, &owner, grant_id, config.clone()).unwrap();

        let mut profile = crate::types::ContributorProfile {
            contributor: applicant1.clone(),
            name: String::from_str(&env, "Applicant1"),
            reputation_score: 500,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            registered_at: 0,
            metadata: Vec::new(&env),
        };
        Storage::set_contributor(&env, applicant1.clone(), &mut profile);

        profile.contributor = applicant2.clone();
        Storage::set_contributor(&env, applicant2.clone(), &mut profile);

        join(&env, &applicant1, grant_id).unwrap();
        join(&env, &applicant2, grant_id).unwrap();

        assert_eq!(position_of(&env, &applicant1, grant_id), Some(1));
        assert_eq!(position_of(&env, &applicant2, grant_id), Some(2));

        let other = Address::generate(&env);
        assert_eq!(position_of(&env, &other, grant_id), None);
    }
}
