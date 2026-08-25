use soroban_sdk::{Address, Bytes, Env, Vec};

use crate::errors::ContractError;
use crate::events::{Events, MultisigExecuted, MultisigProposalCreated, MultisigSigned};
use crate::storage::Storage;
use crate::types::{MultisigProposal, MultisigSigner, SignatureStatus};

// ── Public API ────────────────────────────────────────────────────────────────

/// Create a new multisig proposal for a grant action. Grant owner or admin only.
pub fn create_proposal(
    env: &Env,
    creator: &Address,
    grant_id: u64,
    action_payload: Bytes,
    signer_addresses: Vec<Address>,
    threshold: u32,
    ttl_ledgers: u32,
) -> Result<u32, ContractError> {
    if signer_addresses.is_empty()
        || threshold == 0
        || threshold > signer_addresses.len()
    {
        return Err(ContractError::InvalidInput);
    }

    let mut signers: Vec<MultisigSigner> = Vec::new(env);
    for addr in signer_addresses.iter() {
        signers.push_back(MultisigSigner {
            address: addr,
            weight: 1,
            status: SignatureStatus::Pending,
            signed_at: None,
        });
    }

    let now = env.ledger().timestamp();
    let expired_at = now.saturating_add(ttl_ledgers as u64);

    let proposal_id = Storage::next_multisig_proposal_id(env);
    let proposal = MultisigProposal {
        id: proposal_id,
        grant_id,
        action_payload,
        signers,
        threshold,
        total_weight_signed: 0,
        executed: false,
        expired: false,
        expired_at,
        created_by: creator.clone(),
        created_at: now,
    };

    Storage::set_multisig_proposal(env, &proposal);

    MultisigProposalCreated {
        proposal_id,
        grant_id,
        created_by: creator.clone(),
        threshold,
        timestamp: now,
    }
    .publish(env);

    Ok(proposal_id)
}

/// A registered signer adds their signature (approve) or veto (reject).
/// Returns updated total_weight_signed.
pub fn sign(
    env: &Env,
    signer: &Address,
    proposal_id: u32,
    approve: bool,
) -> Result<u32, ContractError> {
    let mut proposal =
        Storage::get_multisig_proposal(env, proposal_id).ok_or(ContractError::ProposalNotFound)?;

    if proposal.executed {
        return Err(ContractError::ProposalAlreadyExecuted);
    }
    if env.ledger().timestamp() > proposal.expired_at {
        return Err(ContractError::ProposalExpired);
    }

    let mut found = false;
    for i in 0..proposal.signers.len() {
        let mut s = proposal.signers.get(i).unwrap();
        if s.address == *signer {
            found = true;
            if s.status != SignatureStatus::Pending {
                return Err(ContractError::AlreadyVoted);
            }
            if approve {
                s.status = SignatureStatus::Signed;
                s.signed_at = Some(env.ledger().timestamp());
                proposal.total_weight_signed =
                    proposal.total_weight_signed.saturating_add(s.weight);
            } else {
                s.status = SignatureStatus::Rejected;
                s.signed_at = Some(env.ledger().timestamp());
            }
            proposal.signers.set(i, s);
            break;
        }
    }

    if !found {
        return Err(ContractError::NotAProposalSigner);
    }

    let total_weight = proposal.total_weight_signed;
    Storage::set_multisig_proposal(env, &proposal);

    MultisigSigned {
        proposal_id,
        signer: signer.clone(),
        approved: approve,
        total_weight_signed: total_weight,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(total_weight)
}

/// Execute the proposal if threshold is met and TTL has not expired.
/// Returns action_payload for dispatch.
pub fn execute(env: &Env, caller: &Address, proposal_id: u32) -> Result<Bytes, ContractError> {
    let mut proposal =
        Storage::get_multisig_proposal(env, proposal_id).ok_or(ContractError::ProposalNotFound)?;

    if proposal.executed {
        return Err(ContractError::ProposalAlreadyExecuted);
    }
    if env.ledger().timestamp() > proposal.expired_at {
        return Err(ContractError::ProposalExpired);
    }
    if !is_threshold_met(&proposal) {
        return Err(ContractError::ThresholdNotMet);
    }

    proposal.executed = true;
    let payload = proposal.action_payload.clone();
    let grant_id = proposal.grant_id;
    Storage::set_multisig_proposal(env, &proposal);

    MultisigExecuted {
        proposal_id,
        grant_id,
        executed_by: caller.clone(),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(payload)
}

/// Check whether a proposal has reached its signing threshold.
pub fn is_threshold_met(proposal: &MultisigProposal) -> bool {
    proposal.total_weight_signed >= proposal.threshold
}

/// Return a proposal by id.
pub fn get_proposal(env: &Env, proposal_id: u32) -> Result<MultisigProposal, ContractError> {
    Storage::get_multisig_proposal(env, proposal_id).ok_or(ContractError::ProposalNotFound)
}

/// Mark a proposal expired if its TTL has passed. Anyone can call.
pub fn expire_proposal(env: &Env, proposal_id: u32) -> Result<(), ContractError> {
    let proposal =
        Storage::get_multisig_proposal(env, proposal_id).ok_or(ContractError::ProposalNotFound)?;

    if proposal.executed {
        return Err(ContractError::ProposalAlreadyExecuted);
    }
    if env.ledger().timestamp() <= proposal.expired_at {
        return Err(ContractError::InvalidState);
    }

    let mut proposal = proposal;
    proposal.expired = true;
    Storage::set_multisig_proposal(env, &proposal);
    Events::emit_multisig_proposal_expired(env, proposal_id);
    Ok(())
}

// ── Action payload encoding helpers ──────────────────────────────────────────

/// Encode a grant_id as an 8-byte big-endian payload (GrantWithdraw action).
pub fn encode_grant_withdraw(env: &Env, grant_id: u64) -> Bytes {
    let bytes = grant_id.to_be_bytes();
    let mut payload = Bytes::new(env);
    for b in bytes.iter() {
        payload.push_back(*b);
    }
    payload
}

/// Decode a grant_id from an 8-byte big-endian payload.
pub fn decode_grant_withdraw(payload: &Bytes) -> Option<u64> {
    if payload.len() < 8 {
        return None;
    }
    let mut bytes = [0u8; 8];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = payload.get(i as u32)?;
    }
    Some(u64::from_be_bytes(bytes))
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _};

    #[test]
    fn threshold_and_duplicate_signatures() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            let creator = Address::generate(&env);
            let signer1 = Address::generate(&env);
            let signer2 = Address::generate(&env);
            let mut signers = Vec::new(&env);
            signers.push_back(signer1.clone());
            signers.push_back(signer2.clone());

            let payload = encode_grant_withdraw(&env, 7);
            let id = create_proposal(&env, &creator, 7, payload.clone(), signers, 2, 100).unwrap();

            assert_eq!(sign(&env, &signer1, id, true), Ok(1));
            assert!(!is_threshold_met(&get_proposal(&env, id).unwrap()));
            assert_eq!(
                execute(&env, &creator, id),
                Err(ContractError::ThresholdNotMet)
            );

            assert_eq!(
                sign(&env, &signer1, id, true),
                Err(ContractError::AlreadyVoted)
            );
            assert_eq!(sign(&env, &signer2, id, true), Ok(2));
            assert!(is_threshold_met(&get_proposal(&env, id).unwrap()));
            assert_eq!(execute(&env, &creator, id), Ok(payload));
        });
    }

    #[test]
    fn rejects_unreachable_threshold() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            let creator = Address::generate(&env);
            let signer = Address::generate(&env);
            let mut signers = Vec::new(&env);
            signers.push_back(signer);

            assert_eq!(
                create_proposal(
                    &env,
                    &creator,
                    1,
                    Bytes::new(&env),
                    signers,
                    2,
                    100,
                ),
                Err(ContractError::InvalidInput)
            );
        });
    }

    #[test]
    fn expired_proposal_cannot_be_signed_or_executed() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            let creator = Address::generate(&env);
            let signer = Address::generate(&env);
            let mut signers = Vec::new(&env);
            signers.push_back(signer.clone());

            let id = create_proposal(&env, &creator, 1, Bytes::new(&env), signers, 1, 10).unwrap();
            env.ledger().set_timestamp(env.ledger().timestamp() + 11);

            assert_eq!(expire_proposal(&env, id), Ok(()));
            assert!(get_proposal(&env, id).unwrap().expired);

            assert_eq!(
                sign(&env, &signer, id, true),
                Err(ContractError::ProposalExpired)
            );
            assert_eq!(
                execute(&env, &creator, id),
                Err(ContractError::ProposalExpired)
            );
        });
    }

    #[test]
    fn grant_withdraw_payload_round_trip() {
        let env = Env::default();
        let grant_id = 123_456_789;
        let payload = encode_grant_withdraw(&env, grant_id);

        assert_eq!(decode_grant_withdraw(&payload), Some(grant_id));
    }
}
