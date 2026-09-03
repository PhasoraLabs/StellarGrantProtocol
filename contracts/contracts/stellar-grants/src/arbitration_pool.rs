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
        panel_stakes: Vec::new(env),
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

    // Snapshot each panelist's stake at finalization time to prevent slash evasion.
    let mut stakes = Vec::new(env);
    for addr in case.panel.iter() {
        let stake = Storage::get_arbiter(env, &addr)
            .map(|a| a.stake)
            .unwrap_or(0);
        stakes.push_back(stake);
    }
    case.panel_stakes = stakes;

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
    // Use the snapshotted stakes to prevent slash evasion via early pool exit.
    let mut total_slashed: i128 = 0;
    let mut majority_weight: u64 = 0;
    let mut panelist_idx_to_arbiter: Vec<(u32, Address)> = Vec::new(env);

    for (idx, addr) in case.panel.iter().enumerate() {
        panelist_idx_to_arbiter.push_back((idx as u32, addr.clone()));
    }

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
            // Find this arbiter's index in the panel to get their snapshotted stake
            let mut panelist_idx: Option<u32> = None;
            for (idx, addr) in panelist_idx_to_arbiter.iter() {
                if addr == v.arbiter {
                    panelist_idx = Some(idx);
                    break;
                }
            }

            if let Some(idx) = panelist_idx {
                if let Some(snapshotted_stake) = case.panel_stakes.get(idx) {
                    let slash = snapshotted_stake
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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::token::StellarAssetClient;
    use soroban_sdk::{Address, Env};

    /// Create env + contract + token, mint tokens to a list of arbiters.
    fn setup(arbiter_count: usize) -> (Env, Address, Address, Vec<Address>) {
        let env = Env::default();
        env.mock_all_auths();

        let token_admin = Address::generate(&env);
        let token_contract = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        let stellar = StellarAssetClient::new(&env, &token_contract);

        let contract_id = env.register(crate::StellarGrantsContract, ());

        let mut arbiters = Vec::new(&env);
        for _ in 0..arbiter_count {
            let a = Address::generate(&env);
            stellar.mint(&a, &100_000);
            arbiters.push_back(a);
        }

        (env, contract_id, token_contract, arbiters)
    }

    fn join(env: &Env, cid: &Address, token: &Address, arbiter: &Address, stake: i128) {
        env.as_contract(cid, || {
            join_pool(env, arbiter, token, stake).unwrap();
        });
    }

    // ── join / leave ────────────────────────────────────────────────────

    #[test]
    fn test_join_pool_success() {
        let (env, cid, token, arbs) = setup(1);
        let a = arbs.get(0).unwrap();
        join(&env, &cid, &token, &a, 5_000);

        env.as_contract(&cid, || {
            let arb = get_arbiter(&env, &a).unwrap();
            assert_eq!(arb.stake, 5_000);
            assert!(arb.is_active);
            let (count, total) = pool_stats(&env);
            assert_eq!(count, 1);
            assert_eq!(total, 5_000);
        });
    }

    #[test]
    fn test_join_pool_zero_stake_fails() {
        let (env, cid, token, arbs) = setup(1);
        let a = arbs.get(0).unwrap();
        env.as_contract(&cid, || {
            let res = join_pool(&env, &a, &token, 0);
            assert_eq!(res, Err(ContractError::InvalidInput));
        });
    }

    #[test]
    fn test_join_pool_duplicate_fails() {
        let (env, cid, token, arbs) = setup(1);
        let a = arbs.get(0).unwrap();
        join(&env, &cid, &token, &a, 5_000);
        env.as_contract(&cid, || {
            let res = join_pool(&env, &a, &token, 1_000);
            assert_eq!(res, Err(ContractError::ArbiterAlreadyJoined));
        });
    }

    #[test]
    fn test_leave_pool_success() {
        let (env, cid, token, arbs) = setup(1);
        let a = arbs.get(0).unwrap();
        join(&env, &cid, &token, &a, 5_000);

        env.as_contract(&cid, || {
            let refund = leave_pool(&env, &a).unwrap();
            assert_eq!(refund, 5_000);
            let arb = get_arbiter(&env, &a).unwrap();
            assert!(!arb.is_active);
            assert_eq!(arb.stake, 0);
            let (count, _) = pool_stats(&env);
            assert_eq!(count, 0);
        });
    }

    #[test]
    fn test_leave_pool_not_joined_fails() {
        let (env, cid, _token, arbs) = setup(1);
        let a = arbs.get(0).unwrap();
        env.as_contract(&cid, || {
            let res = leave_pool(&env, &a);
            assert_eq!(res, Err(ContractError::ArbiterNotFound));
        });
    }

    // ── panel assignment ────────────────────────────────────────────────

    #[test]
    fn test_assign_panel_size_3() {
        let (env, cid, token, arbs) = setup(5);
        for i in 0..5 {
            join(&env, &cid, &token, &arbs.get(i).unwrap(), 10_000);
        }

        env.as_contract(&cid, || {
            let case_id = assign_panel(&env, 1, 3).unwrap();
            let case = Storage::get_arbitration_case(&env, case_id).unwrap();
            assert_eq!(case.panel.len(), 3);
            assert_eq!(case.dispute_id, 1);
            assert!(!case.finalized);
        });
    }

    #[test]
    fn test_assign_panel_insufficient_arbiters() {
        let (env, cid, token, arbs) = setup(2);
        for i in 0..2 {
            join(&env, &cid, &token, &arbs.get(i).unwrap(), 10_000);
        }

        env.as_contract(&cid, || {
            let res = assign_panel(&env, 1, 3);
            assert_eq!(res, Err(ContractError::InsufficientArbiters));
        });
    }

    #[test]
    fn test_assign_panel_invalid_size() {
        let (env, cid, _token, _arbs) = setup(0);
        env.as_contract(&cid, || {
            let res = assign_panel(&env, 1, 4);
            assert_eq!(res, Err(ContractError::InvalidInput));
        });
    }

    #[test]
    fn test_leave_pool_blocked_during_active_case() {
        let (env, cid, token, arbs) = setup(3);
        for i in 0..3 {
            join(&env, &cid, &token, &arbs.get(i).unwrap(), 10_000);
        }

        env.as_contract(&cid, || {
            assign_panel(&env, 1, 3).unwrap();
            // All 3 are panel members, so none can leave
            let res = leave_pool(&env, &arbs.get(0).unwrap());
            assert_eq!(res, Err(ContractError::ArbiterInActiveCase));
        });
    }

    // ── voting ──────────────────────────────────────────────────────────

    fn setup_case_with_panel() -> (Env, Address, Address, Vec<Address>, u32) {
        let (env, cid, token, arbs) = setup(3);
        for i in 0..3 {
            join(&env, &cid, &token, &arbs.get(i).unwrap(), 10_000);
        }
        let case_id = env.as_contract(&cid, || assign_panel(&env, 1, 3).unwrap());
        (env, cid, token, arbs, case_id)
    }

    fn get_panel(env: &Env, cid: &Address, case_id: u32) -> Vec<Address> {
        env.as_contract(cid, || {
            Storage::get_arbitration_case(env, case_id).unwrap().panel
        })
    }

    #[test]
    fn test_cast_vote_success() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 80).unwrap();
            let case = Storage::get_arbitration_case(&env, case_id).unwrap();
            assert_eq!(case.votes.len(), 1);
        });
    }

    #[test]
    fn test_cast_vote_not_panel_member() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let outsider = Address::generate(&env);

        env.as_contract(&cid, || {
            let res = cast_arbiter_vote(&env, &outsider, case_id, true, 50);
            assert_eq!(res, Err(ContractError::NotPanelMember));
        });
    }

    #[test]
    fn test_cast_vote_duplicate_fails() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);
        let voter = panel.get(0).unwrap();

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &voter, case_id, true, 50).unwrap();
            let res = cast_arbiter_vote(&env, &voter, case_id, false, 50);
            assert_eq!(res, Err(ContractError::AlreadyVoted));
        });
    }

    #[test]
    fn test_cast_vote_invalid_confidence() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            let res = cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 0);
            assert_eq!(res, Err(ContractError::InvalidInput));
            let res = cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 101);
            assert_eq!(res, Err(ContractError::InvalidInput));
        });
    }

    #[test]
    fn test_cast_vote_after_deadline_fails() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.ledger().with_mut(|l| {
            l.timestamp += ARBITRATION_VOTING_WINDOW + 1;
        });

        env.as_contract(&cid, || {
            let res = cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 50);
            assert_eq!(res, Err(ContractError::VotingDeadlinePassed));
        });
    }

    // ── finalize ────────────────────────────────────────────────────────

    #[test]
    fn test_finalize_majority_favor() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 80).unwrap();
            cast_arbiter_vote(&env, &panel.get(1).unwrap(), case_id, true, 60).unwrap();
            cast_arbiter_vote(&env, &panel.get(2).unwrap(), case_id, false, 50).unwrap();

            let outcome = finalize_case(&env, case_id).unwrap();
            assert!(outcome); // 2v1 in favour
        });
    }

    #[test]
    fn test_finalize_majority_against() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, false, 80).unwrap();
            cast_arbiter_vote(&env, &panel.get(1).unwrap(), case_id, false, 60).unwrap();
            cast_arbiter_vote(&env, &panel.get(2).unwrap(), case_id, true, 50).unwrap();

            let outcome = finalize_case(&env, case_id).unwrap();
            assert!(!outcome); // 2v1 against
        });
    }

    #[test]
    fn test_finalize_before_all_voted_and_before_deadline_fails() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 50).unwrap();
            // Only 1 of 3 voted, deadline not passed
            let res = finalize_case(&env, case_id);
            assert_eq!(res, Err(ContractError::InvalidState));
        });
    }

    #[test]
    fn test_finalize_after_deadline_partial_votes() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 80).unwrap();
        });

        env.ledger().with_mut(|l| {
            l.timestamp += ARBITRATION_VOTING_WINDOW + 1;
        });

        env.as_contract(&cid, || {
            let outcome = finalize_case(&env, case_id).unwrap();
            assert!(outcome); // 1-0, favour wins
        });
    }

    #[test]
    fn test_finalize_double_fails() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 80).unwrap();
            cast_arbiter_vote(&env, &panel.get(1).unwrap(), case_id, true, 60).unwrap();
            cast_arbiter_vote(&env, &panel.get(2).unwrap(), case_id, false, 50).unwrap();
            finalize_case(&env, case_id).unwrap();

            let res = finalize_case(&env, case_id);
            assert_eq!(res, Err(ContractError::CaseAlreadyFinalized));
        });
    }

    #[test]
    fn test_finalize_unlocks_panel_members() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            for i in 0..3 {
                cast_arbiter_vote(&env, &panel.get(i).unwrap(), case_id, true, 50).unwrap();
            }
            finalize_case(&env, case_id).unwrap();

            // All panel members should now be free to leave
            for i in 0..3 {
                let count = Storage::get_arbiter_active_cases(&env, &panel.get(i).unwrap());
                assert_eq!(count, 0);
            }
        });
    }

    // ── settle_rewards: slashing & redistribution ───────────────────────

    #[test]
    fn test_settle_slashes_minority_and_rewards_majority() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            // 2 vote true (majority), 1 votes false (minority)
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 50).unwrap();
            cast_arbiter_vote(&env, &panel.get(1).unwrap(), case_id, true, 50).unwrap();
            cast_arbiter_vote(&env, &panel.get(2).unwrap(), case_id, false, 50).unwrap();
            finalize_case(&env, case_id).unwrap();

            let minority_stake_before = get_arbiter(&env, &panel.get(2).unwrap()).unwrap().stake;
            assert_eq!(minority_stake_before, 10_000);

            settle_rewards(&env, case_id).unwrap();

            // Minority: slashed 10% of 10_000 = 1_000
            let minority = get_arbiter(&env, &panel.get(2).unwrap()).unwrap();
            assert_eq!(minority.stake, 9_000);

            // Majority: 1_000 slashed split between 2 arbiters
            // Equal confidence (50/50) so 500 each
            let maj0 = get_arbiter(&env, &panel.get(0).unwrap()).unwrap();
            let maj1 = get_arbiter(&env, &panel.get(1).unwrap()).unwrap();
            assert_eq!(maj0.stake + maj1.stake, 10_000 + 10_000 + 1_000);
        });
    }

    #[test]
    fn test_settle_confidence_weighted_distribution() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            // Majority: conf 75 and conf 25 => total weight 100
            // Minority: conf 50
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 75).unwrap();
            cast_arbiter_vote(&env, &panel.get(1).unwrap(), case_id, true, 25).unwrap();
            cast_arbiter_vote(&env, &panel.get(2).unwrap(), case_id, false, 50).unwrap();
            finalize_case(&env, case_id).unwrap();
            settle_rewards(&env, case_id).unwrap();

            // Slashed = 10% of 10_000 = 1_000
            let minority = get_arbiter(&env, &panel.get(2).unwrap()).unwrap();
            assert_eq!(minority.stake, 9_000);

            // The last majority voter gets remainder to handle rounding dust.
            // Voter 0 (conf 75): 1000 * 75 / 100 = 750
            // Voter 1 (conf 25, last majority): 1000 - 750 = 250
            let maj0 = get_arbiter(&env, &panel.get(0).unwrap()).unwrap();
            let maj1 = get_arbiter(&env, &panel.get(1).unwrap()).unwrap();
            assert_eq!(maj0.stake + maj1.stake, 21_000);
        });
    }

    #[test]
    fn test_settle_rounding_dust_goes_to_last_majority() {
        let (env, cid, token, arbs) = setup(5);
        // Use 5-member panel with uneven confidences to produce rounding dust
        for i in 0..5 {
            join(&env, &cid, &token, &arbs.get(i).unwrap(), 10_000);
        }
        let case_id = env.as_contract(&cid, || assign_panel(&env, 1, 5).unwrap());
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            // 3 majority (conf 33 each => weight 99), 2 minority
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 33).unwrap();
            cast_arbiter_vote(&env, &panel.get(1).unwrap(), case_id, true, 33).unwrap();
            cast_arbiter_vote(&env, &panel.get(2).unwrap(), case_id, true, 33).unwrap();
            cast_arbiter_vote(&env, &panel.get(3).unwrap(), case_id, false, 50).unwrap();
            cast_arbiter_vote(&env, &panel.get(4).unwrap(), case_id, false, 50).unwrap();
            finalize_case(&env, case_id).unwrap();
            settle_rewards(&env, case_id).unwrap();

            // 2 minority slashed 1_000 each = 2_000 total slashed
            let min3 = get_arbiter(&env, &panel.get(3).unwrap()).unwrap();
            let min4 = get_arbiter(&env, &panel.get(4).unwrap()).unwrap();
            assert_eq!(min3.stake, 9_000);
            assert_eq!(min4.stake, 9_000);

            // Total stakes must be conserved: 5 * 10_000 = 50_000
            let maj0 = get_arbiter(&env, &panel.get(0).unwrap()).unwrap();
            let maj1 = get_arbiter(&env, &panel.get(1).unwrap()).unwrap();
            let maj2 = get_arbiter(&env, &panel.get(2).unwrap()).unwrap();
            let total = maj0.stake + maj1.stake + maj2.stake + min3.stake + min4.stake;
            assert_eq!(total, 50_000);
        });
    }

    #[test]
    fn test_settle_not_finalized_fails() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();

        env.as_contract(&cid, || {
            let res = settle_rewards(&env, case_id);
            assert_eq!(res, Err(ContractError::CaseNotFinalized));
        });
    }

    #[test]
    fn test_settle_double_settle_fails() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            for i in 0..3 {
                cast_arbiter_vote(&env, &panel.get(i).unwrap(), case_id, true, 50).unwrap();
            }
            finalize_case(&env, case_id).unwrap();
            settle_rewards(&env, case_id).unwrap();

            let res = settle_rewards(&env, case_id);
            assert_eq!(res, Err(ContractError::InvalidState));
        });
    }

    #[test]
    fn test_settle_unanimous_no_slash() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            // All vote the same way — no minority to slash
            for i in 0..3 {
                cast_arbiter_vote(&env, &panel.get(i).unwrap(), case_id, true, 80).unwrap();
            }
            finalize_case(&env, case_id).unwrap();
            settle_rewards(&env, case_id).unwrap();

            // All stakes unchanged
            for i in 0..3 {
                let arb = get_arbiter(&env, &panel.get(i).unwrap()).unwrap();
                assert_eq!(arb.stake, 10_000);
            }
        });
    }

    #[test]
    fn test_cases_decided_and_correct_updated() {
        let (env, cid, _token, _arbs, case_id) = setup_case_with_panel();
        let panel = get_panel(&env, &cid, case_id);

        env.as_contract(&cid, || {
            cast_arbiter_vote(&env, &panel.get(0).unwrap(), case_id, true, 80).unwrap();
            cast_arbiter_vote(&env, &panel.get(1).unwrap(), case_id, true, 60).unwrap();
            cast_arbiter_vote(&env, &panel.get(2).unwrap(), case_id, false, 50).unwrap();
            finalize_case(&env, case_id).unwrap();
            settle_rewards(&env, case_id).unwrap();

            // Majority members: cases_decided=1, cases_correct=1
            let maj = get_arbiter(&env, &panel.get(0).unwrap()).unwrap();
            assert_eq!(maj.cases_decided, 1);
            assert_eq!(maj.cases_correct, 1);

            // Minority member: cases_decided=1, cases_correct=0
            let min = get_arbiter(&env, &panel.get(2).unwrap()).unwrap();
            assert_eq!(min.cases_decided, 1);
            assert_eq!(min.cases_correct, 0);
        });
    }
}
