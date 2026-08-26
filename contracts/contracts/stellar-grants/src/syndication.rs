use soroban_sdk::{Address, Env, Symbol, Vec};

use crate::errors::ContractError;
use crate::escrow;
use crate::storage::Storage;
use crate::types::{SyndicateGrant, SyndicateMember, SyndicateStatus};

fn total_deposited(env: &Env, grant_id: u64) -> i128 {
    let mut total = 0i128;
    for member in get_members(env, grant_id).iter() {
        total = total.saturating_add(member.deposited_amount);
    }
    total
}

/// Lead initiates a syndicate for a grant.
pub fn form_syndicate(
    env: &Env,
    lead: &Address,
    grant_id: u64,
    target_total: i128,
    min_commitment: i128,
    max_members: u32,
    deadline_ledgers: u32,
) -> Result<(), ContractError> {
    if target_total <= 0 || min_commitment <= 0 || max_members == 0 || deadline_ledgers == 0 {
        return Err(ContractError::InvalidInput);
    }
    if Storage::get_syndicate_grant(env, grant_id).is_some() {
        return Err(ContractError::InvalidState);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *lead {
        return Err(ContractError::Unauthorized);
    }

    let syndicate = SyndicateGrant {
        grant_id,
        lead: lead.clone(),
        target_total,
        token: grant.token,
        status: SyndicateStatus::Forming,
        member_count: 0,
        min_commitment,
        max_members,
        formation_deadline: (env.ledger().sequence().saturating_add(deadline_ledgers)) as u64,
    };

    Storage::set_syndicate_grant(env, grant_id, &syndicate);
    Storage::set_syndicate_member_index(env, grant_id, &Vec::new(env));
    env.events().publish(
        (Symbol::new(env, "syndicate_formed"), grant_id),
        (lead.clone(), target_total),
    );
    Ok(())
}

/// Member commits and deposits their share.
pub fn join_syndicate(
    env: &Env,
    member: &Address,
    grant_id: u64,
    amount: i128,
) -> Result<(), ContractError> {
    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let mut syndicate =
        Storage::get_syndicate_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if syndicate.status != SyndicateStatus::Forming {
        return Err(ContractError::InvalidState);
    }
    if (env.ledger().sequence() as u64) > syndicate.formation_deadline {
        return Err(ContractError::DeadlinePassed);
    }
    if amount < syndicate.min_commitment {
        return Err(ContractError::InvalidInput);
    }
    if Storage::get_syndicate_member(env, grant_id, member).is_none()
        && syndicate.member_count >= syndicate.max_members
    {
        return Err(ContractError::InvalidInput);
    }

    // Check total commitments don't exceed target_total
    let total_committed = total_deposited(env, grant_id);
    let remaining = syndicate.target_total.saturating_sub(total_committed);
    if amount > remaining {
        return Err(ContractError::InvalidInput);
    }

    escrow::deposit(env, grant_id, member, amount)?;

    let prior = Storage::get_syndicate_member(env, grant_id, member);
    let committed_amount = prior
        .as_ref()
        .map(|m| m.committed_amount)
        .unwrap_or(0)
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;
    let deposited_amount = prior
        .as_ref()
        .map(|m| m.deposited_amount)
        .unwrap_or(0)
        .checked_add(amount)
        .ok_or(ContractError::InvalidInput)?;
    let share_bps = committed_amount
        .saturating_mul(10_000)
        .checked_div(syndicate.target_total)
        .unwrap_or(0) as u32;

    let record = SyndicateMember {
        member: member.clone(),
        committed_amount,
        deposited_amount,
        share_bps,
        is_lead: *member == syndicate.lead,
        joined_at: prior
            .as_ref()
            .map(|m| m.joined_at)
            .unwrap_or_else(|| env.ledger().timestamp()),
    };
    Storage::set_syndicate_member(env, grant_id, member, &record);

    if prior.is_none() {
        let mut index = Storage::get_syndicate_member_index(env, grant_id);
        index.push_back(member.clone());
        Storage::set_syndicate_member_index(env, grant_id, &index);
        syndicate.member_count += 1;
        Storage::set_syndicate_grant(env, grant_id, &syndicate);
    }

    env.events().publish(
        (Symbol::new(env, "member_joined"), grant_id),
        (member.clone(), amount, share_bps),
    );
    Ok(())
}

/// Close syndicate formation and activate the grant once target is met.
pub fn close_syndicate(env: &Env, lead: &Address, grant_id: u64) -> Result<(), ContractError> {
    let mut syndicate =
        Storage::get_syndicate_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if syndicate.lead != *lead {
        return Err(ContractError::Unauthorized);
    }
    if syndicate.status != SyndicateStatus::Forming {
        return Err(ContractError::InvalidState);
    }

    let deposited = total_deposited(env, grant_id);
    if deposited < syndicate.target_total {
        return Err(ContractError::InvalidInput);
    }

    syndicate.status = SyndicateStatus::Active;
    Storage::set_syndicate_grant(env, grant_id, &syndicate);
    env.events().publish(
        (Symbol::new(env, "syndicate_closed"), grant_id),
        (lead.clone(), deposited),
    );
    Ok(())
}

/// Distribute a milestone payout proportionally across syndicate members' views.
pub fn record_payout_allocation(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
    payout: i128,
) -> Result<(), ContractError> {
    if payout <= 0 {
        return Err(ContractError::ZeroAmount);
    }
    let syndicate =
        Storage::get_syndicate_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if syndicate.lead != *caller {
        return Err(ContractError::Unauthorized);
    }
    if syndicate.status != SyndicateStatus::Active {
        return Err(ContractError::InvalidState);
    }

    let mut allocations: Vec<(Address, i128)> = Vec::new(env);
    for member in get_members(env, grant_id).iter() {
        let amount = payout
            .saturating_mul(member.share_bps as i128)
            .checked_div(10_000)
            .unwrap_or(0);
        allocations.push_back((member.member, amount));
    }
    Storage::set_syndicate_payouts(env, grant_id, milestone_idx, &allocations);
    Ok(())
}

/// Allow members to withdraw after an unclosed formation expires.
pub fn withdraw_syndicate(
    env: &Env,
    member: &Address,
    grant_id: u64,
) -> Result<i128, ContractError> {
    let mut syndicate =
        Storage::get_syndicate_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if syndicate.status != SyndicateStatus::Forming {
        return Err(ContractError::InvalidState);
    }
    if (env.ledger().sequence() as u64) <= syndicate.formation_deadline {
        return Err(ContractError::InvalidState);
    }

    let record = Storage::get_syndicate_member(env, grant_id, member)
        .ok_or(ContractError::NoRefundableAmount)?;
    let amount = escrow::refund(env, grant_id, member)?;
    Storage::remove_syndicate_member(env, grant_id, member);

    let mut index = Storage::get_syndicate_member_index(env, grant_id);
    let mut updated = Vec::new(env);
    for addr in index.iter() {
        if addr != *member {
            updated.push_back(addr);
        }
    }
    index = updated;
    Storage::set_syndicate_member_index(env, grant_id, &index);
    syndicate.member_count = syndicate.member_count.saturating_sub(1);
    Storage::set_syndicate_grant(env, grant_id, &syndicate);

    Ok(amount.min(record.deposited_amount))
}

/// Return a member's syndicate record.
pub fn get_member(env: &Env, grant_id: u64, member: &Address) -> Option<SyndicateMember> {
    Storage::get_syndicate_member(env, grant_id, member)
}

/// Return all syndicate members for a grant.
pub fn get_members(env: &Env, grant_id: u64) -> Vec<SyndicateMember> {
    let mut members = Vec::new(env);
    for addr in Storage::get_syndicate_member_index(env, grant_id).iter() {
        if let Some(member) = Storage::get_syndicate_member(env, grant_id, &addr) {
            members.push_back(member);
        }
    }
    members
}

/// Return the syndicate grant config.
pub fn get_syndicate(env: &Env, grant_id: u64) -> Option<SyndicateGrant> {
    Storage::get_syndicate_grant(env, grant_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{DataKey, GrantKey, Storage};
    use crate::types::{Grant, GrantStatus};
    use crate::{StellarGrantsContract, StellarGrantsContractClient};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token, token::StellarAssetClient, Address, Env, String, Vec};

    const GRANT_ID: u64 = 1;
    const TARGET: i128 = 1_000;
    const MIN_COMMIT: i128 = 100;
    const MAX_MEMBERS: u32 = 5;
    const DEADLINE_LEDGERS: u32 = 50;

    struct Fixture<'a> {
        env: Env,
        contract_id: Address,
        client: StellarGrantsContractClient<'a>,
        lead: Address,
        member_a: Address,
        member_b: Address,
        token_id: Address,
    }

    fn setup<'a>() -> Fixture<'a> {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(&env, &contract_id);
        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = asset.address();
        let stellar_asset = StellarAssetClient::new(&env, &token_id);

        let lead = Address::generate(&env);
        let member_a = Address::generate(&env);
        let member_b = Address::generate(&env);

        stellar_asset.mint(&lead, &10_000);
        stellar_asset.mint(&member_a, &10_000);
        stellar_asset.mint(&member_b, &10_000);

        let grant = Grant {
            id: GRANT_ID,
            owner: lead.clone(),
            title: String::from_str(&env, "Syndicated Grant"),
            description: String::from_str(&env, "Desc"),
            token: token_id.clone(),
            status: GrantStatus::Active,
            total_amount: TARGET,
            milestone_amount: TARGET,
            reviewers: Vec::new(&env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: Vec::new(&env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };

        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, GRANT_ID, &grant);
            escrow::open(&env, GRANT_ID, &lead, &token_id).unwrap();
        });

        Fixture {
            env,
            contract_id,
            client,
            lead,
            member_a,
            member_b,
            token_id,
        }
    }

    fn form(f: &Fixture<'_>) {
        f.client.form_syndicate(
            &f.lead,
            &GRANT_ID,
            &TARGET,
            &MIN_COMMIT,
            &MAX_MEMBERS,
            &DEADLINE_LEDGERS,
        );
    }

    fn join(f: &Fixture<'_>, member: &Address, amount: i128) {
        f.client.join_syndicate(member, &GRANT_ID, &amount);
    }

    fn read_payouts(f: &Fixture<'_>, milestone_idx: u32) -> Vec<(Address, i128)> {
        f.env.as_contract(&f.contract_id, || {
            f.env
                .storage()
                .persistent()
                .get(&DataKey::Grant(GrantKey::SyndicatePayouts(
                    GRANT_ID,
                    milestone_idx,
                )))
                .unwrap()
        })
    }

    #[test]
    fn test_form_syndicate_creates_forming_syndicate() {
        let f = setup();
        form(&f);

        f.env.as_contract(&f.contract_id, || {
            let syn = get_syndicate(&f.env, GRANT_ID).unwrap();
            assert_eq!(syn.lead, f.lead);
            assert_eq!(syn.target_total, TARGET);
            assert_eq!(syn.min_commitment, MIN_COMMIT);
            assert_eq!(syn.max_members, MAX_MEMBERS);
            assert_eq!(syn.status, SyndicateStatus::Forming);
            assert_eq!(syn.member_count, 0);
            assert_eq!(syn.token, f.token_id);
            assert!(get_members(&f.env, GRANT_ID).is_empty());
        });
    }

    #[test]
    fn test_form_syndicate_rejects_non_owner() {
        let f = setup();
        let stranger = Address::generate(&f.env);
        let err = f
            .client
            .try_form_syndicate(
                &stranger,
                &GRANT_ID,
                &TARGET,
                &MIN_COMMIT,
                &MAX_MEMBERS,
                &DEADLINE_LEDGERS,
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, ContractError::Unauthorized);
    }

    #[test]
    fn test_form_syndicate_rejects_invalid_params_and_duplicate() {
        let f = setup();

        assert_eq!(
            f.client
                .try_form_syndicate(&f.lead, &GRANT_ID, &0, &MIN_COMMIT, &MAX_MEMBERS, &10)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidInput
        );
        assert_eq!(
            f.client
                .try_form_syndicate(&f.lead, &GRANT_ID, &TARGET, &0, &MAX_MEMBERS, &10)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidInput
        );
        assert_eq!(
            f.client
                .try_form_syndicate(&f.lead, &GRANT_ID, &TARGET, &MIN_COMMIT, &0, &10)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidInput
        );
        assert_eq!(
            f.client
                .try_form_syndicate(&f.lead, &GRANT_ID, &TARGET, &MIN_COMMIT, &MAX_MEMBERS, &0)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidInput
        );

        form(&f);
        assert_eq!(
            f.client
                .try_form_syndicate(
                    &f.lead,
                    &GRANT_ID,
                    &TARGET,
                    &MIN_COMMIT,
                    &MAX_MEMBERS,
                    &DEADLINE_LEDGERS,
                )
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidState
        );
    }

    #[test]
    fn test_join_and_membership_tracks_shares() {
        let f = setup();
        form(&f);

        // 60/40 of a 1000 target → 6000 / 4000 bps
        join(&f, &f.member_a, 600);
        join(&f, &f.member_b, 400);

        f.env.as_contract(&f.contract_id, || {
            let members = get_members(&f.env, GRANT_ID);
            assert_eq!(members.len(), 2);

            let a = get_member(&f.env, GRANT_ID, &f.member_a).unwrap();
            assert_eq!(a.deposited_amount, 600);
            assert_eq!(a.committed_amount, 600);
            assert_eq!(a.share_bps, 6_000);
            assert!(!a.is_lead);

            let b = get_member(&f.env, GRANT_ID, &f.member_b).unwrap();
            assert_eq!(b.share_bps, 4_000);

            let syn = get_syndicate(&f.env, GRANT_ID).unwrap();
            assert_eq!(syn.member_count, 2);
        });

        // Top-up from an existing member updates amounts and share_bps.
        join(&f, &f.member_a, 100);

        f.env.as_contract(&f.contract_id, || {
            let a2 = get_member(&f.env, GRANT_ID, &f.member_a).unwrap();
            assert_eq!(a2.deposited_amount, 700);
            assert_eq!(a2.share_bps, 7_000);
            assert_eq!(get_syndicate(&f.env, GRANT_ID).unwrap().member_count, 2);
        });
    }

    #[test]
    fn test_join_rejects_below_min_and_when_full() {
        let f = setup();
        f.client.form_syndicate(
            &f.lead,
            &GRANT_ID,
            &TARGET,
            &MIN_COMMIT,
            &1,
            &DEADLINE_LEDGERS,
        );

        assert_eq!(
            f.client
                .try_join_syndicate(&f.member_a, &GRANT_ID, &(MIN_COMMIT - 1))
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidInput
        );

        join(&f, &f.member_a, MIN_COMMIT);

        assert_eq!(
            f.client
                .try_join_syndicate(&f.member_b, &GRANT_ID, &MIN_COMMIT)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidInput
        );
    }

    #[test]
    fn test_close_syndicate_requires_target_and_lead() {
        let f = setup();
        form(&f);
        join(&f, &f.member_a, 600);

        // Under target
        assert_eq!(
            f.client
                .try_close_syndicate(&f.lead, &GRANT_ID)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidInput
        );

        // Non-lead
        assert_eq!(
            f.client
                .try_close_syndicate(&f.member_a, &GRANT_ID)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );

        join(&f, &f.member_b, 400);
        f.client.close_syndicate(&f.lead, &GRANT_ID);

        f.env.as_contract(&f.contract_id, || {
            assert_eq!(
                get_syndicate(&f.env, GRANT_ID).unwrap().status,
                SyndicateStatus::Active
            );
        });
    }

    #[test]
    fn test_record_payout_allocation_proportional_splits() {
        let f = setup();
        form(&f);
        join(&f, &f.member_a, 600);
        join(&f, &f.member_b, 400);
        f.client.close_syndicate(&f.lead, &GRANT_ID);

        let payout = 1_000i128;
        f.client
            .record_payout_allocation(&f.lead, &GRANT_ID, &0, &payout);

        assert_eq!(
            f.client
                .try_record_payout_allocation(&f.lead, &GRANT_ID, &0, &0)
                .unwrap_err()
                .unwrap(),
            ContractError::ZeroAmount
        );

        let allocations = read_payouts(&f, 0);
        assert_eq!(allocations.len(), 2);

        let mut got_a = 0i128;
        let mut got_b = 0i128;
        for (addr, amount) in allocations.iter() {
            if addr == f.member_a {
                got_a = amount;
            } else if addr == f.member_b {
                got_b = amount;
            }
        }
        // 6000 bps / 4000 bps of 1000
        assert_eq!(got_a, 600);
        assert_eq!(got_b, 400);
        assert_eq!(got_a.saturating_add(got_b), payout);
    }

    #[test]
    fn test_record_payout_uses_saturating_mul_without_panic() {
        let f = setup();
        form(&f);
        // Full target from one member → share_bps = 10000.
        join(&f, &f.member_a, TARGET);
        f.client.close_syndicate(&f.lead, &GRANT_ID);

        // saturating_mul(i128::MAX, 10000) must not panic through checked_div.
        f.client
            .record_payout_allocation(&f.lead, &GRANT_ID, &1, &i128::MAX);

        let allocations = read_payouts(&f, 1);
        assert_eq!(allocations.len(), 1);
        let (_addr, amount) = allocations.get(0).unwrap();
        assert!(amount >= 0);
    }

    #[test]
    fn test_record_payout_rejected_while_forming() {
        let f = setup();
        form(&f);
        join(&f, &f.member_a, TARGET);

        assert_eq!(
            f.client
                .try_record_payout_allocation(&f.lead, &GRANT_ID, &0, &100)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidState
        );
    }

    #[test]
    fn test_record_payout_allocation_rejects_stranger() {
        let f = setup();
        form(&f);
        join(&f, &f.member_a, 600);
        join(&f, &f.member_b, 400);
        f.client.close_syndicate(&f.lead, &GRANT_ID);

        let stranger = Address::generate(&f.env);
        assert_eq!(
            f.client
                .try_record_payout_allocation(&stranger, &GRANT_ID, &0, &1_000)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    #[test]
    fn test_withdraw_syndicate_after_deadline() {
        let f = setup();
        form(&f);
        join(&f, &f.member_a, 500);

        let token_client = token::Client::new(&f.env, &f.token_id);
        let before = token_client.balance(&f.member_a);

        // Still within deadline → rejected
        assert_eq!(
            f.client
                .try_withdraw_syndicate(&f.member_a, &GRANT_ID)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidState
        );

        // Advance past formation_deadline
        let deadline = f.env.as_contract(&f.contract_id, || {
            get_syndicate(&f.env, GRANT_ID).unwrap().formation_deadline
        });
        f.env
            .ledger()
            .with_mut(|li| li.sequence_number = deadline as u32 + 1);

        let refunded = f.client.withdraw_syndicate(&f.member_a, &GRANT_ID);
        assert_eq!(refunded, 500);
        assert_eq!(token_client.balance(&f.member_a), before + 500);

        f.env.as_contract(&f.contract_id, || {
            assert!(get_member(&f.env, GRANT_ID, &f.member_a).is_none());
            assert_eq!(get_syndicate(&f.env, GRANT_ID).unwrap().member_count, 0);
        });
    }

    #[test]
    fn test_withdraw_rejects_non_member_and_double_withdraw() {
        let f = setup();
        form(&f);
        join(&f, &f.member_a, 500);

        let deadline = f.env.as_contract(&f.contract_id, || {
            get_syndicate(&f.env, GRANT_ID).unwrap().formation_deadline
        });
        f.env
            .ledger()
            .with_mut(|li| li.sequence_number = deadline as u32 + 1);

        // Non-member (never joined)
        assert_eq!(
            f.client
                .try_withdraw_syndicate(&f.member_b, &GRANT_ID)
                .unwrap_err()
                .unwrap(),
            ContractError::NoRefundableAmount
        );

        // First withdraw ok
        f.client.withdraw_syndicate(&f.member_a, &GRANT_ID);

        // Double withdraw rejected (member record removed)
        assert_eq!(
            f.client
                .try_withdraw_syndicate(&f.member_a, &GRANT_ID)
                .unwrap_err()
                .unwrap(),
            ContractError::NoRefundableAmount
        );
    }

    #[test]
    fn test_withdraw_rejected_after_syndicate_closed() {
        let f = setup();
        form(&f);
        join(&f, &f.member_a, TARGET);
        f.client.close_syndicate(&f.lead, &GRANT_ID);

        let deadline = f.env.as_contract(&f.contract_id, || {
            get_syndicate(&f.env, GRANT_ID).unwrap().formation_deadline
        });
        f.env
            .ledger()
            .with_mut(|li| li.sequence_number = deadline as u32 + 1);

        assert_eq!(
            f.client
                .try_withdraw_syndicate(&f.member_a, &GRANT_ID)
                .unwrap_err()
                .unwrap(),
            ContractError::InvalidState
        );
    }
}
