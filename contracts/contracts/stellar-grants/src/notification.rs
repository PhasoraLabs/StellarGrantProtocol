use soroban_sdk::{Address, Env, Vec};

use crate::constants;
use crate::errors::ContractError;
use crate::storage::DataKey;
use crate::types::{NotificationEvent, Subscription, SubscriptionScope};

fn scope_type_and_data(scope: &SubscriptionScope) -> (u32, u128) {
    match scope {
        SubscriptionScope::Global => (0, 0),
        SubscriptionScope::PerGrant(id) => (1, *id as u128),
        SubscriptionScope::PerContributor(_addr) => (2, 0),
        SubscriptionScope::PerTag(tag_hash) => (3, *tag_hash),
    }
}

pub fn subscribe(
    env: &Env,
    subscriber: &Address,
    event: NotificationEvent,
    scope: SubscriptionScope,
) -> Result<(), ContractError> {
    let existing = get_subscriptions(env, subscriber);
    if existing.len() >= constants::MAX_SUBSCRIPTIONS_PER_ADDRESS {
        return Err(ContractError::InvalidInput);
    }

    for sub in existing.iter() {
        if sub.event == event && sub.scope == scope {
            return Ok(());
        }
    }

    let sub = Subscription {
        subscriber: subscriber.clone(),
        event: event.clone(),
        scope: scope.clone(),
        subscribed_at: env.ledger().timestamp(),
        is_active: true,
    };

    let mut subs: Vec<Subscription> = env
        .storage()
        .persistent()
        .get(&DataKey::NotifSub(subscriber.clone(), 0, 0, 0))
        .unwrap_or_else(|| Vec::new(env));
    subs.push_back(sub);
    env.storage()
        .persistent()
        .set(&DataKey::NotifSub(subscriber.clone(), 0, 0, 0), &subs);

    let (scope_type, _scope_data) = scope_type_and_data(&scope);
    let list_key = DataKey::NotifSubList(event as u32, scope_type, scope.clone());
    let mut list: Vec<Address> = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));
    if !list.contains(subscriber.clone()) {
        list.push_back(subscriber.clone());
        env.storage().persistent().set(&list_key, &list);
    }

    Ok(())
}

pub fn unsubscribe(
    env: &Env,
    subscriber: &Address,
    event: NotificationEvent,
    scope: &SubscriptionScope,
) -> Result<(), ContractError> {
    let mut subs: Vec<Subscription> = env
        .storage()
        .persistent()
        .get(&DataKey::NotifSub(subscriber.clone(), 0, 0, 0))
        .unwrap_or_else(|| Vec::new(env));

    if let Some(pos) = (0..subs.len()).find(|&i| {
        if let Some(s) = subs.get(i) {
            s.event == event && s.scope == *scope
        } else {
            false
        }
    }) {
        subs.remove(pos);
    }

    env.storage()
        .persistent()
        .set(&DataKey::NotifSub(subscriber.clone(), 0, 0, 0), &subs);

    let (scope_type, _scope_data) = scope_type_and_data(scope);
    let list_key = DataKey::NotifSubList(event as u32, scope_type, scope.clone());
    let mut list: Vec<Address> = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));
    if let Some(pos) = (0..list.len()).find(|&i| list.get(i) == Some(subscriber.clone())) {
        list.remove(pos);
        env.storage().persistent().set(&list_key, &list);
    }

    Ok(())
}

pub fn get_subscriptions(env: &Env, subscriber: &Address) -> Vec<Subscription> {
    env.storage()
        .persistent()
        .get(&DataKey::NotifSub(subscriber.clone(), 0, 0, 0))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_subscribers(
    env: &Env,
    event: NotificationEvent,
    scope: &SubscriptionScope,
) -> Vec<Address> {
    let (scope_type, _scope_data) = scope_type_and_data(scope);
    let list_key = DataKey::NotifSubList(event as u32, scope_type, scope.clone());
    env.storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn emit_notification(
    env: &Env,
    event: NotificationEvent,
    scope: &SubscriptionScope,
    payload: u128,
) {
    let (scope_type, scope_data) = scope_type_and_data(scope);
    env.events().publish(
        (
            soroban_sdk::Symbol::new(env, "notification"),
            event as u32,
            scope_type,
            scope_data,
        ),
        payload,
    );
}

pub fn is_subscribed(
    env: &Env,
    subscriber: &Address,
    event: &NotificationEvent,
    scope: &SubscriptionScope,
) -> bool {
    let subs: Vec<Subscription> = env
        .storage()
        .persistent()
        .get(&DataKey::NotifSub(subscriber.clone(), 0, 0, 0))
        .unwrap_or_else(|| Vec::new(env));
    for s in subs.iter() {
        if s.event == *event && s.scope == *scope && s.is_active {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn subscription_lists_are_isolated_by_scope() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            let subscriber_a = Address::generate(&env);
            let subscriber_b = Address::generate(&env);
            let contributor_a = Address::generate(&env);
            let contributor_b = Address::generate(&env);

            subscribe(
                &env,
                &subscriber_a,
                NotificationEvent::MilestoneApproved,
                SubscriptionScope::PerGrant(1),
            )
            .unwrap();
            subscribe(
                &env,
                &subscriber_b,
                NotificationEvent::MilestoneApproved,
                SubscriptionScope::PerGrant(2),
            )
            .unwrap();
            subscribe(
                &env,
                &subscriber_a,
                NotificationEvent::NewGrant,
                SubscriptionScope::PerContributor(contributor_a.clone()),
            )
            .unwrap();
            subscribe(
                &env,
                &subscriber_b,
                NotificationEvent::NewGrant,
                SubscriptionScope::PerContributor(contributor_b.clone()),
            )
            .unwrap();

            assert_eq!(
                get_subscribers(
                    &env,
                    NotificationEvent::MilestoneApproved,
                    &SubscriptionScope::PerGrant(1),
                ),
                Vec::from_array(&env, [subscriber_a.clone()])
            );
            assert_eq!(
                get_subscribers(
                    &env,
                    NotificationEvent::NewGrant,
                    &SubscriptionScope::PerContributor(contributor_a),
                ),
                Vec::from_array(&env, [subscriber_a])
            );
        });
    }
}
