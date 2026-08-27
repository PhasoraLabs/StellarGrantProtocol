use soroban_sdk::{contractevent, contracttype, Address, Env, Vec};

use crate::types::{ContractError, Delegation, DelegationScope};
use crate::Storage;

#[contracttype]
pub enum DelegateKey {
    Delegation(Address, DelegationScope),
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelegationCreated {
    pub delegator: Address,
    pub delegate: Address,
    pub created_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelegationRevoked {
    pub delegator: Address,
    pub revoked_at: u64,
}

fn is_registered_reviewer(env: &Env, reviewer: &Address) -> bool {
    let count = Storage::get_grant_count(env);
    let mut id = 1u64;
    while id <= count {
        if let Some(grant) = Storage::get_grant(env, id) {
            if grant.reviewers.contains(reviewer.clone()) {
                return true;
            }
        }
        id += 1;
    }
    false
}

fn get_raw(env: &Env, delegator: &Address, scope: &DelegationScope) -> Option<Delegation> {
    env.storage()
        .persistent()
        .get(&DelegateKey::Delegation(delegator.clone(), scope.clone()))
}

fn set_raw(env: &Env, delegation: &Delegation) {
    env.storage().persistent().set(
        &DelegateKey::Delegation(delegation.delegator.clone(), delegation.scope.clone()),
        delegation,
    );
}

fn active_for_scope(env: &Env, delegator: &Address, scope: &DelegationScope) -> Option<Delegation> {
    let mut delegation = get_raw(env, delegator, scope)?;
    if delegation.revoked || delegation.uses_remaining == Some(0) {
        return None;
    }
    if let Some(expires_at) = delegation.expires_at {
        if env.ledger().timestamp() >= expires_at {
            delegation.revoked = true;
            set_raw(env, &delegation);
            return None;
        }
    }
    Some(delegation)
}

fn active_matching(env: &Env, delegator: &Address, grant_id: u64) -> Option<Delegation> {
    let per_grant = DelegationScope::PerGrant(grant_id);
    active_for_scope(env, delegator, &per_grant)
        .or_else(|| active_for_scope(env, delegator, &DelegationScope::Global))
}

fn would_create_cycle(env: &Env, delegator: &Address, delegate: &Address, scope: &DelegationScope) -> bool {
    if delegator == delegate {
        return true;
    }
    let mut current = delegate.clone();
    let mut visited = Vec::new(env);
    let max_chain_length = 256;

    loop {
        if current == *delegator {
            return true;
        }
        if visited.contains(&current) {
            return true;
        }
        if visited.len() >= max_chain_length {
            return false;
        }
        visited.push_back(current.clone());

        let next = active_for_scope(env, &current, scope)
            .or_else(|| active_for_scope(env, &current, &DelegationScope::Global));
        match next {
            Some(d) => current = d.delegate,
            None => return false,
        }
    }
}

pub fn delegate_vote(
    env: &Env,
    delegator: &Address,
    delegate: &Address,
    scope: DelegationScope,
    expires_at: Option<u64>,
    max_uses: Option<u32>,
) -> Result<(), ContractError> {
    delegator.require_auth();
    if !is_registered_reviewer(env, delegator) {
        return Err(ContractError::Unauthorized);
    }
    if max_uses == Some(0) || would_create_cycle(env, delegator, delegate, &scope) {
        return Err(ContractError::Unauthorized);
    }

    let delegation = Delegation {
        delegator: delegator.clone(),
        delegate: delegate.clone(),
        scope,
        created_at: env.ledger().timestamp(),
        expires_at,
        revoked: false,
        uses_remaining: max_uses,
    };
    set_raw(env, &delegation);
    DelegationCreated {
        delegator: delegator.clone(),
        delegate: delegate.clone(),
        created_at: delegation.created_at,
    }
    .publish(env);
    Ok(())
}

pub fn revoke_delegation(
    env: &Env,
    delegator: &Address,
    scope: &DelegationScope,
) -> Result<(), ContractError> {
    delegator.require_auth();
    let mut delegation = get_raw(env, delegator, scope).ok_or(ContractError::InvalidState)?;
    delegation.revoked = true;
    set_raw(env, &delegation);
    DelegationRevoked {
        delegator: delegator.clone(),
        revoked_at: env.ledger().timestamp(),
    }
    .publish(env);
    Ok(())
}

pub fn is_authorized_proxy(env: &Env, delegator: &Address, proxy: &Address, grant_id: u64) -> bool {
    active_matching(env, delegator, grant_id)
        .map(|d| d.delegate == *proxy)
        .unwrap_or(false)
}

pub fn resolve_delegator(env: &Env, voter: &Address, grant_id: u64) -> Address {
    let grant = match Storage::get_grant(env, grant_id) {
        Some(g) => g,
        None => return voter.clone(),
    };
    for reviewer in grant.reviewers.iter() {
        if is_authorized_proxy(env, &reviewer, voter, grant_id) {
            return reviewer;
        }
    }
    voter.clone()
}

pub fn consume_delegation_use(
    env: &Env,
    delegator: &Address,
    scope: &DelegationScope,
) -> Result<(), ContractError> {
    let mut delegation = get_raw(env, delegator, scope).ok_or(ContractError::InvalidState)?;
    if let Some(remaining) = delegation.uses_remaining {
        let next = remaining.saturating_sub(1);
        delegation.uses_remaining = Some(next);
        if next == 0 {
            delegation.revoked = true;
        }
        set_raw(env, &delegation);
    }
    Ok(())
}

pub fn consume_delegation_for_vote(
    env: &Env,
    delegator: &Address,
    proxy: &Address,
    grant_id: u64,
) -> Result<(), ContractError> {
    if let Some(d) = active_matching(env, delegator, grant_id) {
        if d.delegate == *proxy {
            return consume_delegation_use(env, delegator, &d.scope);
        }
    }
    Err(ContractError::Unauthorized)
}

pub fn get_delegation(env: &Env, delegator: &Address, scope: &DelegationScope) -> Option<Delegation> {
    active_for_scope(env, delegator, scope)
}
