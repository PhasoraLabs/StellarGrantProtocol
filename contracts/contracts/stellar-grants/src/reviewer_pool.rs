use crate::config;
use crate::constants::DEFAULT_REVIEWER_SLA_SECONDS;
use crate::reviewer_sla;
use crate::storage::Storage;
use crate::types::{
    ContractError, ReviewerAvailability, ReviewerProfile, ReviewerRequest, ReviewerRequestStatus,
    WhitelistScope,
};
use crate::whitelist;
use soroban_sdk::{Address, Env, String, Vec};

/// Upper bound on results returned by [`find_by_tag`], to keep the scan bounded.
const MAX_TAG_QUERY_RESULTS: u32 = 50;

/// Drop `reviewer` from the index of every tag in `tags`.
fn unindex_tags(env: &Env, reviewer: &Address, tags: &Vec<String>) {
    for tag in tags.iter() {
        let index = Storage::get_reviewer_tag_index(env, &tag);
        let mut filtered = Vec::new(env);
        for indexed in index.iter() {
            if indexed != *reviewer {
                filtered.push_back(indexed);
            }
        }
        Storage::set_reviewer_tag_index(env, &tag, &filtered);
    }
}

/// Add `reviewer` to the index of every tag in `tags`, skipping duplicates.
fn index_tags(env: &Env, reviewer: &Address, tags: &Vec<String>) {
    for tag in tags.iter() {
        let mut index = Storage::get_reviewer_tag_index(env, &tag);
        if index.iter().any(|indexed| indexed == *reviewer) {
            continue;
        }
        index.push_back(reviewer.clone());
        Storage::set_reviewer_tag_index(env, &tag, &index);
    }
}

pub fn register_reviewer(
    env: &Env,
    reviewer: &Address,
    display_name: String,
    expertise_tags: Vec<String>,
    hourly_rate: Option<i128>,
) -> Result<(), ContractError> {
    reviewer.require_auth();

    let profile = ReviewerProfile {
        reviewer: reviewer.clone(),
        display_name,
        expertise_tags: expertise_tags.clone(),
        hourly_rate,
        reviews_completed: 0,
        average_turnaround_ledgers: 0,
        availability: ReviewerAvailability::Available,
        registered_at: env.ledger().timestamp(),
        reputation_score: Storage::get_reviewer_reputation(env, reviewer.clone()),
    };

    // Re-registering replaces the tag set, so retire the previous entries first.
    if let Some(existing) = Storage::get_reviewer_profile(env, reviewer) {
        unindex_tags(env, reviewer, &existing.expertise_tags);
    }

    Storage::set_reviewer_profile(env, &profile);
    index_tags(env, reviewer, &expertise_tags);
    Ok(())
}

pub fn set_availability(
    env: &Env,
    reviewer: &Address,
    availability: ReviewerAvailability,
) -> Result<(), ContractError> {
    reviewer.require_auth();

    let mut profile =
        Storage::get_reviewer_profile(env, reviewer).ok_or(ContractError::InvalidState)?;

    profile.availability = availability;
    Storage::set_reviewer_profile(env, &profile);
    Ok(())
}

pub fn request_reviewer(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    reviewer: &Address,
    message: String,
    ttl_ledgers: u32,
) -> Result<(), ContractError> {
    owner.require_auth();

    if !Storage::has_grant(env, grant_id) {
        return Err(ContractError::GrantNotFound);
    }

    let request = ReviewerRequest {
        grant_id,
        reviewer: reviewer.clone(),
        requested_by: owner.clone(),
        message,
        status: ReviewerRequestStatus::Pending,
        requested_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (ttl_ledgers as u64 * 5),
    };

    Storage::set_reviewer_request(env, &request);
    Ok(())
}

pub fn accept_request(env: &Env, reviewer: &Address, grant_id: u64) -> Result<(), ContractError> {
    reviewer.require_auth();

    let request = Storage::get_reviewer_request(env, grant_id, reviewer)
        .ok_or(ContractError::InvalidState)?;

    if request.status != ReviewerRequestStatus::Pending {
        return Err(ContractError::InvalidState);
    }

    if env.ledger().timestamp() > request.expires_at {
        return Err(ContractError::InvalidState);
    }

    let profile =
        Storage::get_reviewer_profile(env, reviewer).ok_or(ContractError::InvalidState)?;

    if profile.availability != ReviewerAvailability::Available {
        return Err(ContractError::InvalidState);
    }

    if !whitelist::is_allowed(env, reviewer, &WhitelistScope::GlobalReviewer) {
        return Err(ContractError::AddressNotWhitelisted);
    }

    let mut grant = Storage::get_grant_v(env, grant_id);
    if grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::AlreadyRegistered);
    }

    let protocol_cfg = config::get_config(env);
    if grant.reviewers.len() >= protocol_cfg.max_reviewers {
        return Err(ContractError::ReviewerLimitExceeded);
    }

    grant.reviewers.push_back(reviewer.clone());
    Storage::set_grant(env, grant_id, &grant);

    // Issue #611: accepting an assignment starts the review clock. Every
    // milestone the reviewer is now on the hook for gets its own SLA record;
    // missing the deadline forfeits reward accrual for that milestone.
    let deadline = env
        .ledger()
        .timestamp()
        .saturating_add(DEFAULT_REVIEWER_SLA_SECONDS);
    for milestone_idx in 0..grant.total_milestones {
        reviewer_sla::register_sla(
            env,
            reviewer,
            reviewer_sla::milestone_sla_id(grant_id, milestone_idx),
            deadline,
        );
    }

    let mut updated_request = request;
    updated_request.status = ReviewerRequestStatus::Accepted;
    Storage::set_reviewer_request(env, &updated_request);
    Ok(())
}

pub fn decline_request(env: &Env, reviewer: &Address, grant_id: u64) -> Result<(), ContractError> {
    reviewer.require_auth();

    let request = Storage::get_reviewer_request(env, grant_id, reviewer)
        .ok_or(ContractError::InvalidState)?;

    if request.status != ReviewerRequestStatus::Pending {
        return Err(ContractError::InvalidState);
    }

    let mut updated_request = request;
    updated_request.status = ReviewerRequestStatus::Declined;
    Storage::set_reviewer_request(env, &updated_request);
    Ok(())
}

/// Return up to `limit` reviewer profiles whose `expertise_tags` contain `tag`.
///
/// Backed by the per-tag index maintained by [`register_reviewer`]; matching is
/// an exact, case-sensitive comparison on the tag string.
pub fn find_by_tag(env: &Env, tag: &String, limit: u32) -> Vec<ReviewerProfile> {
    let limit = limit.min(MAX_TAG_QUERY_RESULTS);
    let mut result = Vec::new(env);
    if limit == 0 {
        return result;
    }

    for reviewer in Storage::get_reviewer_tag_index(env, tag).iter() {
        if let Some(profile) = Storage::get_reviewer_profile(env, &reviewer) {
            // The index is authoritative, but guard against a stale entry.
            if profile.expertise_tags.iter().any(|t| t == *tag) {
                result.push_back(profile);
                if result.len() >= limit {
                    break;
                }
            }
        }
    }

    result
}

pub fn get_profile(env: &Env, reviewer: &Address) -> Option<ReviewerProfile> {
    Storage::get_reviewer_profile(env, reviewer)
}

pub fn get_request(env: &Env, grant_id: u64, reviewer: &Address) -> Option<ReviewerRequest> {
    Storage::get_reviewer_request(env, grant_id, reviewer)
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::types::WhitelistMode;
    use crate::{StellarGrantsContract, StellarGrantsContractClient};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::vec;

    fn tag(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    fn setup<'a>() -> (Env, Address, StellarGrantsContractClient<'a>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(&env, &contract_id);
        (env, contract_id, client)
    }

    fn register(
        env: &Env,
        client: &StellarGrantsContractClient<'_>,
        name: &str,
        tags: Vec<String>,
    ) -> Address {
        let reviewer = Address::generate(env);
        client.reviewer_register(&reviewer, &String::from_str(env, name), &tags, &None);
        reviewer
    }

    fn find(
        env: &Env,
        contract_id: &Address,
        tag_val: &String,
        limit: u32,
    ) -> Vec<ReviewerProfile> {
        env.as_contract(contract_id, || find_by_tag(env, tag_val, limit))
    }

    fn addresses(profiles: &Vec<ReviewerProfile>) -> soroban_sdk::Vec<Address> {
        let mut out = soroban_sdk::Vec::new(profiles.env());
        for p in profiles.iter() {
            out.push_back(p.reviewer);
        }
        out
    }

    #[test]
    fn test_find_by_tag_returns_only_reviewers_with_that_tag() {
        let (env, contract_id, client) = setup();

        let rustacean = register(
            &env,
            &client,
            "Rustacean",
            vec![&env, tag(&env, "rust"), tag(&env, "security")],
        );
        let designer = register(&env, &client, "Designer", vec![&env, tag(&env, "design")]);
        let auditor = register(
            &env,
            &client,
            "Auditor",
            vec![&env, tag(&env, "security"), tag(&env, "solidity")],
        );

        let rust_hits = find(&env, &contract_id, &tag(&env, "rust"), 10);
        assert_eq!(addresses(&rust_hits), vec![&env, rustacean.clone()]);

        let security_hits = find(&env, &contract_id, &tag(&env, "security"), 10);
        assert_eq!(
            addresses(&security_hits),
            vec![&env, rustacean, auditor.clone()]
        );

        let design_hits = find(&env, &contract_id, &tag(&env, "design"), 10);
        assert_eq!(addresses(&design_hits), vec![&env, designer]);

        let solidity_hits = find(&env, &contract_id, &tag(&env, "solidity"), 10);
        assert_eq!(addresses(&solidity_hits), vec![&env, auditor]);
    }

    #[test]
    fn test_find_by_tag_returns_full_profiles() {
        let (env, contract_id, client) = setup();

        let reviewer = register(&env, &client, "Rustacean", vec![&env, tag(&env, "rust")]);

        let hits = find(&env, &contract_id, &tag(&env, "rust"), 10);
        assert_eq!(hits.len(), 1);
        let profile = hits.get(0).unwrap();
        assert_eq!(profile.reviewer, reviewer);
        assert_eq!(profile.display_name, String::from_str(&env, "Rustacean"));
        assert_eq!(profile.expertise_tags, vec![&env, tag(&env, "rust")]);
    }

    #[test]
    fn test_find_by_tag_unknown_tag_is_empty() {
        let (env, contract_id, client) = setup();
        register(&env, &client, "Rustacean", vec![&env, tag(&env, "rust")]);

        assert_eq!(find(&env, &contract_id, &tag(&env, "cobol"), 10).len(), 0);
        // Matching is exact and case-sensitive.
        assert_eq!(find(&env, &contract_id, &tag(&env, "Rust"), 10).len(), 0);
        assert_eq!(find(&env, &contract_id, &tag(&env, "rus"), 10).len(), 0);
    }

    #[test]
    fn test_find_by_tag_respects_limit() {
        let (env, contract_id, client) = setup();
        for i in 0..3 {
            let _ = i;
            register(&env, &client, "R", vec![&env, tag(&env, "rust")]);
        }

        assert_eq!(find(&env, &contract_id, &tag(&env, "rust"), 0).len(), 0);
        assert_eq!(find(&env, &contract_id, &tag(&env, "rust"), 2).len(), 2);
        assert_eq!(find(&env, &contract_id, &tag(&env, "rust"), 100).len(), 3);
    }

    #[test]
    fn test_reregistering_replaces_the_indexed_tags() {
        let (env, contract_id, client) = setup();
        let reviewer = register(&env, &client, "Rustacean", vec![&env, tag(&env, "rust")]);

        client.reviewer_register(
            &reviewer,
            &String::from_str(&env, "Rustacean"),
            &vec![&env, tag(&env, "design")],
            &None,
        );

        assert_eq!(find(&env, &contract_id, &tag(&env, "rust"), 10).len(), 0);
        assert_eq!(
            addresses(&find(&env, &contract_id, &tag(&env, "design"), 10)),
            vec![&env, reviewer]
        );
    }

    #[test]
    fn test_reregistering_same_tag_does_not_duplicate() {
        let (env, contract_id, client) = setup();
        let _reviewer = register(&env, &client, "Rustacean", vec![&env, tag(&env, "rust")]);

        client.reviewer_register(
            &_reviewer,
            &String::from_str(&env, "Rustacean v2"),
            &vec![&env, tag(&env, "rust")],
            &None,
        );

        assert_eq!(find(&env, &contract_id, &tag(&env, "rust"), 10).len(), 1);
    }

    #[test]
    fn test_accept_request_enforces_whitelist_dedup_and_cap() {
        let (env, contract_id, client) = setup();

        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let token = Address::generate(&env);

        env.as_contract(&contract_id, || {
            Storage::set_global_admin(&env, &admin);
            let mut cfg = crate::config::default_config();
            cfg.max_reviewers = 1;
            Storage::set_protocol_config(&env, &cfg);
        });

        let grant_id = client.grant_create(
            &owner,
            &String::from_str(&env, "Grant"),
            &String::from_str(&env, "Desc"),
            &token,
            &1_000,
            &1_000,
            &1,
            &vec![&env],
        );

        client.whitelist_set_mode(
            &admin,
            &WhitelistScope::GlobalReviewer,
            &WhitelistMode::Restricted,
        );

        // A non-whitelisted reviewer cannot accept, even with a valid pending request.
        let stranger = register(&env, &client, "Stranger", vec![&env]);
        client.reviewer_request(
            &owner,
            &grant_id,
            &stranger,
            &String::from_str(&env, "msg"),
            &1000,
        );
        assert_eq!(
            client
                .try_reviewer_accept_request(&stranger, &grant_id)
                .unwrap_err()
                .unwrap(),
            ContractError::AddressNotWhitelisted
        );

        let reviewer_a = register(&env, &client, "A", vec![&env]);
        let reviewer_b = register(&env, &client, "B", vec![&env]);
        client.whitelist_add(&admin, &reviewer_a, &WhitelistScope::GlobalReviewer);
        client.whitelist_add(&admin, &reviewer_b, &WhitelistScope::GlobalReviewer);

        client.reviewer_request(
            &owner,
            &grant_id,
            &reviewer_a,
            &String::from_str(&env, "msg"),
            &1000,
        );
        client.reviewer_accept_request(&reviewer_a, &grant_id);

        env.as_contract(&contract_id, || {
            assert_eq!(Storage::get_grant_v(&env, grant_id).reviewers.len(), 1);
        });

        // Dedup: re-requesting and re-accepting the same reviewer must not push a duplicate.
        client.reviewer_request(
            &owner,
            &grant_id,
            &reviewer_a,
            &String::from_str(&env, "again"),
            &1000,
        );
        assert_eq!(
            client
                .try_reviewer_accept_request(&reviewer_a, &grant_id)
                .unwrap_err()
                .unwrap(),
            ContractError::AlreadyRegistered
        );

        // Cap: max_reviewers is 1 and already reached, so a second distinct
        // whitelisted reviewer cannot be accepted.
        client.reviewer_request(
            &owner,
            &grant_id,
            &reviewer_b,
            &String::from_str(&env, "msg"),
            &1000,
        );
        assert_eq!(
            client
                .try_reviewer_accept_request(&reviewer_b, &grant_id)
                .unwrap_err()
                .unwrap(),
            ContractError::ReviewerLimitExceeded
        );
    }
}
