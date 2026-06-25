use crate::storage::Storage;
use crate::types::{
    ContractError, ReviewerAvailability, ReviewerProfile, ReviewerRequest, ReviewerRequestStatus,
};
use soroban_sdk::{Address, Env, String, Vec};

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
        expertise_tags,
        hourly_rate,
        reviews_completed: 0,
        average_turnaround_ledgers: 0,
        availability: ReviewerAvailability::Available,
        registered_at: env.ledger().timestamp(),
        reputation_score: Storage::get_reviewer_reputation(&env, reviewer.clone()),
    };

    Storage::set_reviewer_profile(&env, &profile);
    Ok(())
}

pub fn set_availability(
    env: &Env,
    reviewer: &Address,
    availability: ReviewerAvailability,
) -> Result<(), ContractError> {
    reviewer.require_auth();

    let mut profile =
        Storage::get_reviewer_profile(&env, reviewer).ok_or(ContractError::InvalidState)?;

    profile.availability = availability;
    Storage::set_reviewer_profile(&env, &profile);
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

    if !Storage::has_grant(&env, grant_id) {
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

    Storage::set_reviewer_request(&env, &request);
    Ok(())
}

pub fn accept_request(env: &Env, reviewer: &Address, grant_id: u64) -> Result<(), ContractError> {
    reviewer.require_auth();

    let request = Storage::get_reviewer_request(&env, grant_id, reviewer)
        .ok_or(ContractError::InvalidState)?;

    if request.status != ReviewerRequestStatus::Pending {
        return Err(ContractError::InvalidState);
    }

    if env.ledger().timestamp() > request.expires_at {
        return Err(ContractError::InvalidState);
    }

    let profile =
        Storage::get_reviewer_profile(&env, reviewer).ok_or(ContractError::InvalidState)?;

    if profile.availability != ReviewerAvailability::Available {
        return Err(ContractError::InvalidState);
    }

    let mut grant = Storage::get_grant_v(&env, grant_id);
    grant.reviewers.push_back(reviewer.clone());
    Storage::set_grant(&env, grant_id, &grant);

    let mut updated_request = request;
    updated_request.status = ReviewerRequestStatus::Accepted;
    Storage::set_reviewer_request(&env, &updated_request);
    Ok(())
}

pub fn decline_request(env: &Env, reviewer: &Address, grant_id: u64) -> Result<(), ContractError> {
    reviewer.require_auth();

    let request = Storage::get_reviewer_request(&env, grant_id, reviewer)
        .ok_or(ContractError::InvalidState)?;

    if request.status != ReviewerRequestStatus::Pending {
        return Err(ContractError::InvalidState);
    }

    let mut updated_request = request;
    updated_request.status = ReviewerRequestStatus::Declined;
    Storage::set_reviewer_request(&env, &updated_request);
    Ok(())
}

pub fn find_by_tag(_env: &Env, _tag: &String, _limit: u32) -> Vec<ReviewerProfile> {
    Vec::new(_env)
}

pub fn get_profile(env: &Env, reviewer: &Address) -> Option<ReviewerProfile> {
    Storage::get_reviewer_profile(&env, reviewer)
}

pub fn get_request(env: &Env, grant_id: u64, reviewer: &Address) -> Option<ReviewerRequest> {
    Storage::get_reviewer_request(&env, grant_id, reviewer)
}
