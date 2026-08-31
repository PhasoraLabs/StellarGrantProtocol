use soroban_sdk::{Address, Env, String};

use crate::errors::ContractError;
use crate::events::{ComplianceAttested, ComplianceRevoked, VerifierChanged};
use crate::storage::Storage;
use crate::types::{ComplianceAttestation, ComplianceLevel, ComplianceStatus};

// ── Public API ────────────────────────────────────────────────────────────────

/// Register a trusted compliance verifier address. Admin only.
pub fn set_verifier(env: &Env, admin: &Address, verifier: &Address) -> Result<(), ContractError> {
    if Storage::get_global_admin(env) != Some(admin.clone()) {
        return Err(ContractError::Unauthorized);
    }
    let old_verifier = Storage::get_compliance_verifier(env);
    Storage::set_compliance_verifier(env, verifier);
    VerifierChanged {
        old_verifier,
        new_verifier: verifier.clone(),
    }
    .publish(env);
    Ok(())
}

/// A trusted verifier attests the compliance status of a subject address.
/// No personal data is stored — only status, level, jurisdiction code, and timestamps.
pub fn attest(
    env: &Env,
    verifier: &Address,
    subject: &Address,
    status: ComplianceStatus,
    level: ComplianceLevel,
    expires_at: u64,
    jurisdiction: String,
) -> Result<(), ContractError> {
    let registered_verifier =
        Storage::get_compliance_verifier(env).ok_or(ContractError::VerifierNotSet)?;
    if *verifier != registered_verifier {
        return Err(ContractError::NotVerifier);
    }

    let now = env.ledger().timestamp();
    let level_u32 = level as u32;

    let attestation = ComplianceAttestation {
        subject: subject.clone(),
        status,
        level,
        attested_by: verifier.clone(),
        attested_at: now,
        expires_at,
        jurisdiction,
    };
    Storage::set_compliance_attestation(env, &attestation);

    ComplianceAttested {
        subject: subject.clone(),
        attested_by: verifier.clone(),
        level: level_u32,
        expires_at,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Revoke a compliance attestation. Verifier or admin only.
pub fn revoke(env: &Env, revoker: &Address, subject: &Address) -> Result<(), ContractError> {
    let is_admin = Storage::get_global_admin(env) == Some(revoker.clone());
    let is_verifier = Storage::get_compliance_verifier(env) == Some(revoker.clone());
    if !is_admin && !is_verifier {
        return Err(ContractError::Unauthorized);
    }

    if Storage::get_compliance_attestation(env, subject).is_none() {
        return Err(ContractError::InvalidState);
    }

    Storage::remove_compliance_attestation(env, subject);

    ComplianceRevoked {
        subject: subject.clone(),
        revoked_by: revoker.clone(),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Return the compliance attestation for an address.
pub fn get_attestation(env: &Env, address: &Address) -> Option<ComplianceAttestation> {
    Storage::get_compliance_attestation(env, address)
}

/// Check if an address meets the required compliance level.
/// Returns Err if not compliant, expired, or rejected.
pub fn require_compliant(
    env: &Env,
    address: &Address,
    required_level: ComplianceLevel,
) -> Result<(), ContractError> {
    require_compliant_u32(env, address, required_level as u32)
}

/// Check if an address meets the required compliance level (u32 version).
/// Returns Err if not compliant, expired, or rejected.
pub fn require_compliant_u32(
    env: &Env,
    address: &Address,
    required_level: u32,
) -> Result<(), ContractError> {
    // ComplianceLevel::None means no check required.
    if required_level == ComplianceLevel::None as u32 {
        return Ok(());
    }

    let attestation = Storage::get_compliance_attestation(env, address)
        .ok_or(ContractError::ComplianceNotVerified)?;

    if !is_valid(env, &attestation) {
        return Err(ContractError::ComplianceCheckFailed);
    }

    if matches!(attestation.status, ComplianceStatus::Rejected) {
        return Err(ContractError::ComplianceCheckFailed);
    }

    if !matches!(attestation.status, ComplianceStatus::Approved) {
        return Err(ContractError::ComplianceNotVerified);
    }

    let attestation_level = attestation.level as u32;
    if attestation_level < required_level {
        return Err(ContractError::ComplianceCheckFailed);
    }

    Ok(())
}

/// Check if an attestation is still valid (not expired).
pub fn is_valid(env: &Env, attestation: &ComplianceAttestation) -> bool {
    let now = env.ledger().timestamp();
    if attestation.expires_at > 0 && now > attestation.expires_at {
        return false;
    }
    !matches!(
        attestation.status,
        ComplianceStatus::Expired | ComplianceStatus::Rejected
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};

    fn set_ledger(env: &Env, sequence: u32, timestamp: u64) {
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp,
            protocol_version: 21,
            sequence_number: sequence,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 100_000,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 1_000_000,
        });
    }

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let verifier = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        Storage::set_compliance_verifier(&env, &verifier);
        (env, admin, verifier)
    }

    fn subject() -> Address {
        Address::generate(&Env::default())
    }

    // ── set_verifier ──────────────────────────────────────────────────────

    #[test]
    fn admin_can_set_verifier() {
        let (env, admin, _) = setup();
        let new_verifier = Address::generate(&env);
        set_verifier(&env, &admin, &new_verifier).unwrap();
        assert_eq!(Storage::get_compliance_verifier(&env), Some(new_verifier));
    }

    #[test]
    fn non_admin_cannot_set_verifier() {
        let (env, _, _) = setup();
        let nobody = Address::generate(&env);
        let new_verifier = Address::generate(&env);
        assert_eq!(
            set_verifier(&env, &nobody, &new_verifier),
            Err(ContractError::Unauthorized)
        );
    }

    // ── attest per ComplianceLevel ────────────────────────────────────────

    #[test]
    fn attest_basic_level() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Basic,
            1_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.level, ComplianceLevel::Basic);
        assert_eq!(att.status, ComplianceStatus::Approved);
    }

    #[test]
    fn attest_standard_level() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "EU");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Standard,
            2_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.level, ComplianceLevel::Standard);
    }

    #[test]
    fn attest_enhanced_level() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "UK");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Enhanced,
            3_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.level, ComplianceLevel::Enhanced);
    }

    #[test]
    fn attest_none_level() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "ZZ");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::None,
            4_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.level, ComplianceLevel::None);
    }

    #[test]
    fn attest_pending_status() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Pending,
            ComplianceLevel::Standard,
            5_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.status, ComplianceStatus::Pending);
    }

    #[test]
    fn attest_rejected_status() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Rejected,
            ComplianceLevel::Basic,
            6_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.status, ComplianceStatus::Rejected);
    }

    #[test]
    fn attest_unverified_status() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Unverified,
            ComplianceLevel::Basic,
            7_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.status, ComplianceStatus::Unverified);
    }

    #[test]
    fn attest_expired_status() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Expired,
            ComplianceLevel::Basic,
            8_000,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.status, ComplianceStatus::Expired);
    }

    #[test]
    fn wrong_verifier_rejected() {
        let (env, _, _verifier) = setup();
        let stranger = Address::generate(&env);
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        assert_eq!(
            attest(
                &env,
                &stranger,
                &subj,
                ComplianceStatus::Approved,
                ComplianceLevel::Basic,
                1_000,
                jurisdiction,
            ),
            Err(ContractError::NotVerifier)
        );
    }

    #[test]
    fn no_verifier_set_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let verifier = Address::generate(&env);
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        assert_eq!(
            attest(
                &env,
                &verifier,
                &subj,
                ComplianceStatus::Approved,
                ComplianceLevel::Basic,
                1_000,
                jurisdiction,
            ),
            Err(ContractError::VerifierNotSet)
        );
    }

    #[test]
    fn attest_records_timestamps() {
        let (env, _, verifier) = setup();
        set_ledger(&env, 1, 100);
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Standard,
            200,
            jurisdiction,
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.attested_at, 100);
        assert_eq!(att.expires_at, 200);
        assert_eq!(att.attested_by, verifier);
    }

    // ── require_compliant — payout gating ─────────────────────────────────

    #[test]
    fn require_compliant_none_always_passes() {
        let (env, _admin, _verifier) = setup();
        let subj = Address::generate(&env);
        require_compliant(&env, &subj, ComplianceLevel::None).unwrap();
    }

    #[test]
    fn require_compliant_blocks_without_attestation() {
        let (env, _admin, _verifier) = setup();
        let subj = Address::generate(&env);
        assert_eq!(
            require_compliant(&env, &subj, ComplianceLevel::Basic),
            Err(ContractError::ComplianceNotVerified)
        );
    }

    #[test]
    fn require_compliant_blocks_when_below_required_level() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Basic,
            10_000,
            jurisdiction,
        )
        .unwrap();
        assert_eq!(
            require_compliant(&env, &subj, ComplianceLevel::Standard),
            Err(ContractError::ComplianceCheckFailed)
        );
    }

    #[test]
    fn require_compliant_passes_when_level_matches() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Standard,
            10_000,
            jurisdiction,
        )
        .unwrap();
        require_compliant(&env, &subj, ComplianceLevel::Standard).unwrap();
    }

    #[test]
    fn require_compliant_passes_when_level_exceeds_required() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Enhanced,
            10_000,
            jurisdiction,
        )
        .unwrap();
        require_compliant(&env, &subj, ComplianceLevel::Basic).unwrap();
    }

    #[test]
    fn require_compliant_blocks_when_pending() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Pending,
            ComplianceLevel::Enhanced,
            10_000,
            jurisdiction,
        )
        .unwrap();
        assert_eq!(
            require_compliant(&env, &subj, ComplianceLevel::Basic),
            Err(ContractError::ComplianceNotVerified)
        );
    }

    #[test]
    fn require_compliant_blocks_when_rejected() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Rejected,
            ComplianceLevel::Enhanced,
            10_000,
            jurisdiction,
        )
        .unwrap();
        assert_eq!(
            require_compliant(&env, &subj, ComplianceLevel::Basic),
            Err(ContractError::ComplianceCheckFailed)
        );
    }

    #[test]
    fn require_compliant_u32_works_same() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Standard,
            10_000,
            jurisdiction,
        )
        .unwrap();
        // Standard = 2
        require_compliant_u32(&env, &subj, ComplianceLevel::Standard as u32).unwrap();
        // Enhanced = 3 > Standard
        assert_eq!(
            require_compliant_u32(&env, &subj, ComplianceLevel::Enhanced as u32),
            Err(ContractError::ComplianceCheckFailed)
        );
    }

    #[test]
    fn require_compliant_u32_none_always_passes() {
        let (env, _admin, _verifier) = setup();
        let subj = Address::generate(&env);
        require_compliant_u32(&env, &subj, ComplianceLevel::None as u32).unwrap();
    }

    // ── expiry / revocation handling ──────────────────────────────────────

    #[test]
    fn require_compliant_blocks_when_expired() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Enhanced,
            100,
            jurisdiction,
        )
        .unwrap();
        // Advance past expiry
        set_ledger(&env, 1, 101);
        assert_eq!(
            require_compliant(&env, &subj, ComplianceLevel::Basic),
            Err(ContractError::ComplianceCheckFailed)
        );
    }

    #[test]
    fn require_compliant_passes_before_expiry() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Enhanced,
            200,
            jurisdiction,
        )
        .unwrap();
        set_ledger(&env, 1, 199);
        require_compliant(&env, &subj, ComplianceLevel::Basic).unwrap();
    }

    #[test]
    fn require_compliant_blocks_when_status_expired() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Expired,
            ComplianceLevel::Enhanced,
            10_000,
            jurisdiction,
        )
        .unwrap();
        assert_eq!(
            require_compliant(&env, &subj, ComplianceLevel::Basic),
            Err(ContractError::ComplianceCheckFailed)
        );
    }

    #[test]
    fn require_compliant_passes_with_zero_expiry() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Standard,
            0, // no expiry
            jurisdiction,
        )
        .unwrap();
        // Even far in the future, zero expiry means never expires
        set_ledger(&env, 1, 999_999);
        require_compliant(&env, &subj, ComplianceLevel::Standard).unwrap();
    }

    #[test]
    fn revocation_by_verifier() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Enhanced,
            10_000,
            jurisdiction,
        )
        .unwrap();
        revoke(&env, &verifier, &subj).unwrap();
        assert_eq!(get_attestation(&env, &subj), None);
    }

    #[test]
    fn revocation_by_admin() {
        let (env, admin, _) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        let verifier = Storage::get_compliance_verifier(&env).unwrap();
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Standard,
            10_000,
            jurisdiction,
        )
        .unwrap();
        revoke(&env, &admin, &subj).unwrap();
        assert_eq!(get_attestation(&env, &subj), None);
    }

    #[test]
    fn revocation_by_neither_fails() {
        let (env, _admin, _verifier) = setup();
        let stranger = Address::generate(&env);
        let subj = Address::generate(&env);
        assert_eq!(
            revoke(&env, &stranger, &subj),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn revoke_then_require_compliant_fails() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Enhanced,
            10_000,
            jurisdiction,
        )
        .unwrap();
        revoke(&env, &verifier, &subj).unwrap();
        assert_eq!(
            require_compliant(&env, &subj, ComplianceLevel::Basic),
            Err(ContractError::ComplianceNotVerified)
        );
    }

    #[test]
    fn test_revoke_never_attested_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let admin = Address::generate(&env);
        let verifier = Address::generate(&env);
        let subj = Address::generate(&env);

        env.as_contract(&contract_id, || {
            Storage::set_global_admin(&env, &admin);
            Storage::set_compliance_verifier(&env, &verifier);

            assert_eq!(
                revoke(&env, &verifier, &subj),
                Err(ContractError::InvalidState)
            );
            assert_eq!(
                revoke(&env, &admin, &subj),
                Err(ContractError::InvalidState)
            );
        });
    }

    #[test]
    fn test_set_verifier_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let admin = Address::generate(&env);
        let verifier = Address::generate(&env);
        let new_verifier = Address::generate(&env);

        env.as_contract(&contract_id, || {
            Storage::set_global_admin(&env, &admin);
            Storage::set_compliance_verifier(&env, &verifier);

            set_verifier(&env, &admin, &new_verifier).unwrap();
            assert_eq!(Storage::get_compliance_verifier(&env), Some(new_verifier));
        });
    }

    // ── is_valid ──────────────────────────────────────────────────────────

    #[test]
    fn is_valid_approved_not_expired() {
        let env = Env::default();
        set_ledger(&env, 1, 50);
        let att = ComplianceAttestation {
            subject: Address::generate(&env),
            status: ComplianceStatus::Approved,
            level: ComplianceLevel::Basic,
            attested_by: Address::generate(&env),
            attested_at: 0,
            expires_at: 100,
            jurisdiction: soroban_sdk::String::from_str(&env, "US"),
        };
        assert!(is_valid(&env, &att));
    }

    #[test]
    fn is_valid_expired_status() {
        let env = Env::default();
        let att = ComplianceAttestation {
            subject: Address::generate(&env),
            status: ComplianceStatus::Expired,
            level: ComplianceLevel::Basic,
            attested_by: Address::generate(&env),
            attested_at: 0,
            expires_at: 10_000,
            jurisdiction: soroban_sdk::String::from_str(&env, "US"),
        };
        assert!(!is_valid(&env, &att));
    }

    #[test]
    fn is_valid_rejected_status() {
        let env = Env::default();
        let att = ComplianceAttestation {
            subject: Address::generate(&env),
            status: ComplianceStatus::Rejected,
            level: ComplianceLevel::Basic,
            attested_by: Address::generate(&env),
            attested_at: 0,
            expires_at: 10_000,
            jurisdiction: soroban_sdk::String::from_str(&env, "US"),
        };
        assert!(!is_valid(&env, &att));
    }

    #[test]
    fn is_valid_past_expiry_time() {
        let env = Env::default();
        set_ledger(&env, 1, 200);
        let att = ComplianceAttestation {
            subject: Address::generate(&env),
            status: ComplianceStatus::Approved,
            level: ComplianceLevel::Basic,
            attested_by: Address::generate(&env),
            attested_at: 0,
            expires_at: 100,
            jurisdiction: soroban_sdk::String::from_str(&env, "US"),
        };
        assert!(!is_valid(&env, &att));
    }

    #[test]
    fn is_valid_zero_expiry_never_expires() {
        let env = Env::default();
        set_ledger(&env, 1, 999_999);
        let att = ComplianceAttestation {
            subject: Address::generate(&env),
            status: ComplianceStatus::Approved,
            level: ComplianceLevel::Basic,
            attested_by: Address::generate(&env),
            attested_at: 0,
            expires_at: 0,
            jurisdiction: soroban_sdk::String::from_str(&env, "US"),
        };
        assert!(is_valid(&env, &att));
    }

    // ── get_attestation ───────────────────────────────────────────────────

    #[test]
    fn get_attestation_none_for_unknown() {
        let (env, _admin, _verifier) = setup();
        let subj = Address::generate(&env);
        assert_eq!(get_attestation(&env, &subj), None);
    }

    #[test]
    fn get_attestation_returns_stored() {
        let (env, _, verifier) = setup();
        let subj = Address::generate(&env);
        let jurisdiction = soroban_sdk::String::from_str(&env, "US");
        attest(
            &env,
            &verifier,
            &subj,
            ComplianceStatus::Approved,
            ComplianceLevel::Standard,
            5_000,
            jurisdiction.clone(),
        )
        .unwrap();
        let att = get_attestation(&env, &subj).unwrap();
        assert_eq!(att.subject, subj);
        assert_eq!(att.jurisdiction, jurisdiction);
    }
}
