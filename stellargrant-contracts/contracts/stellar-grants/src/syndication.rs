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
    grant_id: u64,
    milestone_idx: u32,
    payout: i128,
) -> Result<(), ContractError> {
    if payout <= 0 {
        return Err(ContractError::ZeroAmount);
    }
    let syndicate =
        Storage::get_syndicate_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
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
