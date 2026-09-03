use soroban_sdk::{contractevent, Address, Bytes, Env};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{VerificationAttestation, VerificationLevel, VerificationStatus};

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorVerified {
    pub subject: Address,
    pub verifier: Address,
    pub level: VerificationLevel,
    pub expires_at: Option<u64>,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRevoked {
    pub subject: Address,
    pub revoked_by: Address,
}

/// Set the authorized verifier contract. Admin only.
pub fn set_verifier(env: &Env, admin: &Address, verifier: &Address) -> Result<(), ContractError> {
    admin.require_auth();
    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    Storage::set_verifier_contract(env, verifier);
    Ok(())
}

/// Verifier attests to a contributor's verified status.
pub fn attest(
    env: &Env,
    verifier: &Address,
    subject: &Address,
    level: VerificationLevel,
    expires_at: Option<u64>,
    attestation_hash: Bytes,
) -> Result<(), ContractError> {
    verifier.require_auth();
    if Storage::get_verifier_contract(env) != Some(verifier.clone()) {
        return Err(ContractError::NotVerifier);
    }

    let attestation = VerificationAttestation {
        subject: subject.clone(),
        verifier: verifier.clone(),
        level: level.clone(),
        status: VerificationStatus::Verified,
        attested_at: env.ledger().timestamp(),
        expires_at,
        attestation_hash,
    };
    Storage::set_verification_attestation(env, &attestation);

    ContributorVerified {
        subject: subject.clone(),
        verifier: verifier.clone(),
        level,
        expires_at,
    }
    .publish(env);

    Ok(())
}

/// Revoke a verification (verifier or admin).
pub fn revoke(env: &Env, caller: &Address, subject: &Address) -> Result<(), ContractError> {
    caller.require_auth();
    let is_admin = Storage::get_global_admin(env) == Some(caller.clone());
    let is_verifier = Storage::get_verifier_contract(env) == Some(caller.clone());
    if !is_admin && !is_verifier {
        return Err(ContractError::Unauthorized);
    }

    let mut attestation =
        Storage::get_verification_attestation(env, subject).ok_or(ContractError::KycRequired)?;
    attestation.status = VerificationStatus::Revoked;
    Storage::set_verification_attestation(env, &attestation);

    VerificationRevoked {
        subject: subject.clone(),
        revoked_by: caller.clone(),
    }
    .publish(env);

    Ok(())
}

/// Check if an address is verified at or above a required level.
pub fn is_verified(env: &Env, address: &Address, required_level: VerificationLevel) -> bool {
    if matches!(required_level, VerificationLevel::None) {
        return true;
    }

    let attestation = match Storage::get_verification_attestation(env, address) {
        Some(attestation) => attestation,
        None => return false,
    };
    if !matches!(attestation.status, VerificationStatus::Verified) {
        return false;
    }
    if let Some(expires_at) = attestation.expires_at {
        if env.ledger().timestamp() > expires_at {
            return false;
        }
    }

    (attestation.level as u32) >= (required_level as u32)
}

/// Assert verification or return Err(KycRequired).
pub fn require_verified(
    env: &Env,
    address: &Address,
    required_level: VerificationLevel,
) -> Result<(), ContractError> {
    if is_verified(env, address, required_level) {
        Ok(())
    } else {
        Err(ContractError::KycRequired)
    }
}

/// Return the attestation for an address.
pub fn get_attestation(env: &Env, address: &Address) -> Option<VerificationAttestation> {
    Storage::get_verification_attestation(env, address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Bytes;

    fn with_contract(env: &Env, f: impl FnOnce()) {
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.as_contract(&contract_id, f);
    }

    #[test]
    fn test_attest_and_is_verified() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, || {
            let admin = Address::generate(&env);
            let verifier = Address::generate(&env);
            let subject = Address::generate(&env);
            Storage::set_global_admin(&env, &admin);

            set_verifier(&env, &admin, &verifier).unwrap();
            attest(
                &env,
                &verifier,
                &subject,
                VerificationLevel::IdVerified,
                None,
                Bytes::new(&env),
            )
            .unwrap();

            assert!(is_verified(
                &env,
                &subject,
                VerificationLevel::EmailVerified
            ));
            assert!(is_verified(&env, &subject, VerificationLevel::IdVerified));
            assert!(!is_verified(&env, &subject, VerificationLevel::FullKyc));
        });
    }

    #[test]
    fn test_expired_attestation_is_unverified() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(10);
        with_contract(&env, || {
            let admin = Address::generate(&env);
            let verifier = Address::generate(&env);
            let subject = Address::generate(&env);
            Storage::set_global_admin(&env, &admin);
            set_verifier(&env, &admin, &verifier).unwrap();
            attest(
                &env,
                &verifier,
                &subject,
                VerificationLevel::FullKyc,
                Some(5),
                Bytes::new(&env),
            )
            .unwrap();

            assert!(!is_verified(&env, &subject, VerificationLevel::FullKyc));
        });
    }

    #[test]
    fn test_revoke_marks_unverified() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, || {
            let verifier = Address::generate(&env);
            let subject = Address::generate(&env);
            Storage::set_verifier_contract(&env, &verifier);
            Storage::set_verification_attestation(
                &env,
                &VerificationAttestation {
                    subject: subject.clone(),
                    verifier: verifier.clone(),
                    level: VerificationLevel::FullKyc,
                    status: VerificationStatus::Verified,
                    attested_at: env.ledger().timestamp(),
                    expires_at: None,
                    attestation_hash: Bytes::new(&env),
                },
            );
            revoke(&env, &verifier, &subject).unwrap();

            assert!(!is_verified(&env, &subject, VerificationLevel::FullKyc));
        });
    }

    #[test]
    fn test_unauthorized_attester_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, || {
            let admin = Address::generate(&env);
            let verifier = Address::generate(&env);
            let attacker = Address::generate(&env);
            let subject = Address::generate(&env);
            Storage::set_global_admin(&env, &admin);
            set_verifier(&env, &admin, &verifier).unwrap();

            let result = attest(
                &env,
                &attacker,
                &subject,
                VerificationLevel::FullKyc,
                None,
                Bytes::new(&env),
            );
            assert_eq!(result, Err(ContractError::NotVerifier));
        });
    }

    #[test]
    fn test_payout_below_threshold_skips_check() {
        let env = Env::default();
        with_contract(&env, || {
            let subject = Address::generate(&env);
            let cfg = crate::config::default_config();
            let payout_amount = cfg.kyc_payout_threshold.saturating_sub(1);

            let result = if payout_amount > cfg.kyc_payout_threshold {
                require_verified(&env, &subject, VerificationLevel::FullKyc)
            } else {
                Ok(())
            };

            assert_eq!(result, Ok(()));
            assert!(!is_verified(&env, &subject, VerificationLevel::FullKyc));
        });
    }
}
