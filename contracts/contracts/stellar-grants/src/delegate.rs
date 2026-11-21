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

fn is_registered_reviewer(env: &Env, reviewer: &Address, scope: &DelegationScope) -> bool {
    match scope {
        DelegationScope::PerGrant(grant_id) => Storage::get_grant(env, *grant_id)
            .map(|g| g.reviewers.contains(reviewer.clone()))
            .unwrap_or(false),
        // Global scope has no single grant to check at creation time; the real
        // authorization boundary is enforced per-grant at vote time in
        // resolve_delegator/is_authorized_proxy, which only ever iterate one
        // grant's reviewer list.
        DelegationScope::Global => true,
    }
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

fn would_create_cycle(
    env: &Env,
    delegator: &Address,
    delegate: &Address,
    scope: &DelegationScope,
) -> bool {
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
    if !is_registered_reviewer(env, delegator, &scope) {
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

pub fn get_delegation(
    env: &Env,
    delegator: &Address,
    scope: &DelegationScope,
) -> Option<Delegation> {
    active_for_scope(env, delegator, scope)
}

/// Return every active delegation under which `delegate` may currently vote on
/// `grant_id`, one entry per delegating reviewer.
///
/// Each [`Delegation`] carries its `scope`, `expires_at` and `uses_remaining`,
/// so callers can see exactly what the delegate holds and for how long. For a
/// given reviewer a per-grant delegation takes precedence over a global one,
/// matching the resolution order used when a vote is cast. Revoked, expired and
/// exhausted delegations are never returned. Unknown grants yield an empty list.
pub fn get_delegate_permissions(env: &Env, delegate: &Address, grant_id: u64) -> Vec<Delegation> {
    let mut held = Vec::new(env);
    let Some(grant) = Storage::get_grant(env, grant_id) else {
        return held;
    };
    for reviewer in grant.reviewers.iter() {
        if let Some(d) = active_matching(env, &reviewer, grant_id) {
            if d.delegate == *delegate {
                held.push_back(d);
            }
        }
    }
    held
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantStatus};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::String;

    const GRANT_ID: u64 = 1;

    fn setup(reviewer_count: u32) -> (Env, soroban_sdk::Vec<Address>) {
        let env = Env::default();
        env.mock_all_auths();
        crate::test_utils::register_contract(&env);
        let mut reviewers = Vec::new(&env);
        for _ in 0..reviewer_count {
            reviewers.push_back(Address::generate(&env));
        }
        crate::test_utils::in_contract(&env, || {
            Storage::set_grant(
                &env,
                GRANT_ID,
                &Grant {
                    id: GRANT_ID,
                    owner: Address::generate(&env),
                    title: String::from_str(&env, "Delegates"),
                    description: String::from_str(&env, "Delegate permissions"),
                    token: Address::generate(&env),
                    status: GrantStatus::Active,
                    total_amount: 100,
                    milestone_amount: 100,
                    reviewers: reviewers.clone(),
                    total_milestones: 1,
                    milestones_paid_out: 0,
                    escrow_balance: 0,
                    funders: Vec::new(&env),
                    reason: None,
                    timestamp: 0,
                    require_compliance: None,
                },
            );
        });
        (env, reviewers)
    }

    fn delegate_to(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        scope: DelegationScope,
        expires_at: Option<u64>,
        max_uses: Option<u32>,
    ) {
        // One frame per call: `require_auth` may only be satisfied once per frame.
        crate::test_utils::in_contract(env, || {
            delegate_vote(env, delegator, delegate, scope, expires_at, max_uses).unwrap();
        });
    }

    fn permissions(env: &Env, delegate: &Address) -> Vec<Delegation> {
        crate::test_utils::in_contract(env, || get_delegate_permissions(env, delegate, GRANT_ID))
    }

    #[test]
    fn test_no_delegation_returns_empty() {
        let (env, _) = setup(1);
        let delegate = Address::generate(&env);
        assert!(permissions(&env, &delegate).is_empty());
    }

    #[test]
    fn test_unknown_grant_returns_empty() {
        let (env, _) = setup(1);
        let delegate = Address::generate(&env);
        let held =
            crate::test_utils::in_contract(&env, || get_delegate_permissions(&env, &delegate, 999));
        assert!(held.is_empty());
    }

    #[test]
    fn test_per_grant_delegation_is_reported_with_limits() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        env.ledger().set_timestamp(100);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::PerGrant(GRANT_ID),
            Some(500),
            Some(3),
        );

        let held = permissions(&env, &delegate);
        assert_eq!(held.len(), 1);
        let d = held.get(0).unwrap();
        assert_eq!(d.delegator, reviewer);
        assert_eq!(d.delegate, delegate);
        assert_eq!(d.scope, DelegationScope::PerGrant(GRANT_ID));
        assert_eq!(d.created_at, 100);
        assert_eq!(d.expires_at, Some(500));
        assert_eq!(d.uses_remaining, Some(3));
        assert!(!d.revoked);
    }

    #[test]
    fn test_global_delegation_applies_to_the_grant() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::Global,
            None,
            None,
        );

        let held = permissions(&env, &delegate);
        assert_eq!(held.len(), 1);
        assert_eq!(held.get(0).unwrap().scope, DelegationScope::Global);
    }

    #[test]
    fn test_per_grant_scope_takes_precedence_over_global() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::Global,
            None,
            None,
        );
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::PerGrant(GRANT_ID),
            None,
            Some(1),
        );

        let held = permissions(&env, &delegate);
        assert_eq!(held.len(), 1);
        assert_eq!(
            held.get(0).unwrap().scope,
            DelegationScope::PerGrant(GRANT_ID)
        );
    }

    #[test]
    fn test_only_the_named_delegate_holds_permissions() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        let bystander = Address::generate(&env);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::Global,
            None,
            None,
        );

        assert_eq!(permissions(&env, &delegate).len(), 1);
        assert!(permissions(&env, &bystander).is_empty());
        // The delegator itself is not a delegate of its own delegation.
        assert!(permissions(&env, &reviewer).is_empty());
    }

    #[test]
    fn test_delegations_from_multiple_reviewers_are_all_listed() {
        let (env, reviewers) = setup(2);
        let delegate = Address::generate(&env);
        for reviewer in reviewers.iter() {
            delegate_to(
                &env,
                &reviewer,
                &delegate,
                DelegationScope::Global,
                None,
                None,
            );
        }

        let held = permissions(&env, &delegate);
        assert_eq!(held.len(), 2);
        assert_eq!(held.get(0).unwrap().delegator, reviewers.get(0).unwrap());
        assert_eq!(held.get(1).unwrap().delegator, reviewers.get(1).unwrap());
    }

    #[test]
    fn test_revoked_delegation_is_not_reported() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::Global,
            None,
            None,
        );
        assert_eq!(permissions(&env, &delegate).len(), 1);

        crate::test_utils::in_contract(&env, || {
            revoke_delegation(&env, &reviewer, &DelegationScope::Global).unwrap();
        });
        assert!(permissions(&env, &delegate).is_empty());
    }

    #[test]
    fn test_expired_delegation_is_not_reported() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        env.ledger().set_timestamp(100);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::Global,
            Some(200),
            None,
        );

        env.ledger().set_timestamp(199);
        assert_eq!(permissions(&env, &delegate).len(), 1);
        env.ledger().set_timestamp(200);
        assert!(permissions(&env, &delegate).is_empty());
    }

    #[test]
    fn test_exhausted_delegation_is_not_reported() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::Global,
            None,
            Some(1),
        );
        assert_eq!(
            permissions(&env, &delegate).get(0).unwrap().uses_remaining,
            Some(1)
        );

        crate::test_utils::in_contract(&env, || {
            consume_delegation_use(&env, &reviewer, &DelegationScope::Global).unwrap();
        });
        assert!(permissions(&env, &delegate).is_empty());
    }

    #[test]
    fn test_per_grant_delegation_for_another_grant_is_not_reported() {
        let (env, reviewers) = setup(1);
        let reviewer = reviewers.get(0).unwrap();
        let delegate = Address::generate(&env);
        delegate_to(
            &env,
            &reviewer,
            &delegate,
            DelegationScope::PerGrant(GRANT_ID),
            None,
            None,
        );

        let other = crate::test_utils::in_contract(&env, || {
            get_delegate_permissions(&env, &delegate, GRANT_ID + 1)
        });
        assert!(other.is_empty());
    }
}
