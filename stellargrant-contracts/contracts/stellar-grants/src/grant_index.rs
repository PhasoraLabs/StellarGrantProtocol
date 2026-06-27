use soroban_sdk::{Address, Env, Vec};

use crate::storage::{DataKey, GrantKey};
use crate::types::GrantStatus;

const INDEX_BUCKET_LIMIT: u32 = 10_000;

fn push_to_index(env: &Env, key: &DataKey, grant_id: u64) {
    let mut list: Vec<u64> = env
        .storage()
        .persistent()
        .get(key)
        .unwrap_or_else(|| Vec::new(env));
    if list.len() < INDEX_BUCKET_LIMIT && !list.contains(grant_id) {
        list.push_back(grant_id);
        env.storage().persistent().set(key, &list);
    }
}

fn remove_from_index(env: &Env, key: &DataKey, grant_id: u64) {
    let mut list: Vec<u64> = env
        .storage()
        .persistent()
        .get(key)
        .unwrap_or_else(|| Vec::new(env));
    if let Some(pos) = (0..list.len()).find(|&i| list.get(i) == Some(grant_id)) {
        list.remove(pos);
        env.storage().persistent().set(key, &list);
    }
}

pub fn on_grant_created(
    env: &Env,
    grant_id: u64,
    owner: &Address,
    token: &Address,
    status: GrantStatus,
) {
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::OwnerIndex(owner.clone())),
        grant_id,
    );
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::StatusIndex(status as u32)),
        grant_id,
    );
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::TokenIndex(token.clone())),
        grant_id,
    );
    let order_key = DataKey::Grant(GrantKey::GlobalOrder);
    let mut order: Vec<u64> = env
        .storage()
        .persistent()
        .get(&order_key)
        .unwrap_or_else(|| Vec::new(env));
    if order.len() < INDEX_BUCKET_LIMIT {
        order.push_back(grant_id);
        env.storage().persistent().set(&order_key, &order);
    }
}

pub fn on_status_changed(
    env: &Env,
    grant_id: u64,
    old_status: GrantStatus,
    new_status: GrantStatus,
) {
    if old_status != new_status {
        remove_from_index(
            env,
            &DataKey::Grant(GrantKey::StatusIndex(old_status as u32)),
            grant_id,
        );
        push_to_index(
            env,
            &DataKey::Grant(GrantKey::StatusIndex(new_status as u32)),
            grant_id,
        );
    }
}

pub fn on_contributor_assigned(env: &Env, grant_id: u64, contributor: &Address) {
    push_to_index(
        env,
        &DataKey::Grant(GrantKey::ContribIndex(contributor.clone())),
        grant_id,
    );
}

pub fn by_owner(env: &Env, owner: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::OwnerIndex(owner.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn by_status(env: &Env, status: GrantStatus, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::StatusIndex(status as u32)))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn by_token(env: &Env, token: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::TokenIndex(token.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn by_contributor(env: &Env, contributor: &Address, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::ContribIndex(contributor.clone())))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn recent(env: &Env, offset: u32, limit: u32) -> Vec<u64> {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::GlobalOrder))
        .unwrap_or_else(|| Vec::new(env));
    crate::pagination::paginate(env, &list, offset, limit)
}

pub fn index_counts(env: &Env, owner: Option<&Address>) -> (u32, u32, u32) {
    let owned = if let Some(o) = owner {
        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::OwnerIndex(o.clone())))
            .unwrap_or_else(|| Vec::new(env));
        list.len()
    } else {
        0
    };
    let active: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::Grant(GrantKey::StatusIndex(
            GrantStatus::Active as u32,
        )))
        .unwrap_or_else(|| Vec::new(env));
    let contributed = if let Some(o) = owner {
        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::ContribIndex(o.clone())))
            .unwrap_or_else(|| Vec::new(env));
        list.len()
    } else {
        0
    };
    (owned, active.len(), contributed)
}
