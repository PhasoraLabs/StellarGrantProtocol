use crate::events::Events;
use crate::storage::keys::DataKey;
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
    // Use ContractError::NotFound, but wait, types.rs has ContractError::GrantNotFound or similar? We can just use ContractError::InvalidState or whatever.
    // I'll use ContractError::InvalidState if not found since there is no EscrowRequestNotFound.
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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn setup() -> (Env, u64) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let grant_id = 1u64;

        env.as_contract(&contract_id, || {
            let mut config = crate::config::default_config();
            config.multisig_escrow_threshold = 2;
            env.storage().persistent().set(&DataKey::Config, &config);
        });

        (env, grant_id)
    }

    #[test]
    fn test_create_request() {
        let (env, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
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
        let (env, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let recipient = Address::generate(&env);

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
        let (env, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let approver = Address::generate(&env);
        let recipient = Address::generate(&env);

        env.as_contract(&contract_id, || {
            create_request(&env, grant_id, 0, 1000, recipient).unwrap();

            approve(&env, approver.clone(), grant_id, 0).unwrap();
            let result = approve(&env, approver, grant_id, 0);
            assert_eq!(result, Err(ContractError::AlreadyVoted));
        });
    }

    #[test]
    fn test_execute_before_threshold_rejected() {
        let (env, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let approver = Address::generate(&env);
        let recipient = Address::generate(&env);

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
        let (env, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let recipient = Address::generate(&env);

        env.as_contract(&contract_id, || {
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
        let (env, grant_id) = setup();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let approver = Address::generate(&env);
        let recipient = Address::generate(&env);

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
}
