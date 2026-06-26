use soroban_sdk::{contractevent, token, Address, Env, Vec};

use crate::constants::BASIS_POINTS_SCALE;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{Arbiter, ArbiterVote, ArbitrationCase};

/// Voting window for an arbitration case, in seconds (~1 day).
const ARBITRATION_VOTING_WINDOW: u64 = 86_400;
/// Portion of a minority arbiter's stake that is slashed and redistributed.
const ARBITER_SLASH_BPS: u32 = 1_000; // 10%

// ── Events ──────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbiterJoined {
    pub arbiter: Address,
    pub stake: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbiterLeft {
    pub arbiter: Address,
    pub returned: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelAssigned {
    pub case_id: u32,
    pub dispute_id: u32,
    pub panel_size: u32,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbiterVoteCast {
    pub case_id: u32,
    pub arbiter: Address,
    pub favor_contributor: bool,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseFinalized {
    pub case_id: u32,
    pub outcome: bool,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewardsSettled {
    pub case_id: u32,
    pub total_slashed: i128,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Join the arbitration pool by staking tokens.
pub fn join_pool(
    env: &Env,
    arbiter: &Address,
    token: &Address,
    stake: i128,
) -> Result<(), ContractError> {
    arbiter.require_auth();

    if stake <= 0 {
        return Err(ContractError::InvalidInput);
    }

    // The pool operates on a single staking token, fixed by the first joiner.
    match Storage::get_arbiter_pool_token(env) {
        Some(existing) => {
            if existing != *token {
                return Err(ContractError::InvalidInput);
            }
        }
        None => Storage::set_arbiter_pool_token(env, token),
    }

    if let Some(existing) = Storage::get_arbiter(env, arbiter) {
        if existing.is_active {
            return Err(ContractError::ArbiterAlreadyJoined);
        }
    }

    token::Client::new(env, token).transfer(arbiter, &env.current_contract_address(), &stake);

    let now = env.ledger().timestamp();
    let record = match Storage::get_arbiter(env, arbiter) {
        Some(mut a) => {
            a.stake = a
                .stake
                .checked_add(stake)
                .ok_or(ContractError::InvalidInput)?;
            a.is_active = true;
            a
        }
        None => Arbiter {
            address: arbiter.clone(),
            stake,
            cases_decided: 0,
            cases_correct: 0,
            is_active: true,
            joined_at: now,
        },
    };
    Storage::set_arbiter(env, &record);

    let mut pool = Storage::get_arbiter_pool(env);
    if !pool.contains(arbiter.clone()) {
        pool.push_back(arbiter.clone());
        Storage::set_arbiter_pool(env, &pool);
    }

    ArbiterJoined {
        arbiter: arbiter.clone(),
        stake,
    }
    .publish(env);

    Ok(())
}

/// Leave the pool and withdraw stake (only when not in an active case).
pub fn leave_pool(env: &Env, arbiter: &Address) -> Result<i128, ContractError> {
    arbiter.require_auth();

    let mut record = Storage::get_arbiter(env, arbiter).ok_or(ContractError::ArbiterNotFound)?;
    if !record.is_active {
        return Err(ContractError::ArbiterNotFound);
    }

    if Storage::get_arbiter_active_cases(env, arbiter) > 0 {
        return Err(ContractError::ArbiterInActiveCase);
    }

    let refund = record.stake;
    record.stake = 0;
    record.is_active = false;
    Storage::set_arbiter(env, &record);

    // Remove from active pool list.
    let pool = Storage::get_arbiter_pool(env);
    let mut new_pool = Vec::new(env);
    for a in pool.iter() {
        if a != *arbiter {
            new_pool.push_back(a);
        }
    }
    Storage::set_arbiter_pool(env, &new_pool);

    if refund > 0 {
        let token = Storage::get_arbiter_pool_token(env).ok_or(ContractError::InvalidState)?;
        token::Client::new(env, &token).transfer(&env.current_contract_address(), arbiter, &refund);
    }

    ArbiterLeft {
        arbiter: arbiter.clone(),
        returned: refund,
    }
    .publish(env);

    Ok(refund)
}

/// Randomly assign a panel of `panel_size` arbiters for a dispute.
/// Uses `env.prng().shuffle` over the active arbiter list.
pub fn assign_panel(env: &Env, dispute_id: u32, panel_size: u32) -> Result<u32, ContractError> {
    if panel_size != 3 && panel_size != 5 {
        return Err(ContractError::InvalidInput);
    }

    let mut pool = Storage::get_arbiter_pool(env);
    if pool.len() < panel_size {
        return Err(ContractError::InsufficientArbiters);
    }

    // Fisher-Yates shuffle via ledger PRNG, then take the first `panel_size`.
    env.prng().shuffle(&mut pool);

    let mut panel = Vec::new(env);
    for i in 0..panel_size {
        let addr = pool.get(i).ok_or(ContractError::InsufficientArbiters)?;
        panel.push_back(addr.clone());
        // Lock each selected arbiter against leaving while the case is active.
        let count = Storage::get_arbiter_active_cases(env, &addr);
        Storage::set_arbiter_active_cases(env, &addr, count.saturating_add(1));
    }

    let case_id = Storage::next_arbitration_case_id(env);
    let now = env.ledger().timestamp();
    let case = ArbitrationCase {
        id: case_id,
        dispute_id,
        panel,
        votes: Vec::new(env),
        outcome: None,
        finalized: false,
        assigned_at: now,
        deadline: now + ARBITRATION_VOTING_WINDOW,
    };
    Storage::set_arbitration_case(env, &case);
    Storage::set_case_id_by_dispute(env, dispute_id, case_id);

    PanelAssigned {
        case_id,
        dispute_id,
        panel_size,
    }
    .publish(env);

    Ok(case_id)
}

/// Arbiter casts their vote on an arbitration case.
pub fn cast_arbiter_vote(
    env: &Env,
    arbiter: &Address,
    case_id: u32,
    favor_contributor: bool,
    confidence: u32,
) -> Result<(), ContractError> {
    arbiter.require_auth();

    if confidence < 1 || confidence > 100 {
        return Err(ContractError::InvalidInput);
    }

    let mut case = Storage::get_arbitration_case(env, case_id)
        .ok_or(ContractError::ArbitrationCaseNotFound)?;

    if case.finalized {
        return Err(ContractError::CaseAlreadyFinalized);
    }
    if env.ledger().timestamp() > case.deadline {
        return Err(ContractError::VotingDeadlinePassed);
    }
    if !case.panel.contains(arbiter.clone()) {
        return Err(ContractError::NotPanelMember);
    }
    if Storage::get_arbiter_vote(env, case_id, arbiter).is_some() {
        return Err(ContractError::AlreadyVoted);
    }

    let vote = ArbiterVote {
        arbiter: arbiter.clone(),
        favor_contributor,
        confidence,
        voted_at: env.ledger().timestamp(),
    };
    Storage::set_arbiter_vote(env, case_id, &vote);
    case.votes.push_back(vote);
    Storage::set_arbitration_case(env, &case);

    ArbiterVoteCast {
        case_id,
        arbiter: arbiter.clone(),
        favor_contributor,
    }
    .publish(env);

    Ok(())
}

/// Finalize a case once voting closes. Majority vote determines the outcome.
pub fn finalize_case(env: &Env, case_id: u32) -> Result<bool, ContractError> {
    let mut case = Storage::get_arbitration_case(env, case_id)
        .ok_or(ContractError::ArbitrationCaseNotFound)?;

    if case.finalized {
        return Err(ContractError::CaseAlreadyFinalized);
    }

    // Voting must be closed: either deadline passed or all panellists voted.
    let all_voted = case.votes.len() >= case.panel.len();
    if env.ledger().timestamp() <= case.deadline && !all_voted {
        return Err(ContractError::InvalidState);
    }

    let mut favor: u32 = 0;
    let mut against: u32 = 0;
    for v in case.votes.iter() {
        if v.favor_contributor {
            favor += 1;
        } else {
            against += 1;
        }
    }

    // Majority (>50% of those who voted). Ties resolve in favour of the funder.
    let outcome = favor > against;
    case.outcome = Some(outcome);
    case.finalized = true;
    Storage::set_arbitration_case(env, &case);

    // Release the active-case lock on every panellist.
    for addr in case.panel.iter() {
        let count = Storage::get_arbiter_active_cases(env, &addr);
        Storage::set_arbiter_active_cases(env, &addr, count.saturating_sub(1));
    }

    CaseFinalized { case_id, outcome }.publish(env);

    Ok(outcome)
}

/// Distribute rewards to majority arbiters and slash minority arbiters.
pub fn settle_rewards(env: &Env, case_id: u32) -> Result<(), ContractError> {
    let case = Storage::get_arbitration_case(env, case_id)
        .ok_or(ContractError::ArbitrationCaseNotFound)?;

    if !case.finalized {
        return Err(ContractError::CaseNotFinalized);
    }
    if Storage::is_arbitration_settled(env, case_id) {
        return Err(ContractError::InvalidState);
    }

    let outcome = case.outcome.ok_or(ContractError::CaseNotFinalized)?;

    // Slash minority arbiters and accumulate the redistributable pot. Track the
    // confidence-weighted majority for proportional payout.
    let mut total_slashed: i128 = 0;
    let mut majority_weight: u64 = 0;
    for v in case.votes.iter() {
        let in_majority = v.favor_contributor == outcome;
        let mut arb = match Storage::get_arbiter(env, &v.arbiter) {
            Some(a) => a,
            None => continue,
        };
        arb.cases_decided = arb.cases_decided.saturating_add(1);
        if in_majority {
            arb.cases_correct = arb.cases_correct.saturating_add(1);
            majority_weight = majority_weight.saturating_add(v.confidence as u64);
        } else {
            let slash = arb
                .stake
                .checked_mul(ARBITER_SLASH_BPS as i128)
                .ok_or(ContractError::InvalidInput)?
                .checked_div(BASIS_POINTS_SCALE as i128)
                .ok_or(ContractError::InvalidInput)?;
            if slash > 0 {
                arb.stake = arb
                    .stake
                    .checked_sub(slash)
                    .ok_or(ContractError::InvalidInput)?;
                total_slashed = total_slashed.saturating_add(slash);
            }
        }
        Storage::set_arbiter(env, &arb);
    }

    // Distribute the slashed pot to majority arbiters, weighted by confidence.
    if total_slashed > 0 && majority_weight > 0 {
        let mut distributed: i128 = 0;
        let votes_len = case.votes.len();
        let mut last_majority_idx: Option<u32> = None;
        for i in 0..votes_len {
            let v = case.votes.get(i).ok_or(ContractError::InvalidInput)?;
            if v.favor_contributor == outcome {
                last_majority_idx = Some(i);
            }
        }
        for i in 0..votes_len {
            let v = case.votes.get(i).ok_or(ContractError::InvalidInput)?;
            if v.favor_contributor != outcome {
                continue;
            }
            let share = if Some(i) == last_majority_idx {
                total_slashed - distributed
            } else {
                total_slashed
                    .checked_mul(v.confidence as i128)
                    .ok_or(ContractError::InvalidInput)?
                    .checked_div(majority_weight as i128)
                    .ok_or(ContractError::InvalidInput)?
            };
            if share > 0 {
                if let Some(mut arb) = Storage::get_arbiter(env, &v.arbiter) {
                    arb.stake = arb
                        .stake
                        .checked_add(share)
                        .ok_or(ContractError::InvalidInput)?;
                    Storage::set_arbiter(env, &arb);
                    distributed = distributed.saturating_add(share);
                }
            }
        }
    }

    Storage::set_arbitration_settled(env, case_id);

    RewardsSettled {
        case_id,
        total_slashed,
    }
    .publish(env);

    Ok(())
}

/// Return pool statistics: (active_arbiters, total_staked).
pub fn pool_stats(env: &Env) -> (u32, i128) {
    let pool = Storage::get_arbiter_pool(env);
    let mut total: i128 = 0;
    for addr in pool.iter() {
        if let Some(a) = Storage::get_arbiter(env, &addr) {
            total = total.saturating_add(a.stake);
        }
    }
    (pool.len(), total)
}

/// Return an arbiter's profile.
pub fn get_arbiter(env: &Env, address: &Address) -> Option<Arbiter> {
    Storage::get_arbiter(env, address)
}
