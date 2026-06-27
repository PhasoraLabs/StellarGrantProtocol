use crate::storage::keys::{DataKey, ProvenanceKey};
use crate::types::{ContributionType, ProvenanceRecord};
use soroban_sdk::{Address, Bytes, Env, Vec};

const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_000_000;

/// Record a new provenance entry. Append-only, never fails.
pub fn record(
    env: &Env,
    contribution_type: ContributionType,
    actor: &Address,
    grant_id: u64,
    milestone_idx: Option<u32>,
    amount: Option<i128>,
    token: Option<Address>,
    co_contributors: Vec<Address>,
) {
    let counter_key = DataKey::Provenance(ProvenanceKey::Counter);
    let mut counter: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);
    counter += 1;

    let record_id = counter;
    let timestamp = env.ledger().timestamp();
    let ledger_sequence = env.ledger().sequence();
    let tags = Vec::new(env);

    let record = ProvenanceRecord {
        id: record_id,
        contribution_type,
        actor: actor.clone(),
        grant_id,
        milestone_idx,
        amount,
        token,
        timestamp,
        ledger_sequence,
        co_contributors,
        tags,
    };

    // Store record by ID
    let record_key = DataKey::Provenance(ProvenanceKey::Record(record_id));
    env.storage().persistent().set(&record_key, &record);
    env.storage().persistent().extend_ttl(
        &record_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );

    // Update counter
    env.storage().persistent().set(&counter_key, &counter);

    // Update per-address index
    let index_key = DataKey::Provenance(ProvenanceKey::Index(actor.clone()));
    let mut address_records: Vec<u32> = env
        .storage()
        .persistent()
        .get(&index_key)
        .unwrap_or_else(|| Vec::new(env));
    address_records.push_back(record_id);
    env.storage().persistent().set(&index_key, &address_records);
    env.storage().persistent().extend_ttl(
        &index_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );

    // Update by-grant index
    let grant_key = DataKey::Provenance(ProvenanceKey::ByGrant(grant_id));
    let mut grant_records: Vec<u32> = env
        .storage()
        .persistent()
        .get(&grant_key)
        .unwrap_or_else(|| Vec::new(env));
    grant_records.push_back(record_id);
    env.storage().persistent().set(&grant_key, &grant_records);
    env.storage().persistent().extend_ttl(
        &grant_key,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );
}

/// Return all provenance records for an address, paginated.
pub fn get_by_address(
    env: &Env,
    address: &Address,
    offset: u32,
    limit: u32,
) -> Vec<ProvenanceRecord> {
    let index_key = DataKey::Provenance(ProvenanceKey::Index(address.clone()));
    let record_ids: Vec<u32> = env
        .storage()
        .persistent()
        .get(&index_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut result = Vec::new(env);
    let len = record_ids.len() as u32;
    let end = if offset + limit > len {
        len
    } else {
        offset + limit
    };

    if offset < len {
        for i in offset..end {
            if let Some(record_id) = record_ids.get(i) {
                if let Some(record) = get_record(env, record_id) {
                    result.push_back(record);
                }
            }
        }
    }

    result
}

/// Return a specific provenance record by global ID.
pub fn get_record(env: &Env, record_id: u32) -> Option<ProvenanceRecord> {
    let key = DataKey::Provenance(ProvenanceKey::Record(record_id));
    env.storage().persistent().get(&key)
}

/// Return all provenance records for a grant.
pub fn get_by_grant(env: &Env, grant_id: u64) -> Vec<ProvenanceRecord> {
    let grant_key = DataKey::Provenance(ProvenanceKey::ByGrant(grant_id));
    let record_ids: Vec<u32> = env
        .storage()
        .persistent()
        .get(&grant_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut result = Vec::new(env);
    for i in 0..record_ids.len() {
        if let Some(record_id) = record_ids.get(i) {
            if let Some(record) = get_record(env, record_id) {
                result.push_back(record);
            }
        }
    }

    result
}

/// Return total number of provenance records.
pub fn total_records(env: &Env) -> u32 {
    let counter_key = DataKey::Provenance(ProvenanceKey::Counter);
    env.storage().persistent().get(&counter_key).unwrap_or(0)
}

/// Compute a cryptographic proof-of-contribution hash for a specific record.
/// hash = SHA-256(grant_id || milestone_idx || amount || timestamp || record_id)
pub fn proof_hash(env: &Env, record_id: u32) -> Option<Bytes> {
    let record = get_record(env, record_id)?;

    let mut data = Bytes::new(env);

    // Add grant_id (8 bytes, little-endian)
    data.append(&Bytes::from_slice(env, &record.grant_id.to_le_bytes()));

    // Add milestone_idx if present (4 bytes)
    if let Some(idx) = record.milestone_idx {
        data.append(&Bytes::from_slice(env, &idx.to_le_bytes()));
    }

    // Add amount if present (16 bytes)
    if let Some(amt) = record.amount {
        data.append(&Bytes::from_slice(env, &amt.to_le_bytes()));
    }

    // Add timestamp (8 bytes)
    data.append(&Bytes::from_slice(env, &record.timestamp.to_le_bytes()));

    // Add record ID for uniqueness (4 bytes)
    data.append(&Bytes::from_slice(env, &record.id.to_le_bytes()));

    // Compute SHA-256 hash
    Some(env.crypto().sha256(&data).into())
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::types::ContributionType;

    #[test]
    fn test_record_append_and_retrieve() {
        let env = soroban_sdk::Env::default();
        let actor = Address::random(&env);
        let co_contributors = Vec::new(&env);

        record(
            &env,
            ContributionType::GrantCreated,
            &actor,
            1,
            None,
            Some(1000),
            None,
            co_contributors,
        );

        let total = total_records(&env);
        assert_eq!(total, 1);

        let record = get_record(&env, 1).expect("Record should exist");
        assert_eq!(record.id, 1);
        assert_eq!(record.grant_id, 1);
        assert_eq!(record.contribution_type, ContributionType::GrantCreated);
    }

    #[test]
    fn test_address_index_updated() {
        let env = soroban_sdk::Env::default();
        let actor = Address::random(&env);
        let co_contributors = Vec::new(&env);

        record(
            &env,
            ContributionType::GrantCreated,
            &actor,
            1,
            None,
            Some(1000),
            None,
            co_contributors.clone(),
        );

        record(
            &env,
            ContributionType::MilestoneDelivered,
            &actor,
            1,
            Some(0),
            Some(500),
            None,
            co_contributors,
        );

        let records = get_by_address(&env, &actor, 0, 10);
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_get_by_grant() {
        let env = soroban_sdk::Env::default();
        let actor1 = Address::random(&env);
        let actor2 = Address::random(&env);
        let co_contributors = Vec::new(&env);

        record(
            &env,
            ContributionType::GrantCreated,
            &actor1,
            1,
            None,
            Some(1000),
            None,
            co_contributors.clone(),
        );

        record(
            &env,
            ContributionType::MilestoneReviewed,
            &actor2,
            1,
            Some(0),
            None,
            None,
            co_contributors,
        );

        let records = get_by_grant(&env, 1);
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_proof_hash_consistency() {
        let env = soroban_sdk::Env::default();
        let actor = Address::random(&env);
        let co_contributors = Vec::new(&env);

        record(
            &env,
            ContributionType::GrantCreated,
            &actor,
            1,
            None,
            Some(1000),
            None,
            co_contributors,
        );

        let hash1 = proof_hash(&env, 1).expect("Hash should be generated");
        let hash2 = proof_hash(&env, 1).expect("Hash should be generated");

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_proof_hash_changes_with_different_data() {
        let env = soroban_sdk::Env::default();
        let actor = Address::random(&env);
        let co_contributors = Vec::new(&env);

        record(
            &env,
            ContributionType::GrantCreated,
            &actor,
            1,
            None,
            Some(1000),
            None,
            co_contributors.clone(),
        );

        record(
            &env,
            ContributionType::GrantFunded,
            &actor,
            2,
            None,
            Some(2000),
            None,
            co_contributors,
        );

        let hash1 = proof_hash(&env, 1).expect("Hash should be generated");
        let hash2 = proof_hash(&env, 2).expect("Hash should be generated");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_pagination() {
        let env = soroban_sdk::Env::default();
        let actor = Address::random(&env);
        let co_contributors = Vec::new(&env);

        for i in 0..5 {
            record(
                &env,
                ContributionType::GrantCreated,
                &actor,
                i as u64,
                None,
                Some(1000),
                None,
                co_contributors.clone(),
            );
        }

        let page1 = get_by_address(&env, &actor, 0, 2);
        let page2 = get_by_address(&env, &actor, 2, 2);
        let page3 = get_by_address(&env, &actor, 4, 2);

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);
    }
}
