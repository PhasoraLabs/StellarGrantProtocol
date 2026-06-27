use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::storage::{DataKey, Storage};
use crate::types::{LockupRecord, LockupStatus};

fn get_lockup_key(grant_id: u64, milestone_idx: u32) -> DataKey {
    DataKey::Lockup(grant_id, milestone_idx)
}

pub fn get_lockup(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<LockupRecord> {
    env.storage()
        .persistent()
        .get(&get_lockup_key(grant_id, milestone_idx))
}

fn save_lockup(env: &Env, record: &LockupRecord) {
    env.storage().persistent().set(
        &get_lockup_key(record.grant_id, record.milestone_idx),
        record,
    );
}

pub fn attach_lockup(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    milestone_idx: u32,
    lockup_duration_seconds: u64,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    if get_lockup(env, grant_id, milestone_idx).is_some() {
        return Err(ContractError::LockupAlreadyExists);
    }

    let now = env.ledger().timestamp();
    let unlocks_at = now
        .checked_add(lockup_duration_seconds)
        .ok_or(ContractError::InvalidInput)?;

    let record = LockupRecord {
        grant_id,
        milestone_idx,
        holder: owner.clone(),
        token: grant.token.clone(),
        amount: 0,
        unlocks_at,
        unlocks_at_ledger: 0,
        status: LockupStatus::Active,
        locked_at: 0,
        released_at: None,
    };

    save_lockup(env, &record);
    Ok(())
}

pub fn lock_payout(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    holder: &Address,
    token: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    let key = get_lockup_key(grant_id, milestone_idx);
    let mut record: LockupRecord = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::LockupNotFound)?;

    if record.status != LockupStatus::Active {
        return Err(ContractError::InvalidState);
    }

    let now = env.ledger().timestamp();
    record.holder = holder.clone();
    record.token = token.clone();
    record.amount = amount;
    record.locked_at = now;

    let ledger_sequence = env.ledger().sequence();
    record.unlocks_at_ledger = ledger_sequence
        .checked_add((record.unlocks_at - now) as u32 / 5)
        .unwrap_or(u32::MAX);

    env.storage().persistent().set(&key, &record);
    Ok(())
}

pub fn release(
    env: &Env,
    holder: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<i128, ContractError> {
    holder.require_auth();

    let key = get_lockup_key(grant_id, milestone_idx);
    let mut record: LockupRecord = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::LockupNotFound)?;

    if record.status != LockupStatus::Active {
        return Err(ContractError::LockupAlreadyReleased);
    }

    if record.holder != *holder {
        return Err(ContractError::Unauthorized);
    }

    let now = env.ledger().timestamp();
    if now < record.unlocks_at {
        return Err(ContractError::NotYetUnlocked);
    }

    let amount = record.amount;
    record.status = LockupStatus::Released;
    record.released_at = Some(now);
    env.storage().persistent().set(&key, &record);

    Ok(amount)
}

pub fn is_unlocked(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    match get_lockup(env, grant_id, milestone_idx) {
        Some(record) => {
            record.status == LockupStatus::Active
                && env.ledger().timestamp() >= record.unlocks_at
        }
        None => false,
    }
}

pub fn revoke(
    env: &Env,
    admin: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    admin.require_auth();

    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::LockupRevocationUnauthorized);
    }

    let key = get_lockup_key(grant_id, milestone_idx);
    let mut record: LockupRecord = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::LockupNotFound)?;

    if record.status != LockupStatus::Active {
        return Err(ContractError::LockupAlreadyRevoked);
    }

    record.status = LockupStatus::Revoked;
    record.released_at = Some(env.ledger().timestamp());
    env.storage().persistent().set(&key, &record);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env};

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let contributor = Address::generate(&env);
        (env, admin, owner, contributor)
    }

    #[test]
    fn test_attach_lockup() {
        let (env, _admin, owner, _contributor) = setup();
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence: 100,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        let grant_id = 1u64;
        let milestone_idx = 0u32;

        let result = attach_lockup(&env, &owner, grant_id, milestone_idx, 604800);
        assert!(result.is_ok());

        let record = get_lockup(&env, grant_id, milestone_idx).unwrap();
        assert_eq!(record.status, LockupStatus::Active);
        assert_eq!(record.unlocks_at, 1000 + 604800);
    }

    #[test]
    fn test_attach_lockup_already_exists() {
        let (env, _admin, owner, _contributor) = setup();
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence: 100,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        let result = attach_lockup(&env, &owner, 1, 0, 604800);
        assert!(result.is_ok());

        let result2 = attach_lockup(&env, &owner, 1, 0, 604800);
        assert_eq!(result2, Err(ContractError::LockupAlreadyExists));
    }

    #[test]
    fn test_release_before_expiry() {
        let (env, _admin, owner, contributor) = setup();
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence: 100,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        attach_lockup(&env, &owner, 1, 0, 604800).unwrap();

        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &contributor, &fake_token, 500).unwrap();

        let result = release(&env, &contributor, 1, 0);
        assert_eq!(result, Err(ContractError::NotYetUnlocked));
    }

    #[test]
    fn test_release_after_expiry() {
        let (env, _admin, owner, contributor) = setup();
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence: 100,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        attach_lockup(&env, &owner, 1, 0, 100).unwrap();

        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &contributor, &fake_token, 500).unwrap();

        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1200,
            protocol_version: 21,
            sequence: 140,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        let amount = release(&env, &contributor, 1, 0).unwrap();
        assert_eq!(amount, 500);

        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.status, LockupStatus::Released);
    }

    #[test]
    fn test_revoke() {
        let (env, admin, owner, _contributor) = setup();
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence: 100,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        let grant_id = 1u64;
        attach_lockup(&env, &owner, grant_id, 0, 604800).unwrap();

        revoke(&env, &admin, grant_id, 0).unwrap();

        let record = get_lockup(&env, grant_id, 0).unwrap();
        assert_eq!(record.status, LockupStatus::Revoked);
    }

    #[test]
    fn test_is_unlocked() {
        let (env, _admin, owner, _contributor) = setup();
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence: 100,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        attach_lockup(&env, &owner, 1, 0, 100).unwrap();

        assert!(!is_unlocked(&env, 1, 0));

        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 1100,
            protocol_version: 21,
            sequence: 120,
            base_reserve: 10,
            network_passphrase: Default::default(),
        });

        assert!(is_unlocked(&env, 1, 0));
    }
}
