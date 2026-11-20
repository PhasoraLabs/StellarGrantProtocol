use crate::events::Events;
use crate::storage::keys::DataKey;
use crate::storage::Storage;
use crate::types::{ContractError, EscrowReleaseApproval, EscrowReleaseRequest, ProtocolConfig};
use soroban_sdk::{Address, Env, Vec};

const SECONDS_PER_WEEK: u64 = 604800;

pub fn create_request(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
    amount: i128,
    recipient: Address,
) -> Result<(), ContractError> {
    let key = DataKey::EscrowReleaseRequest(grant_id, milestone_idx);
    let expires_at = env.ledger().timestamp() + (SECONDS_PER_WEEK * 2);
    let request = EscrowReleaseRequest {
        grant_id,
        milestone_idx,
        amount,
        recipient,
        approvals: Vec::new(env),
        expires_at,
        executed: false,
    };
    env.storage().persistent().set(&key, &request);
    Events::emit_escrow_multisig_request_created(env, grant_id, milestone_idx, amount);
    Ok(())
}

pub fn approve(
    env: &Env,
    approver: Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    approver.require_auth();

    // Issue #892: only the grant's owner, one of its registered reviewers, or
    // the global admin may cast a multisig approval — otherwise anyone
    // controlling two throwaway keypairs could meet the default threshold of 2.
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let admin = Storage::get_global_admin(env);
    let is_eligible = grant.owner == approver
        || grant.reviewers.contains(approver.clone())
        || admin == Some(approver.clone());
    if !is_eligible {
        return Err(ContractError::Unauthorized);
    }

    let mut request =
        get_request(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;

    if request.executed {
        return Err(ContractError::InvalidState);
    }
    if env.ledger().timestamp() > request.expires_at {
        return Err(ContractError::InvalidState);
    }

    for approval in request.approvals.iter() {
        if approval.approver == approver {
            return Err(ContractError::AlreadyVoted);
        }
    }

    request.approvals.push_back(EscrowReleaseApproval {
        approver: approver.clone(),
        timestamp: env.ledger().timestamp(),
    });
    let total_approvals = request.approvals.len();

    env.storage().persistent().set(
        &DataKey::EscrowReleaseRequest(grant_id, milestone_idx),
        &request,
    );
    Events::emit_escrow_multisig_approved(env, grant_id, milestone_idx, approver, total_approvals);
    Ok(())
}

pub fn is_approved(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    if let Some(request) = get_request(env, grant_id, milestone_idx) {
        if let Some(config) = env
            .storage()
            .persistent()
            .get::<DataKey, ProtocolConfig>(&DataKey::Config)
        {
            let threshold = config.multisig_escrow_threshold;
            return request.approvals.len() >= threshold;
        }
    }
    false
}

pub fn execute_release(env: &Env, grant_id: u64, milestone_idx: u32) -> Result<(), ContractError> {
    let mut request =
        get_request(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;

    if request.executed {
        return Err(ContractError::InvalidState);
    }
    if env.ledger().timestamp() > request.expires_at {
        return Err(ContractError::InvalidState);
    }

    if !is_approved(env, grant_id, milestone_idx) {
        return Err(ContractError::Unauthorized);
    }

    request.executed = true;
    env.storage().persistent().set(
        &DataKey::EscrowReleaseRequest(grant_id, milestone_idx),
        &request,
    );

    crate::escrow::release(env, grant_id, &request.recipient, request.amount)?;
    Events::emit_escrow_multisig_executed(env, grant_id, milestone_idx, request.amount);
    Ok(())
}

pub fn get_request(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<EscrowReleaseRequest> {
    env.storage()
        .persistent()
        .get(&DataKey::EscrowReleaseRequest(grant_id, milestone_idx))
}

/// True when a request exists, was never executed, and is past `expires_at`.
/// Such a request can no longer be approved or executed (Issue #893).
pub fn is_expired_unexecuted(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
    get_request(env, grant_id, milestone_idx)
        .map(|r| !r.executed && env.ledger().timestamp() > r.expires_at)
        .unwrap_or(false)
}

/// Remove an expired, unexecuted request so a fresh one can be created
/// (Issue #893). Fails while the request is still live or once executed.
pub fn clear_expired_request(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    let request = get_request(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;
    if request.executed || env.ledger().timestamp() <= request.expires_at {
        return Err(ContractError::InvalidState);
    }
    env.storage()
        .persistent()
        .remove(&DataKey::EscrowReleaseRequest(grant_id, milestone_idx));
    Ok(())
}

/// Replace an expired, unexecuted request with a fresh one carrying the same
/// amount and recipient but no approvals and a new expiry (Issue #893).
pub fn recreate_expired_request(
    env: &Env,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    let old = get_request(env, grant_id, milestone_idx).ok_or(ContractError::InvalidState)?;
    clear_expired_request(env, grant_id, milestone_idx)?;
    create_request(env, grant_id, milestone_idx, old.amount, old.recipient)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::EscrowAccount;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::token;

    fn setup() -> (Env, u64, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let grant_id = 1u64;

        env.as_contract(&contract_id, || {
            let mut config = crate::config::default_config();
            config.multisig_escrow_threshold = 2;
            env.storage().persistent().set(&DataKey::Config, &config);
        });

        // The threshold above is written to this instance's storage, so tests
        // have to run against the same `contract_id` instead of registering a
        // second instance, whose storage would look unconfigured.
        (env, grant_id, contract_id)
    }

    /// Issue #892: `approve` now requires the grant's owner/reviewers/admin,
    /// so every test that calls `approve` needs a real Grant record backing
    /// the eligibility check.
    fn setup_grant(
        env: &Env,
        contract_id: &Address,
        grant_id: u64,
        owner: &Address,
        reviewers: Vec<Address>,
    ) {
        env.as_contract(contract_id, || {
            let grant = crate::types::Grant {
                id: grant_id,
                owner: owner.clone(),
                title: soroban_sdk::String::from_str(env, "Title"),
                description: soroban_sdk::String::from_str(env, "Description"),
                token: Address::generate(env),
                status: crate::types::GrantStatus::Active,
                total_amount: 1000,
                milestone_amount: 1000,
                reviewers,
                total_milestones: 1,
                milestones_paid_out: 0,
                escrow_balance: 1000,
                funders: Vec::new(env),
                reason: None,
                timestamp: env.ledger().timestamp(),
                require_compliance: None,
            };
            Storage::set_grant(env, grant_id, &grant);
        });
    }

    #[test]
    fn test_create_request() {
        let (env, grant_id, contract_id) = setup();
        let recipient = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let result = create_request(&env, grant_id, 0, 1000, recipient.clone());
            assert!(result.is_ok());

            let request = get_request(&env, grant_id, 0).unwrap();
            assert_eq!(request.amount, 1000);
            assert_eq!(request.recipient, recipient);
            assert!(!request.executed);
            assert_eq!(request.approvals.len(), 0);
        });
    }

    #[test]
    fn test_approve_accumulates() {
        let (env, grant_id, contract_id) = setup();
        let owner = Address::generate(&env);
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let recipient = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(approver1.clone());
        reviewers.push_back(approver2.clone());
        setup_grant(&env, &contract_id, grant_id, &owner, reviewers);

        env.as_contract(&contract_id, || {
            create_request(&env, grant_id, 0, 1000, recipient).unwrap();

            approve(&env, approver1.clone(), grant_id, 0).unwrap();
            let request = get_request(&env, grant_id, 0).unwrap();
            assert_eq!(request.approvals.len(), 1);
            assert!(!is_approved(&env, grant_id, 0));

            approve(&env, approver2.clone(), grant_id, 0).unwrap();
            let request = get_request(&env, grant_id, 0).unwrap();
            assert_eq!(request.approvals.len(), 2);
            assert!(is_approved(&env, grant_id, 0));
        });
    }

    #[test]
    fn test_duplicate_approval_rejected() {
        let (env, grant_id, contract_id) = setup();
        let owner = Address::generate(&env);
        let approver = Address::generate(&env);
        let recipient = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(approver.clone());
        setup_grant(&env, &contract_id, grant_id, &owner, reviewers);

        env.as_contract(&contract_id, || {
            create_request(&env, grant_id, 0, 1000, recipient).unwrap();
            approve(&env, approver.clone(), grant_id, 0).unwrap();
        });

        // The duplicate lands in its own contract invocation: authorising the
        // same approver twice inside one frame is rejected by the host as a
        // re-authorized frame before `approve` gets to its duplicate check.
        env.as_contract(&contract_id, || {
            let result = approve(&env, approver, grant_id, 0);
            assert_eq!(result, Err(ContractError::AlreadyVoted));
        });
    }

    #[test]
    fn test_execute_before_threshold_rejected() {
        let (env, grant_id, contract_id) = setup();
        let owner = Address::generate(&env);
        let approver = Address::generate(&env);
        let recipient = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(approver.clone());
        setup_grant(&env, &contract_id, grant_id, &owner, reviewers);

        env.as_contract(&contract_id, || {
            create_request(&env, grant_id, 0, 1000, recipient).unwrap();
            approve(&env, approver, grant_id, 0).unwrap();

            // Only 1 approval, threshold is 2
            let result = execute_release(&env, grant_id, 0);
            assert_eq!(result, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn test_execute_already_executed_rejected() {
        let (env, grant_id, contract_id) = setup();
        let owner = Address::generate(&env);
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let recipient = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(approver1.clone());
        reviewers.push_back(approver2.clone());
        setup_grant(&env, &contract_id, grant_id, &owner, reviewers);

        // The first `execute_release` has to succeed for the second one to be
        // rejected as already executed, so the grant needs a funded escrow
        // account and a token contract the contract can actually pay out of.
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        token::StellarAssetClient::new(&env, &token_id).mint(&contract_id, &1000);

        env.as_contract(&contract_id, || {
            Storage::set_escrow_account(
                &env,
                grant_id,
                &EscrowAccount {
                    owner: Address::generate(&env),
                    token: token_id.clone(),
                    balance: 1000,
                    total_deposited: 1000,
                    total_released: 0,
                    locked: false,
                },
            );

            create_request(&env, grant_id, 0, 1000, recipient).unwrap();
            approve(&env, approver1, grant_id, 0).unwrap();
            approve(&env, approver2, grant_id, 0).unwrap();

            execute_release(&env, grant_id, 0).unwrap();

            // Second execution should fail
            let result = execute_release(&env, grant_id, 0);
            assert_eq!(result, Err(ContractError::InvalidState));
        });
    }

    #[test]
    fn test_approve_after_expiry_rejected() {
        let (env, grant_id, contract_id) = setup();
        let owner = Address::generate(&env);
        let approver = Address::generate(&env);
        let recipient = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(approver.clone());
        setup_grant(&env, &contract_id, grant_id, &owner, reviewers);

        env.as_contract(&contract_id, || {
            create_request(&env, grant_id, 0, 1000, recipient).unwrap();

            // Advance time past expiry (2 weeks)
            let mut ledger = env.ledger().get();
            ledger.timestamp += SECONDS_PER_WEEK * 3;
            env.ledger().set(ledger);

            let result = approve(&env, approver, grant_id, 0);
            assert_eq!(result, Err(ContractError::InvalidState));
        });
    }

    fn expire(env: &Env) {
        let mut ledger = env.ledger().get();
        ledger.timestamp += SECONDS_PER_WEEK * 3;
        env.ledger().set(ledger);
    }

    #[test]
    fn test_expired_request_can_be_cleared_and_recreated_then_executed() {
        let (env, grant_id, contract_id) = setup();
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let recipient = Address::generate(&env);
        let mut reviewers = Vec::new(&env);
        reviewers.push_back(approver1.clone());
        reviewers.push_back(approver2.clone());
        setup_grant(
            &env,
            &contract_id,
            grant_id,
            &Address::generate(&env),
            reviewers,
        );

        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        token::StellarAssetClient::new(&env, &token_id).mint(&contract_id, &1000);

        env.as_contract(&contract_id, || {
            Storage::set_escrow_account(
                &env,
                grant_id,
                &EscrowAccount {
                    owner: Address::generate(&env),
                    token: token_id.clone(),
                    balance: 1000,
                    total_deposited: 1000,
                    total_released: 0,
                    locked: false,
                },
            );
            create_request(&env, grant_id, 0, 1000, recipient.clone()).unwrap();
            approve(&env, approver1.clone(), grant_id, 0).unwrap();
        });

        // Stuck state: only 1 of 2 approvals and the request has expired.
        expire(&env);
        env.as_contract(&contract_id, || {
            assert!(is_expired_unexecuted(&env, grant_id, 0));
            assert_eq!(
                approve(&env, approver2.clone(), grant_id, 0),
                Err(ContractError::InvalidState)
            );
            assert_eq!(
                execute_release(&env, grant_id, 0),
                Err(ContractError::InvalidState)
            );

            recreate_expired_request(&env, grant_id, 0).unwrap();
            let fresh = get_request(&env, grant_id, 0).unwrap();
            assert_eq!(fresh.amount, 1000);
            assert_eq!(fresh.recipient, recipient);
            assert_eq!(fresh.approvals.len(), 0);
            assert!(!is_expired_unexecuted(&env, grant_id, 0));
        });

        // The fresh request can be approved and executed.
        env.as_contract(&contract_id, || {
            approve(&env, approver1, grant_id, 0).unwrap();
        });
        env.as_contract(&contract_id, || {
            approve(&env, approver2, grant_id, 0).unwrap();
            execute_release(&env, grant_id, 0).unwrap();
            assert!(get_request(&env, grant_id, 0).unwrap().executed);
        });
        assert_eq!(
            token::Client::new(&env, &token_id).balance(&recipient),
            1000
        );
    }

    #[test]
    fn test_clear_rejected_while_request_is_live() {
        let (env, grant_id, contract_id) = setup();
        env.as_contract(&contract_id, || {
            create_request(&env, grant_id, 0, 1000, Address::generate(&env)).unwrap();
            assert_eq!(
                clear_expired_request(&env, grant_id, 0),
                Err(ContractError::InvalidState)
            );
            assert!(get_request(&env, grant_id, 0).is_some());
        });
    }

    #[test]
    fn test_clear_rejected_when_missing_or_executed() {
        let (env, grant_id, contract_id) = setup();
        env.as_contract(&contract_id, || {
            assert_eq!(
                clear_expired_request(&env, grant_id, 0),
                Err(ContractError::InvalidState)
            );
            let mut request = EscrowReleaseRequest {
                grant_id,
                milestone_idx: 0,
                amount: 1,
                recipient: Address::generate(&env),
                approvals: Vec::new(&env),
                expires_at: 0,
                executed: true,
            };
            env.storage()
                .persistent()
                .set(&DataKey::EscrowReleaseRequest(grant_id, 0), &request);
            assert_eq!(
                clear_expired_request(&env, grant_id, 0),
                Err(ContractError::InvalidState)
            );
            request.executed = false;
            assert!(!is_expired_unexecuted(&env, grant_id, 1));
        });
    }

    #[test]
    fn test_recreate_entrypoint_is_owner_or_admin_only() {
        use crate::types::{Grant, GrantStatus};
        use soroban_sdk::String;

        let (env, grant_id, contract_id) = setup();
        let owner = Address::generate(&env);
        let admin = Address::generate(&env);
        let stranger = Address::generate(&env);

        env.as_contract(&contract_id, || {
            Storage::set_global_admin(&env, &admin);
            Storage::set_grant(
                &env,
                grant_id,
                &Grant {
                    id: grant_id,
                    owner: owner.clone(),
                    title: String::from_str(&env, "T"),
                    description: String::from_str(&env, "D"),
                    token: Address::generate(&env),
                    status: GrantStatus::Active,
                    total_amount: 1000,
                    milestone_amount: 1000,
                    reviewers: Vec::new(&env),
                    total_milestones: 1,
                    milestones_paid_out: 0,
                    escrow_balance: 0,
                    funders: Vec::new(&env),
                    reason: None,
                    timestamp: 0,
                    require_compliance: None,
                },
            );
            create_request(&env, grant_id, 0, 1000, Address::generate(&env)).unwrap();
        });
        expire(&env);
        env.as_contract(&contract_id, || {
            let call = |who: &Address| {
                crate::StellarGrantsContract::recreate_escrow_release_request(
                    env.clone(),
                    who.clone(),
                    grant_id,
                    0,
                )
            };
            assert_eq!(call(&stranger), Err(ContractError::Unauthorized));
            assert_eq!(call(&owner), Ok(()));
            assert!(!is_expired_unexecuted(&env, grant_id, 0));
        });
        expire(&env);
        env.as_contract(&contract_id, || {
            assert_eq!(
                crate::StellarGrantsContract::recreate_escrow_release_request(
                    env.clone(),
                    admin.clone(),
                    grant_id,
                    0
                ),
                Ok(())
            );
        });
    }

    // Issue #892: escrow_multisig::approve() had no signer whitelist — any
    // two throwaway addresses could together meet the default threshold of 2
    // and force through a payout. Verify ineligible callers are rejected and
    // never accumulate toward the threshold.
    #[test]
    fn test_approve_rejects_ineligible_signers() {
        let (env, grant_id, contract_id) = setup();
        let owner = Address::generate(&env);
        let recipient = Address::generate(&env);
        let outsider1 = Address::generate(&env);
        let outsider2 = Address::generate(&env);
        setup_grant(&env, &contract_id, grant_id, &owner, Vec::new(&env));

        env.as_contract(&contract_id, || {
            create_request(&env, grant_id, 0, 1000, recipient).unwrap();

            let result = approve(&env, outsider1, grant_id, 0);
            assert_eq!(result, Err(ContractError::Unauthorized));

            let result = approve(&env, outsider2, grant_id, 0);
            assert_eq!(result, Err(ContractError::Unauthorized));

            // Neither throwaway address counted toward the multisig threshold.
            assert!(!is_approved(&env, grant_id, 0));
            let request = get_request(&env, grant_id, 0).unwrap();
            assert_eq!(request.approvals.len(), 0);
        });
    }
}
