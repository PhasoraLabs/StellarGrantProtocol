/// Reviewer SLA enforcement module (issue #611).
///
/// Tracks the deadline by which a reviewer must cast their vote.
/// If the deadline passes without a vote the reviewer is marked as
/// SLA-breached and loses reward accrual for that milestone.

use soroban_sdk::{contracttype, Address, Env, Symbol, symbol_short};

const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_000_000;

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReviewerSlaRecord {
    /// The reviewer whose SLA this tracks.
    pub reviewer: Address,
    /// Milestone ID this SLA applies to.
    pub milestone_id: u64,
    /// Ledger timestamp by which the reviewer must vote.
    pub deadline: u64,
    /// True once the reviewer has voted within the window.
    pub fulfilled: bool,
    /// True if the deadline passed without a vote.
    pub breached: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum SlaKey {
    /// Per-reviewer, per-milestone SLA record.
    ReviewerSla(Address, u64),
}

fn extend_ttl(env: &Env, key: &SlaKey) {
    if env.storage().persistent().has(key) {
        env.storage().persistent().extend_ttl(
            key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }
}

/// Register a reviewer SLA for `milestone_id` with the given `deadline`.
pub fn register_sla(env: &Env, reviewer: &Address, milestone_id: u64, deadline: u64) {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    let record = ReviewerSlaRecord {
        reviewer: reviewer.clone(),
        milestone_id,
        deadline,
        fulfilled: false,
        breached: false,
    };
    env.storage().persistent().set(&key, &record);
    extend_ttl(env, &key);

    env.events().publish(
        (symbol_short!("sla"), symbol_short!("reg"), milestone_id),
        (reviewer.clone(), deadline),
    );
}

/// Mark a reviewer's SLA as fulfilled (called when they cast their vote).
/// No-op if the SLA doesn't exist or is already resolved.
pub fn fulfill_sla(env: &Env, reviewer: &Address, milestone_id: u64) {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    extend_ttl(env, &key);
    let mut record: ReviewerSlaRecord = match env.storage().persistent().get(&key) {
        Some(r) => r,
        None => return,
    };
    if record.fulfilled || record.breached {
        return;
    }
    record.fulfilled = true;
    env.storage().persistent().set(&key, &record);
    extend_ttl(env, &key);
}

/// Check whether the SLA is breached (deadline passed, not fulfilled).
/// Persists the breached flag and emits an event on first detection.
pub fn check_and_mark_breach(env: &Env, reviewer: &Address, milestone_id: u64) -> bool {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    extend_ttl(env, &key);
    let mut record: ReviewerSlaRecord = match env.storage().persistent().get(&key) {
        Some(r) => r,
        None => return false,
    };
    if record.fulfilled || record.breached {
        return record.breached;
    }
    let now = env.ledger().timestamp();
    if now > record.deadline {
        record.breached = true;
        env.storage().persistent().set(&key, &record);
        extend_ttl(env, &key);
        env.events().publish(
            (symbol_short!("sla"), symbol_short!("breach"), milestone_id),
            reviewer.clone(),
        );
        return true;
    }
    false
}

/// Returns the SLA record for a reviewer / milestone pair, if it exists.
pub fn get_sla(env: &Env, reviewer: &Address, milestone_id: u64) -> Option<ReviewerSlaRecord> {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    extend_ttl(env, &key);
    env.storage().persistent().get(&key)
}
