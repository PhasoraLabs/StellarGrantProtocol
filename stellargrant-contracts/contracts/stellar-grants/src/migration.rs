use soroban_sdk::{Address, Env, String, Vec};

use crate::events::Events;
use crate::storage::{
    ArbitrationKey, BondKey, CollateralKey, CrowdfundKey, DataKey, EscrowKey, GrantKey,
    InsuranceKey, LegacyDataKey, MilestoneKey, Storage, UserKey, VotingKey,
};
use crate::types::{ContractError, ContractVersion, MigrationRecord};

/// Return the current stored contract version.
pub fn get_version(env: &Env) -> Option<ContractVersion> {
    Storage::get_contract_version(env)
}

/// Set the initial version on first deploy. Can only be called once.
pub fn initialize_version(
    env: &Env,
    deployer: &Address,
    major: u32,
    minor: u32,
    patch: u32,
) -> Result<(), ContractError> {
    if Storage::get_contract_version(env).is_some() {
        return Ok(());
    }

    let version = ContractVersion {
        major,
        minor,
        patch,
        deployed_at: env.ledger().timestamp(),
        deployer: deployer.clone(),
    };

    Storage::set_contract_version(env, &version);
    Ok(())
}

/// Run the migration from current version to `target_version`. Admin only.
/// Internally dispatches to versioned migration functions (v1_to_v2, etc.).
/// Idempotent: if already at target_version, returns a no-op MigrationRecord.
pub fn run_migration(
    env: &Env,
    admin: &Address,
    target_version: ContractVersion,
) -> Result<MigrationRecord, ContractError> {
    let global_admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if global_admin != *admin {
        return Err(ContractError::Unauthorized);
    }

    let current = Storage::get_contract_version(env).ok_or(ContractError::InvalidState)?;

    let from_schema = current.major;
    let to_schema = target_version.major;

    // Idempotent: already at target version
    if current.major == target_version.major
        && current.minor == target_version.minor
        && current.patch == target_version.patch
    {
        let record = MigrationRecord {
            from_version: from_schema,
            to_version: to_schema,
            run_by: admin.clone(),
            run_at: env.ledger().timestamp(),
            success: true,
            notes: String::from_str(env, "no-op: already at target version"),
        };
        return Ok(record);
    }

    // Dispatch to versioned migration step
    let notes = if from_schema == 1 && to_schema == 2 {
        migrate_v1_to_v2(env)?
    } else {
        String::from_str(env, "migration step completed")
    };

    Storage::set_contract_version(env, &target_version);

    let record = MigrationRecord {
        from_version: from_schema,
        to_version: to_schema,
        run_by: admin.clone(),
        run_at: env.ledger().timestamp(),
        success: true,
        notes: notes.clone(),
    };

    let mut log: Vec<MigrationRecord> = Storage::get_migration_log(env);
    log.push_back(record.clone());
    Storage::set_migration_log(env, &log);

    Events::emit_contract_migrated(env, from_schema, to_schema, admin.clone());

    Ok(record)
}

/// Return the full migration history.
pub fn migration_history(env: &Env) -> Vec<MigrationRecord> {
    Storage::get_migration_log(env)
}

/// Internal: migration from schema v1 to v2 (placeholder — implement per future schema change).
fn migrate_v1_to_v2(env: &Env) -> Result<String, ContractError> {
    migrate_storage_keys_v2(env)?;
    Ok(String::from_str(env, "migrated schema from v1 to v2"))
}

// ── Storage key migration v1 → v2 ────────────────────────────────────────────

/// Migrate all v1 flat-enum `DataKey` storage entries to the new hierarchical
/// `DataKey` structure introduced in v2.
///
/// **Idempotent**: sets `DataKey::V2KeysMigrated` after completion. Calling
/// this function a second time is a no-op.
///
/// Migration strategy:
/// - Singleton keys: read from `LegacyDataKey`, write to new `DataKey`.
/// - Counter-backed collections: read the counter, iterate 1..=counter.
/// - Per-address collections use the available index structures as the source
///   of addresses. Address-keyed data that has no discoverable address list
///   (e.g. `RateLimit`, `ReferralRewards`) must be lazily re-written on first
///   access by the application; no data is lost because the old keys remain
///   until TTL expiry.
pub fn migrate_storage_keys_v2(env: &Env) -> Result<(), ContractError> {
    // Idempotent guard
    if env.storage().persistent().has(&DataKey::V2KeysMigrated) {
        return Ok(());
    }

    // Helper: read a value from the legacy key encoding.
    macro_rules! read_legacy {
        ($key:expr, $T:ty) => {
            env.storage().persistent().get::<LegacyDataKey, $T>(&$key)
        };
    }

    // ── Protocol singletons ───────────────────────────────────────────────────

    if let Some(v) = read_legacy!(LegacyDataKey::Admin, soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::Admin, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::GlobalAdmin, soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::GlobalAdmin, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::Treasury, soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::Treasury, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::Council, soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::Council, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::IdentityOracle, soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::IdentityOracle, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::MinReviewerStake, i128) {
        env.storage()
            .persistent()
            .set(&DataKey::MinReviewerStake, &v);
    }
    if let Some(v) = read_legacy!(
        LegacyDataKey::ContractVersion,
        crate::types::ContractVersion
    ) {
        env.storage()
            .persistent()
            .set(&DataKey::ContractVersion, &v);
    }
    if let Some(v) = read_legacy!(
        LegacyDataKey::MigrationLog,
        soroban_sdk::Vec<crate::types::MigrationRecord>
    ) {
        env.storage().persistent().set(&DataKey::MigrationLog, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::IsPaused, bool) {
        env.storage().persistent().set(&DataKey::IsPaused, &v);
    }
    if let Some(v) = read_legacy!(
        LegacyDataKey::PauseHistory,
        soroban_sdk::Vec<crate::types::PauseRecord>
    ) {
        env.storage().persistent().set(&DataKey::PauseHistory, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::ProtocolConfig, crate::types::ProtocolConfig) {
        env.storage().persistent().set(&DataKey::Config, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::OracleConfig, crate::types::OracleConfig) {
        env.storage().persistent().set(&DataKey::OracleConfig, &v);
    }
    if let Some(v) = read_legacy!(
        LegacyDataKey::ProtocolMetrics,
        crate::types::ProtocolMetrics
    ) {
        env.storage().persistent().set(&DataKey::Metrics, &v);
    }
    // AnalyticsSnapshot migration is deferred: the type has a pre-existing
    // contracttype macro error (field name > 30 chars) that prevents TryFromVal.
    // Its key will be lazily rewritten when the analytics module is repaired.
    if let Some(v) = read_legacy!(LegacyDataKey::DexConfig, crate::types::DexConfig) {
        env.storage().persistent().set(&DataKey::DexConfig, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::RelayConfig, crate::types::RelayConfig) {
        env.storage().persistent().set(&DataKey::RelayConfig, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::ComplianceVerifier, soroban_sdk::Address) {
        env.storage()
            .persistent()
            .set(&DataKey::ComplianceVerifier, &v);
    }
    if let Some(v) = read_legacy!(
        LegacyDataKey::ParamKeys,
        soroban_sdk::Vec<soroban_sdk::Symbol>
    ) {
        env.storage().persistent().set(&DataKey::ParamKeys, &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::ScoringRubricCounter, u32) {
        env.storage()
            .persistent()
            .set(&DataKey::ScoringRubricCounter, &v);
    }

    // Counters used to drive iteration below
    if let Some(v) = read_legacy!(LegacyDataKey::StreamCounter, u32) {
        env.storage().persistent().set(&DataKey::StreamCounter, &v);
    }

    // ── Global registry index (ContributorIndex / ReviewerAllowlist) ──────────

    if let Some(v) = read_legacy!(
        LegacyDataKey::ContributorIndex,
        soroban_sdk::Vec<crate::types::RegistryEntry>
    ) {
        env.storage()
            .persistent()
            .set(&DataKey::User(UserKey::RegistryIndex), &v);
    }
    if let Some(v) = read_legacy!(
        LegacyDataKey::ReviewerAllowlist,
        soroban_sdk::Vec<soroban_sdk::Address>
    ) {
        env.storage()
            .persistent()
            .set(&DataKey::User(UserKey::ReviewerAllowlist), &v);
    }

    // Global grant order / counters
    if let Some(v) = read_legacy!(LegacyDataKey::GlobalGrantOrder, soroban_sdk::Vec<u64>) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::GlobalOrder), &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::GrantCounter, u64) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::Counter), &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::GrantCounterValue, u64) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::CounterValue), &v);
    }
    if let Some(v) = read_legacy!(
        LegacyDataKey::CategoryList,
        soroban_sdk::Vec<crate::types::GrantCategory>
    ) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::CategoryList), &v);
    }

    // Arbitration pool singletons
    if let Some(v) = read_legacy!(
        LegacyDataKey::ArbiterPool,
        soroban_sdk::Vec<soroban_sdk::Address>
    ) {
        env.storage()
            .persistent()
            .set(&DataKey::Arbitration(ArbitrationKey::Pool), &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::ArbiterPoolToken, soroban_sdk::Address) {
        env.storage()
            .persistent()
            .set(&DataKey::Arbitration(ArbitrationKey::PoolToken), &v);
    }
    if let Some(v) = read_legacy!(LegacyDataKey::ArbitrationCaseCounter, u32) {
        env.storage()
            .persistent()
            .set(&DataKey::Arbitration(ArbitrationKey::CaseCounter), &v);
    }

    // Bond counter
    if let Some(v) = read_legacy!(LegacyDataKey::BondCounter, u32) {
        env.storage()
            .persistent()
            .set(&DataKey::Bond(BondKey::Counter), &v);
    }

    // Insurance claim counter
    if let Some(v) = read_legacy!(LegacyDataKey::InsuranceClaimCounter, u32) {
        env.storage()
            .persistent()
            .set(&DataKey::Insurance(InsuranceKey::ClaimCounter), &v);
    }

    // Multisig proposal counter
    if let Some(v) = read_legacy!(LegacyDataKey::MultisigProposalCounter, u32) {
        env.storage()
            .persistent()
            .set(&DataKey::Voting(VotingKey::ProposalCounter), &v);
    }

    // Crowdfund counter
    if let Some(v) = read_legacy!(LegacyDataKey::CrowdfundCounter, u64) {
        env.storage()
            .persistent()
            .set(&DataKey::Crowdfund(CrowdfundKey::Counter), &v);
    }

    // NFT counter
    if let Some(v) = read_legacy!(LegacyDataKey::NftCounter, u32) {
        env.storage()
            .persistent()
            .set(&DataKey::Milestone(MilestoneKey::NftCounter), &v);
    }

    // ── Grant-indexed data (iterate 1..=grant_counter) ────────────────────────

    let grant_counter: u64 = read_legacy!(LegacyDataKey::GrantCounter, u64).unwrap_or(0);
    for gid in 1..=grant_counter {
        if let Some(v) = read_legacy!(LegacyDataKey::Grant(gid), crate::types::Grant) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::Data(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::AuditLog(gid),
            soroban_sdk::Vec<crate::types::AuditEntry>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::AuditLog(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::EscrowState(gid), crate::types::EscrowState) {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(EscrowKey::State(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::EscrowAccount(gid),
            crate::types::EscrowAccount
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(EscrowKey::Account(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::EscrowFundersList(gid),
            soroban_sdk::Vec<soroban_sdk::Address>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(EscrowKey::FundersList(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::GrantTags(gid), crate::types::GrantTag) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::Tags(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::RenewalProposal(gid),
            crate::types::RenewalProposal
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::Renewal(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::RenewalHistory(gid), u64) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::RenewalHistory(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::ForkRecord(gid), crate::types::ForkRecord) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::Fork(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::ForkChildren(gid), soroban_sdk::Vec<u64>) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::ForkChildren(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::TransferProposal(gid),
            crate::types::TransferProposal
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::Transfer(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::SyndicateGrant(gid),
            crate::types::SyndicateGrant
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::Syndicate(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::SyndicateMembers(gid),
            soroban_sdk::Vec<soroban_sdk::Address>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::SyndicateMembers(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::MilestoneDag(gid), crate::types::MilestoneDag)
        {
            env.storage()
                .persistent()
                .set(&DataKey::Milestone(MilestoneKey::Dag(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::CollateralRequirement(gid),
            crate::types::CollateralRequirement
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Collateral(CollateralKey::Requirement(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::PerformanceBond(gid),
            crate::types::PerformanceBond
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Bond(BondKey::Bond(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::AmendmentHistory(gid), soroban_sdk::Vec<u32>) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::AmendmentHistory(gid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::CurrentVersion(gid), u32) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::CurrentVersion(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::ExtensionHistory(gid),
            soroban_sdk::Vec<crate::types::ExtensionRequest>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Milestone(MilestoneKey::ExtensionHistory(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::InsurancePolicy(gid),
            crate::types::InsurancePolicy
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Insurance(InsuranceKey::Policy(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::MultisigSigners(gid),
            soroban_sdk::Vec<soroban_sdk::Address>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Voting(VotingKey::MultisigSigners(gid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::VotingMechanism(gid),
            crate::types::VotingMechanism
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Voting(VotingKey::Mechanism(gid)), &v);
        }
    }

    // ── Streams (1..=stream_counter) ──────────────────────────────────────────

    let stream_counter: u32 = read_legacy!(LegacyDataKey::StreamCounter, u32).unwrap_or(0);
    for sid in 1..=stream_counter {
        if let Some(v) = read_legacy!(LegacyDataKey::Stream(sid), crate::types::PaymentStream) {
            env.storage().persistent().set(&DataKey::Stream(sid), &v);
        }
    }

    // ── Insurance claims (1..=claim_counter) ─────────────────────────────────

    let claim_counter: u32 = read_legacy!(LegacyDataKey::InsuranceClaimCounter, u32).unwrap_or(0);
    for cid in 1..=claim_counter {
        if let Some(v) = read_legacy!(
            LegacyDataKey::InsuranceClaim(cid),
            crate::types::InsuranceClaim
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Insurance(InsuranceKey::Claim(cid)), &v);
        }
    }

    // ── Multisig proposals (1..=proposal_counter) ────────────────────────────

    let proposal_counter: u32 =
        read_legacy!(LegacyDataKey::MultisigProposalCounter, u32).unwrap_or(0);
    for pid in 1..=proposal_counter {
        if let Some(v) = read_legacy!(
            LegacyDataKey::MultisigProposal(pid),
            crate::types::MultisigProposal
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Voting(VotingKey::Proposal(pid)), &v);
        }
    }

    // ── Crowdfund campaigns (1..=crowdfund_counter) ───────────────────────────

    let cf_counter: u64 = read_legacy!(LegacyDataKey::CrowdfundCounter, u64).unwrap_or(0);
    for cid in 1..=cf_counter {
        if let Some(v) = read_legacy!(
            LegacyDataKey::CrowdfundCampaign(cid),
            crate::types::CrowdfundCampaign
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Crowdfund(CrowdfundKey::Campaign(cid)), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::CrowdfundBackers(cid),
            soroban_sdk::Vec<soroban_sdk::Address>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Crowdfund(CrowdfundKey::Backers(cid)), &v);
        }
    }

    // ── Arbitration cases (1..=case_counter) ─────────────────────────────────

    let case_counter: u32 = read_legacy!(LegacyDataKey::ArbitrationCaseCounter, u32).unwrap_or(0);
    for cid in 1..=case_counter {
        if let Some(v) = read_legacy!(
            LegacyDataKey::ArbitrationCase(cid),
            crate::types::ArbitrationCase
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Arbitration(ArbitrationKey::Case(cid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::ArbitrationSettled(cid), bool) {
            env.storage()
                .persistent()
                .set(&DataKey::Arbitration(ArbitrationKey::Settled(cid)), &v);
        }
    }

    // ── Bond index (1..=bond_counter) ────────────────────────────────────────

    let bond_counter: u32 = read_legacy!(LegacyDataKey::BondCounter, u32).unwrap_or(0);
    for bid in 1..=bond_counter {
        if let Some(v) = read_legacy!(LegacyDataKey::BondGrant(bid), u64) {
            env.storage()
                .persistent()
                .set(&DataKey::Bond(BondKey::BondGrant(bid)), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::BondClaim(bid), crate::types::BondClaim) {
            env.storage()
                .persistent()
                .set(&DataKey::Bond(BondKey::BondClaim(bid)), &v);
        }
    }

    // ── Scoring rubrics (1..=rubric_counter) ─────────────────────────────────

    let rubric_counter: u32 = read_legacy!(LegacyDataKey::ScoringRubricCounter, u32).unwrap_or(0);
    for rid in 1..=rubric_counter {
        if let Some(v) = read_legacy!(
            LegacyDataKey::ScoringRubric(rid),
            crate::types::ScoringRubric
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::ScoringRubric(rid), &v);
        }
    }

    // ── Per-address data discovered from the global contributor registry ──────
    //
    // We use ContributorIndex (the existing registry) as a source of known
    // contributor addresses, and ReviewerAllowlist for reviewer addresses.
    // Data keyed only by address (e.g. RateLimit, ReferralRewards) that has
    // no index is lazily re-written by the application on first access.

    let contributor_index = Storage::get_contributor_index(env);
    for entry in contributor_index.iter() {
        let addr = entry.address.clone();
        if let Some(v) = read_legacy!(
            LegacyDataKey::Contributor(addr.clone()),
            crate::types::ContributorProfile
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::User(UserKey::Profile(addr.clone())), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::ContributorGrantIds(addr.clone()),
            soroban_sdk::Vec<u64>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::User(UserKey::GrantIds(addr.clone())), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::FunderGrantIndex(addr.clone()),
            soroban_sdk::Vec<u64>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::User(UserKey::FunderGrants(addr.clone())), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::ReviewerReputation(addr.clone()), u32) {
            env.storage()
                .persistent()
                .set(&DataKey::User(UserKey::ReviewerRep(addr.clone())), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::ReviewerProfile(addr.clone()),
            crate::types::ReviewerProfile
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::User(UserKey::ReviewerProfile(addr.clone())), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::NftsByAddress(addr.clone()),
            soroban_sdk::Vec<u32>
        ) {
            env.storage().persistent().set(
                &DataKey::Milestone(MilestoneKey::NftsByOwner(addr.clone())),
                &v,
            );
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::IndexByOwner(addr.clone()),
            soroban_sdk::Vec<u64>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::OwnerIndex(addr.clone())), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::IndexByContributor(addr.clone()),
            soroban_sdk::Vec<u64>
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::Grant(GrantKey::ContribIndex(addr.clone())), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::ReferralRecord(addr.clone()),
            crate::types::ReferralRecord
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::ReferralRecord(addr.clone()), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::ComplianceAttestation(addr.clone()),
            crate::types::ComplianceAttestation
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::ComplianceAttestation(addr.clone()), &v);
        }
        if let Some(v) = read_legacy!(
            LegacyDataKey::RelayAllowance(addr.clone()),
            crate::types::RelayAllowance
        ) {
            env.storage()
                .persistent()
                .set(&DataKey::RelayAllowance(addr.clone()), &v);
        }
        if let Some(v) = read_legacy!(LegacyDataKey::RelayNonce(addr.clone()), u32) {
            env.storage()
                .persistent()
                .set(&DataKey::RelayNonce(addr.clone()), &v);
        }
    }

    // Mark migration complete — idempotent guard for future calls
    env.storage()
        .persistent()
        .set(&DataKey::V2KeysMigrated, &true);
    Ok(())
}
