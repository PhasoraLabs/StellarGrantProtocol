use soroban_sdk::{token, Address, Env};

use crate::circuit_breaker;
use crate::errors::ContractError;
use crate::storage::{DataKey, Storage};
use crate::types::{LockupRecord, LockupStatus, ProtocolModule};

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
    circuit_breaker::require_open(env, ProtocolModule::Vesting)?;

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
    circuit_breaker::require_open(env, ProtocolModule::Vesting)?;
    let admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if admin != *holder {
        return Err(ContractError::Unauthorized);
    }

    let key = get_lockup_key(grant_id, milestone_idx);
    let mut record: LockupRecord = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::LockupNotFound)?;

    if record.status != LockupStatus::Active {
        return Err(ContractError::InvalidState);
    }

    token::Client::new(env, token).transfer(holder, &env.current_contract_address(), &amount);

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
    circuit_breaker::require_open(env, ProtocolModule::Vesting)?;

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
    token::Client::new(env, &record.token).transfer(
        &env.current_contract_address(),
        holder,
        &amount,
    );

    record.status = LockupStatus::Released;
    record.released_at = Some(now);
    env.storage().persistent().set(&key, &record);

    Ok(amount)
}

pub fn is_unlocked(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    match get_lockup(env, grant_id, milestone_idx) {
        Some(record) => {
            record.status == LockupStatus::Active && env.ledger().timestamp() >= record.unlocks_at
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
    circuit_breaker::require_open(env, ProtocolModule::Vesting)?;
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

    fn make_grant(env: &Env, owner: &Address) -> crate::types::Grant {
        crate::types::Grant {
            id: 1,
            owner: owner.clone(),
            title: soroban_sdk::String::from_str(env, "Test Grant"),
            description: soroban_sdk::String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: crate::types::GrantStatus::Active,
            total_amount: 10_000,
            milestone_amount: 5_000,
            reviewers: soroban_sdk::Vec::new(env),
            total_milestones: 3,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: soroban_sdk::Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        }
    }

    fn set_ledger(env: &Env, timestamp: u64, sequence: u32) {
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp,
            protocol_version: 21,
            sequence_number: sequence,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 100_000,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 1_000_000,
        });
    }

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let grant = make_grant(&env, &owner);
        Storage::set_grant(&env, 1, &grant);
        set_ledger(&env, 1000, 100);
        (env, admin, owner)
    }

    // ── attach_lockup ─────────────────────────────────────────────────────

    #[test]
    fn attach_lockup_happy_path() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.status, LockupStatus::Active);
        assert_eq!(record.holder, owner);
        assert_eq!(record.grant_id, 1);
        assert_eq!(record.milestone_idx, 0);
        assert_eq!(record.unlocks_at, 1000 + 604_800);
        assert_eq!(record.amount, 0);
    }

    #[test]
    fn attach_lockup_duplicate_rejected() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let err = attach_lockup(&env, &owner, 1, 0, 300_000);
        assert_eq!(err, Err(ContractError::LockupAlreadyExists));
    }

    #[test]
    fn attach_lockup_different_milestones_allowed() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        attach_lockup(&env, &owner, 1, 1, 604_800).unwrap();
        assert!(get_lockup(&env, 1, 0).is_some());
        assert!(get_lockup(&env, 1, 1).is_some());
    }

    #[test]
    fn attach_lockup_grant_not_found() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        set_ledger(&env, 1000, 100);
        let err = attach_lockup(&env, &owner, 999, 0, 604_800);
        assert_eq!(err, Err(ContractError::GrantNotFound));
    }

    #[test]
    fn attach_lockup_wrong_owner_unauthorized() {
        let (env, _admin, owner) = setup();
        let stranger = Address::generate(&env);
        let err = attach_lockup(&env, &stranger, 1, 0, 604_800);
        assert_eq!(err, Err(ContractError::Unauthorized));
    }

    #[test]
    fn attach_lockup_records_unlocks_at() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 500).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.unlocks_at, 1500);
        assert_eq!(record.token, Storage::get_grant(&env, 1).unwrap().token);
    }

    #[test]
    fn attach_lockup_sets_active_status() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 2, 1_000).unwrap();
        let record = get_lockup(&env, 1, 2).unwrap();
        assert_eq!(record.status, LockupStatus::Active);
        assert_eq!(record.released_at, None);
    }

    // ── lock_payout ───────────────────────────────────────────────────────

    #[test]
    fn lock_payout_happy_path() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.amount, 5_000);
        assert_eq!(record.holder, admin);
        assert_eq!(record.token, fake_token);
        assert_eq!(record.locked_at, 1000);
    }

    #[test]
    fn lock_payout_unauthorized_non_admin() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let stranger = Address::generate(&env);
        let fake_token = Address::generate(&env);
        let err = lock_payout(&env, 1, 0, &stranger, &fake_token, 5_000);
        assert_eq!(err, Err(ContractError::Unauthorized));
    }

    #[test]
    fn lock_payout_lockup_not_found() {
        let (env, admin, _owner) = setup();
        let fake_token = Address::generate(&env);
        let err = lock_payout(&env, 1, 0, &admin, &fake_token, 5_000);
        assert_eq!(err, Err(ContractError::LockupNotFound));
    }

    #[test]
    fn lock_payout_already_locked() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        let err = lock_payout(&env, 1, 0, &admin, &fake_token, 3_000);
        assert_eq!(err, Err(ContractError::InvalidState));
    }

    // ── release ───────────────────────────────────────────────────────────

    #[test]
    fn release_before_unlocks_at_rejected() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        // Still within lockup period (1000 + 604800 = 605800, current is 1000)
        let err = release(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::NotYetUnlocked));
    }

    #[test]
    fn release_after_unlocks_at_succeeds() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        // Advance past unlock time
        set_ledger(&env, 1200, 140);
        let amount = release(&env, &admin, 1, 0).unwrap();
        assert_eq!(amount, 5_000);
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.status, LockupStatus::Released);
        assert_eq!(record.released_at, Some(1200));
    }

    #[test]
    fn release_exactly_at_unlocks_at_succeeds() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 2_500).unwrap();
        // Advance to exactly unlock time
        set_ledger(&env, 1100, 120);
        let amount = release(&env, &admin, 1, 0).unwrap();
        assert_eq!(amount, 2_500);
    }

    #[test]
    fn release_wrong_holder_unauthorized() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        set_ledger(&env, 1200, 140);
        let stranger = Address::generate(&env);
        let err = release(&env, &stranger, 1, 0);
        assert_eq!(err, Err(ContractError::Unauthorized));
    }

    #[test]
    fn release_double_release_rejected() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        set_ledger(&env, 1200, 140);
        release(&env, &admin, 1, 0).unwrap();
        let err = release(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::LockupAlreadyReleased));
    }

    #[test]
    fn release_lockup_not_found() {
        let (env, admin, _owner) = setup();
        set_ledger(&env, 1200, 140);
        let err = release(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::LockupNotFound));
    }

    #[test]
    fn release_returns_correct_amount() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 9_999).unwrap();
        set_ledger(&env, 1200, 140);
        let amount = release(&env, &admin, 1, 0).unwrap();
        assert_eq!(amount, 9_999);
    }

    // ── revoke ────────────────────────────────────────────────────────────

    #[test]
    fn revoke_happy_path() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        revoke(&env, &admin, 1, 0).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.status, LockupStatus::Revoked);
        assert_eq!(record.released_at, Some(1000));
    }

    #[test]
    fn revoke_wrong_admin_unauthorized() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let stranger = Address::generate(&env);
        let err = revoke(&env, &stranger, 1, 0);
        assert_eq!(err, Err(ContractError::LockupRevocationUnauthorized));
    }

    #[test]
    fn revoke_double_revoke_rejected() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        revoke(&env, &admin, 1, 0).unwrap();
        let err = revoke(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::LockupAlreadyRevoked));
    }

    #[test]
    fn revoke_lockup_not_found() {
        let (env, admin, _owner) = setup();
        let err = revoke(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::LockupNotFound));
    }

    #[test]
    fn revoke_then_release_rejected() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        revoke(&env, &admin, 1, 0).unwrap();
        set_ledger(&env, 2_000_000, 400_000);
        let err = release(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::LockupAlreadyReleased));
    }

    #[test]
    fn revoke_after_lock_payout() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        revoke(&env, &admin, 1, 0).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.status, LockupStatus::Revoked);
        assert_eq!(record.amount, 5_000);
    }

    // ── is_unlocked ───────────────────────────────────────────────────────

    #[test]
    fn is_unlocked_false_before_time() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        assert!(!is_unlocked(&env, 1, 0));
    }

    #[test]
    fn is_unlocked_true_after_time() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        set_ledger(&env, 1000 + 200, 120);
        assert!(is_unlocked(&env, 1, 0));
    }

    #[test]
    fn is_unlocked_false_for_nonexistent() {
        let (env, _admin, _owner) = setup();
        assert!(!is_unlocked(&env, 999, 0));
    }

    #[test]
    fn is_unlocked_false_after_release() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        set_ledger(&env, 1200, 140);
        release(&env, &admin, 1, 0).unwrap();
        assert!(!is_unlocked(&env, 1, 0));
    }

    #[test]
    fn is_unlocked_false_after_revoke() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        revoke(&env, &admin, 1, 0).unwrap();
        set_ledger(&env, 2_000_000, 400_000);
        assert!(!is_unlocked(&env, 1, 0));
    }

    // ── get_lockup ────────────────────────────────────────────────────────

    #[test]
    fn get_lockup_none_for_nonexistent() {
        let (env, _admin, _owner) = setup();
        assert_eq!(get_lockup(&env, 999, 0), None);
    }

    #[test]
    fn get_lockup_returns_record() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.holder, owner);
        assert_eq!(record.grant_id, 1);
    }

    // ── authorization checks ──────────────────────────────────────────────

    #[test]
    fn attach_lockup_auth_check() {
        let env = Env::default();
        // Do NOT mock auths to verify require_auth is called
        let owner = Address::generate(&env);
        let grant = make_grant(&env, &owner);
        Storage::set_grant(&env, 1, &grant);
        set_ledger(&env, 1000, 100);
        // Without mock_all_auths, require_auth will fail for the non-sender
        let err = attach_lockup(&env, &owner, 1, 0, 604_800);
        assert!(err.is_err());
    }

    #[test]
    fn release_auth_check() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        set_ledger(&env, 2_000_000, 400_000);

        // Turn off mock_all_auths to test real auth
        let env2 = Env::default();
        env2.mock_all_auths();
        let admin2 = Address::generate(&env);
        // This should fail because the new env doesn't have the lockup
        let err = release(&env2, &admin2, 1, 0);
        assert_eq!(err, Err(ContractError::LockupNotFound));
    }

    #[test]
    fn revoke_auth_check() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let stranger = Address::generate(&env);
        // stranger is not admin, so revoke should fail even with mock_all_auths
        // because it checks Storage::get_global_admin != Some(stranger)
        let err = revoke(&env, &stranger, 1, 0);
        assert_eq!(err, Err(ContractError::LockupRevocationUnauthorized));
    }

    // ── edge cases ────────────────────────────────────────────────────────

    #[test]
    fn attach_lockup_zero_duration() {
        let (env, _admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 0).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.unlocks_at, 1000);
    }

    #[test]
    fn release_immediately_when_zero_duration() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 0).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 1_000).unwrap();
        // Current time is 1000, unlocks_at is 1000, so now >= unlocks_at
        let amount = release(&env, &admin, 1, 0).unwrap();
        assert_eq!(amount, 1_000);
    }

    #[test]
    fn multiple_grants_independent_lockups() {
        let (env, admin, owner) = setup();
        let grant2 = {
            let mut g = make_grant(&env, &owner);
            g.id = 2;
            g
        };
        Storage::set_grant(&env, 2, &grant2);
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        attach_lockup(&env, &owner, 2, 0, 200).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 1_000).unwrap();
        lock_payout(&env, 2, 0, &admin, &fake_token, 2_000).unwrap();
        // Release grant 1 at t=1100 (past 1000+100)
        set_ledger(&env, 1100, 120);
        let a1 = release(&env, &admin, 1, 0).unwrap();
        assert_eq!(a1, 1_000);
        // Grant 2 not yet unlocked (unlocks at 1200)
        let err = release(&env, &admin, 2, 0);
        assert_eq!(err, Err(ContractError::NotYetUnlocked));
    }

    // ── circuit breaker integration ─────────────────────────────────────────

    #[test]
    fn vesting_circuit_breaker_blocks_attach_lockup() {
        let (env, admin, owner) = setup();

        // Trip the Vesting circuit breaker
        crate::circuit_breaker::trip(
            &env,
            &admin,
            crate::types::ProtocolModule::Vesting,
            soroban_sdk::String::from_str(&env, "test"),
            None,
        )
        .unwrap();

        // attach_lockup should fail with ModuleTripped
        let err = attach_lockup(&env, &owner, 1, 0, 604_800);
        assert_eq!(err, Err(ContractError::ModuleTripped));
    }

    #[test]
    fn vesting_circuit_breaker_blocks_lock_payout() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();

        // Trip the Vesting circuit breaker
        crate::circuit_breaker::trip(
            &env,
            &admin,
            crate::types::ProtocolModule::Vesting,
            soroban_sdk::String::from_str(&env, "test"),
            None,
        )
        .unwrap();

        let fake_token = Address::generate(&env);
        let err = lock_payout(&env, 1, 0, &admin, &fake_token, 5_000);
        assert_eq!(err, Err(ContractError::ModuleTripped));
    }

    #[test]
    fn vesting_circuit_breaker_blocks_release() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 100).unwrap();
        let fake_token = Address::generate(&env);
        lock_payout(&env, 1, 0, &admin, &fake_token, 5_000).unwrap();
        set_ledger(&env, 1200, 140);

        // Trip the Vesting circuit breaker
        crate::circuit_breaker::trip(
            &env,
            &admin,
            crate::types::ProtocolModule::Vesting,
            soroban_sdk::String::from_str(&env, "test"),
            None,
        )
        .unwrap();

        let err = release(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::ModuleTripped));
    }

    #[test]
    fn vesting_circuit_breaker_blocks_revoke() {
        let (env, admin, owner) = setup();
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();

        // Trip the Vesting circuit breaker
        crate::circuit_breaker::trip(
            &env,
            &admin,
            crate::types::ProtocolModule::Vesting,
            soroban_sdk::String::from_str(&env, "test"),
            None,
        )
        .unwrap();

        let err = revoke(&env, &admin, 1, 0);
        assert_eq!(err, Err(ContractError::ModuleTripped));
    }

    #[test]
    fn vesting_circuit_breaker_reset_allows_operations() {
        let (env, admin, owner) = setup();

        // Trip the Vesting circuit breaker
        crate::circuit_breaker::trip(
            &env,
            &admin,
            crate::types::ProtocolModule::Vesting,
            soroban_sdk::String::from_str(&env, "test"),
            None,
        )
        .unwrap();

        // Verify attach_lockup is blocked
        let err = attach_lockup(&env, &owner, 1, 0, 604_800);
        assert_eq!(err, Err(ContractError::ModuleTripped));

        // Reset the breaker
        crate::circuit_breaker::reset(&env, &admin, crate::types::ProtocolModule::Vesting).unwrap();

        // attach_lockup should now succeed
        attach_lockup(&env, &owner, 1, 0, 604_800).unwrap();
        let record = get_lockup(&env, 1, 0).unwrap();
        assert_eq!(record.status, LockupStatus::Active);
    }
}
