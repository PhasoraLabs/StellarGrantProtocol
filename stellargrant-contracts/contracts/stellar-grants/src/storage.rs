use crate::types::{
    AuditEntry, ContractError, ContractVersion, ContributorProfile, Dispute, EscrowState, Grant,
    HookEvent, HookRegistration, InsuranceClaim, InsurancePolicy, MigrationRecord, Milestone,
    PauseRecord, PaymentStream, ProtocolConfig, QuadraticVoteRecord, RegistryEntry, OracleConfig,
    VoiceCredits, VotingMechanism,
};
use soroban_sdk::{contracttype, Address, Env, Vec};

#[contracttype]
pub enum DataKey {
    Admin,
    Grant(u64),
    Milestone(u64, u32),
    ReviewerStake(u64, Address),
    MinReviewerStake,
    Treasury,
    IdentityOracle,
    GlobalAdmin,
    Council,
    Contributor(Address),
    GrantCounter,
    EscrowState(u64),
    MultisigSigners(u64),
    ReleaseApproval(u64, Address),
    ReviewerReputation(Address),
    // Contract version tracking (#527)
    ContractVersion,
    MigrationLog,
    // Global registry (#520)
    ContributorIndex,
    ReviewerAllowlist,
    // Immutable audit trail (#523)
    AuditLog(u64),
    // Emergency pause (#521)
    IsPaused,
    PauseHistory,
    // Streaming payments (#531)
    Stream(u32),
    StreamCounter,
    // Quadratic voting (#537)
    VoiceCredits(Address, u64),
    VotingMechanism(u64),
    QvVotes(u64, u32),
    // Insurance pool (#538)
    InsurancePool(Address),
    InsurancePolicy(u64),
    InsuranceClaim(u32),
    InsuranceClaimCounter,
    // External hooks (#539)
    HookRegistry(u32),
    // Issue #151: milestone reputation tracking
    MilestoneReputationApplied(u64, u32),
    // Issue #514: arbiter-based dispute record
    DisputeRecord(u64, u32),
    // Issue #516: runtime protocol configuration
    ProtocolConfig,
    // Issue #517: cumulative fees per token
    FeesCollected(Address),
    // Issue #524: price oracle configuration
    OracleConfig,
}

const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_000_000;

pub struct Storage;

impl Storage {
    fn bump_persistent_ttl(env: &Env, key: &DataKey) {
        env.storage().persistent().extend_ttl(
            key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }

    pub fn increment_grant_counter(env: &Env) -> u64 {
        let mut count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::GrantCounter)
            .unwrap_or(0);
        count += 1;
        env.storage()
            .persistent()
            .set(&DataKey::GrantCounter, &count);
        count
    }

    pub fn get_global_admin(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::GlobalAdmin)
    }

    pub fn set_global_admin(env: &Env, admin: &Address) {
        env.storage().persistent().set(&DataKey::GlobalAdmin, admin);
    }

    pub fn get_council(env: &Env) -> Option<soroban_sdk::Address> {
        env.storage().persistent().get(&DataKey::Council)
    }

    pub fn set_council(env: &Env, council: &soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::Council, council);
    }

    pub fn get_grant(env: &Env, grant_id: u64) -> Option<Grant> {
        env.storage().persistent().get(&DataKey::Grant(grant_id))
    }

    pub fn get_grant_v(env: &Env, grant_id: u64) -> Grant {
        Self::get_grant(env, grant_id).unwrap_or_else(|| {
            env.panic_with_error(ContractError::GrantNotFound);
        })
    }

    pub fn set_grant(env: &Env, grant_id: u64, grant: &Grant) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(grant_id), grant);
    }

    pub fn has_grant(env: &Env, grant_id: u64) -> bool {
        env.storage().persistent().has(&DataKey::Grant(grant_id))
    }

    pub fn get_milestone(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Milestone> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(grant_id, milestone_idx))
    }

    pub fn get_milestone_v(env: &Env, grant_id: u64, milestone_idx: u32) -> Milestone {
        Self::get_milestone(env, grant_id, milestone_idx).unwrap_or_else(|| {
            env.panic_with_error(ContractError::MilestoneNotFound);
        })
    }

    pub fn set_milestone(env: &Env, grant_id: u64, milestone_idx: u32, milestone: &Milestone) {
        env.storage()
            .persistent()
            .set(&DataKey::Milestone(grant_id, milestone_idx), milestone);
    }

    pub fn get_contributor(env: &Env, contributor: Address) -> Option<ContributorProfile> {
        env.storage()
            .persistent()
            .get(&DataKey::Contributor(contributor))
    }

    pub fn set_contributor(env: &Env, contributor: Address, profile: &ContributorProfile) {
        env.storage()
            .persistent()
            .set(&DataKey::Contributor(contributor), profile);
    }

    pub fn get_escrow_state(env: &Env, grant_id: u64) -> EscrowState {
        env.storage()
            .persistent()
            .get(&DataKey::EscrowState(grant_id))
            .unwrap_or_else(|| {
                env.panic_with_error(ContractError::InvalidState);
            })
    }

    pub fn set_escrow_state(env: &Env, grant_id: u64, state: &EscrowState) {
        env.storage()
            .persistent()
            .set(&DataKey::EscrowState(grant_id), state);
    }

    pub fn get_multisig_signers(env: &Env, grant_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::MultisigSigners(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_multisig_signers(env: &Env, grant_id: u64, signers: &Vec<Address>) {
        env.storage()
            .persistent()
            .set(&DataKey::MultisigSigners(grant_id), signers);
    }

    pub fn has_release_approval(env: &Env, grant_id: u64, signer: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseApproval(grant_id, signer.clone()))
            .unwrap_or(false)
    }

    pub fn set_release_approval(env: &Env, grant_id: u64, signer: &Address, approved: bool) {
        env.storage().persistent().set(
            &DataKey::ReleaseApproval(grant_id, signer.clone()),
            &approved,
        );
    }

    pub fn get_reviewer_reputation(env: &Env, reviewer: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ReviewerReputation(reviewer))
            .unwrap_or(1)
    }

    pub fn set_reviewer_reputation(env: &Env, reviewer: Address, reputation: u32) {
        env.storage()
            .persistent()
            .set(&DataKey::ReviewerReputation(reviewer), &reputation);
    }

    pub fn get_reviewer_stake(env: &Env, grant_id: u64, reviewer: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::ReviewerStake(grant_id, reviewer.clone()))
            .unwrap_or(0)
    }

    pub fn set_reviewer_stake(env: &Env, grant_id: u64, reviewer: &Address, amount: i128) {
        env.storage()
            .persistent()
            .set(&DataKey::ReviewerStake(grant_id, reviewer.clone()), &amount);
    }

    pub fn get_min_reviewer_stake(env: &Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::MinReviewerStake)
            .unwrap_or(0)
    }

    pub fn get_treasury(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::Treasury)
    }

    pub fn set_treasury(env: &Env, treasury: &Address) {
        env.storage().persistent().set(&DataKey::Treasury, treasury);
    }

    pub fn get_identity_oracle(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::IdentityOracle)
    }

    // ── Contract Version (#527) ───────────────────────────────────────

    pub fn get_contract_version(env: &Env) -> Option<ContractVersion> {
        env.storage().persistent().get(&DataKey::ContractVersion)
    }

    pub fn set_contract_version(env: &Env, version: &ContractVersion) {
        env.storage()
            .persistent()
            .set(&DataKey::ContractVersion, version);
    }

    pub fn get_migration_log(env: &Env) -> Vec<MigrationRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::MigrationLog)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_migration_log(env: &Env, log: &Vec<MigrationRecord>) {
        env.storage().persistent().set(&DataKey::MigrationLog, log);
    }

    // ── Global Registry (#520) ────────────────────────────────────────

    pub fn get_contributor_index(env: &Env) -> Vec<RegistryEntry> {
        env.storage()
            .persistent()
            .get(&DataKey::ContributorIndex)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_contributor_index(env: &Env, index: &Vec<RegistryEntry>) {
        env.storage()
            .persistent()
            .set(&DataKey::ContributorIndex, index);
    }

    pub fn get_reviewer_allowlist(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::ReviewerAllowlist)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_reviewer_allowlist(env: &Env, list: &Vec<Address>) {
        env.storage()
            .persistent()
            .set(&DataKey::ReviewerAllowlist, list);
    }

    // ── Immutable Audit Trail (#523) ──────────────────────────────────

    const AUDIT_TTL_THRESHOLD: u32 = 100_000;
    const AUDIT_TTL_EXTEND_TO: u32 = 1_000_000;

    pub fn get_audit_log(env: &Env, grant_id: u64) -> Vec<AuditEntry> {
        env.storage()
            .persistent()
            .get(&DataKey::AuditLog(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn append_audit_entry(env: &Env, grant_id: u64, entry: &AuditEntry) {
        let key = DataKey::AuditLog(grant_id);
        let mut log = Self::get_audit_log(env, grant_id);
        log.push_back(entry.clone());
        env.storage().persistent().set(&key, &log);
        env.storage().persistent().extend_ttl(
            &key,
            Self::AUDIT_TTL_THRESHOLD,
            Self::AUDIT_TTL_EXTEND_TO,
        );
        env.storage()
            .instance()
            .extend_ttl(Self::AUDIT_TTL_THRESHOLD, Self::AUDIT_TTL_EXTEND_TO);
    }

    // ── Emergency Pause (#521) ───────────────────────────────────────────────

    pub fn get_is_paused(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    pub fn set_is_paused(env: &Env, paused: bool) {
        env.storage()
            .persistent()
            .set(&DataKey::IsPaused, &paused);
    }

    pub fn get_pause_history(env: &Env) -> Vec<PauseRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::PauseHistory)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn append_pause_record(env: &Env, record: &PauseRecord) {
        let mut history = Self::get_pause_history(env);
        history.push_back(record.clone());
        env.storage()
            .persistent()
            .set(&DataKey::PauseHistory, &history);
    }

    pub fn set_latest_pause_unpaused_at(env: &Env, timestamp: u64) {
        let mut history = Self::get_pause_history(env);
        let len = history.len();
        if len == 0 {
            return;
        }
        let mut last = history.get(len - 1).unwrap();
        last.unpaused_at = Some(timestamp);
        history.set(len - 1, last);
        env.storage()
            .persistent()
            .set(&DataKey::PauseHistory, &history);
    }

    // ── Streaming Payments (#531) ─────────────────────────────────────────────

    pub fn next_stream_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::StreamCounter)
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::StreamCounter, &id);
        id
    }

    pub fn get_stream(env: &Env, stream_id: u32) -> Option<PaymentStream> {
        env.storage()
            .persistent()
            .get(&DataKey::Stream(stream_id))
    }

    pub fn set_stream(env: &Env, stream: &PaymentStream) {
        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream.id), stream);
    }

    // ── Quadratic Voting (#537) ───────────────────────────────────────────────

    pub fn get_voice_credits(env: &Env, voter: &Address, grant_id: u64) -> Option<VoiceCredits> {
        env.storage()
            .persistent()
            .get(&DataKey::VoiceCredits(voter.clone(), grant_id))
    }

    pub fn set_voice_credits(env: &Env, credits: &VoiceCredits) {
        env.storage().persistent().set(
            &DataKey::VoiceCredits(credits.voter.clone(), credits.grant_id),
            credits,
        );
    }

    pub fn get_voting_mechanism(env: &Env, grant_id: u64) -> VotingMechanism {
        env.storage()
            .persistent()
            .get(&DataKey::VotingMechanism(grant_id))
            .unwrap_or(VotingMechanism::SimpleMajority)
    }

    pub fn set_voting_mechanism(env: &Env, grant_id: u64, mechanism: &VotingMechanism) {
        env.storage()
            .persistent()
            .set(&DataKey::VotingMechanism(grant_id), mechanism);
    }

    pub fn get_qv_votes(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<QuadraticVoteRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::QvVotes(grant_id, milestone_idx))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_qv_votes(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        votes: &Vec<QuadraticVoteRecord>,
    ) {
        env.storage()
            .persistent()
            .set(&DataKey::QvVotes(grant_id, milestone_idx), votes);
    }

    // ── Insurance Pool (#538) ─────────────────────────────────────────────────

    pub fn get_insurance_pool(env: &Env, token: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::InsurancePool(token.clone()))
            .unwrap_or(0)
    }

    pub fn set_insurance_pool(env: &Env, token: &Address, balance: i128) {
        env.storage()
            .persistent()
            .set(&DataKey::InsurancePool(token.clone()), &balance);
    }

    pub fn get_insurance_policy(env: &Env, grant_id: u64) -> Option<InsurancePolicy> {
        env.storage()
            .persistent()
            .get(&DataKey::InsurancePolicy(grant_id))
    }

    pub fn set_insurance_policy(env: &Env, policy: &InsurancePolicy) {
        env.storage()
            .persistent()
            .set(&DataKey::InsurancePolicy(policy.grant_id), policy);
    }

    pub fn next_claim_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::InsuranceClaimCounter)
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::InsuranceClaimCounter, &id);
        id
    }

    pub fn get_insurance_claim(env: &Env, claim_id: u32) -> Option<InsuranceClaim> {
        env.storage()
            .persistent()
            .get(&DataKey::InsuranceClaim(claim_id))
    }

    pub fn set_insurance_claim(env: &Env, claim: &InsuranceClaim) {
        env.storage()
            .persistent()
            .set(&DataKey::InsuranceClaim(claim.id), claim);
    }

    // ── External Hooks (#539) ─────────────────────────────────────────────────

    pub fn get_hook_registry(env: &Env, event: &HookEvent) -> Vec<HookRegistration> {
        let key = DataKey::HookRegistry(event.clone() as u32);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_hook_registry(env: &Env, event: &HookEvent, hooks: &Vec<HookRegistration>) {
        env.storage()
            .persistent()
            .set(&DataKey::HookRegistry(event.clone() as u32), hooks);
    }

    // ── Issue #151: milestone reputation tracking ─────────────────────────────

    pub fn has_milestone_reputation_applied(env: &Env, grant_id: u64, milestone_idx: u32) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::MilestoneReputationApplied(
                grant_id,
                milestone_idx,
            ))
    }

    pub fn mark_milestone_reputation_applied(env: &Env, grant_id: u64, milestone_idx: u32) {
        let key = DataKey::MilestoneReputationApplied(grant_id, milestone_idx);
        env.storage().persistent().set(&key, &true);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #514: arbiter-based dispute record ──────────────────────────────

    pub fn get_dispute(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Dispute> {
        let key = DataKey::DisputeRecord(grant_id, milestone_idx);
        let d = env.storage().persistent().get(&key);
        if d.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        d
    }

    pub fn set_dispute(env: &Env, grant_id: u64, milestone_idx: u32, dispute: &Dispute) {
        let key = DataKey::DisputeRecord(grant_id, milestone_idx);
        env.storage().persistent().set(&key, dispute);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn remove_dispute(env: &Env, grant_id: u64, milestone_idx: u32) {
        env.storage()
            .persistent()
            .remove(&DataKey::DisputeRecord(grant_id, milestone_idx));
    }

    // ── Issue #516: ProtocolConfig ────────────────────────────────────────────

    pub fn get_protocol_config(env: &Env) -> Option<ProtocolConfig> {
        let key = DataKey::ProtocolConfig;
        let cfg = env.storage().persistent().get(&key);
        if cfg.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        cfg
    }

    pub fn set_protocol_config(env: &Env, cfg: &ProtocolConfig) {
        let key = DataKey::ProtocolConfig;
        env.storage().persistent().set(&key, cfg);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #517: cumulative fees per token ─────────────────────────────────

    pub fn get_fees_collected(env: &Env, token: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::FeesCollected(token.clone()))
            .unwrap_or(0)
    }

    pub fn add_fees_collected(env: &Env, token: &Address, amount: i128) {
        let key = DataKey::FeesCollected(token.clone());
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&key, &current.saturating_add(amount));
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #524: Price oracle configuration ────────────────────────────────

    pub fn get_oracle_config(env: &Env) -> Option<OracleConfig> {
        let key = DataKey::OracleConfig;
        let config = env.storage().persistent().get(&key);
        if config.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        config
    }

    pub fn set_oracle_config(env: &Env, config: &OracleConfig) {
        let key = DataKey::OracleConfig;
        env.storage().persistent().set(&key, config);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Emergency pause storage ───────────────────────────────────────────────

    pub fn get_is_paused(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    pub fn set_is_paused(env: &Env, val: bool) {
        env.storage().persistent().set(&DataKey::IsPaused, &val);
    }

    pub fn get_pause_history(env: &Env) -> soroban_sdk::Vec<PauseRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::PauseHistory)
            .unwrap_or_else(|| soroban_sdk::Vec::new(env))
    }

    pub fn append_pause_record(env: &Env, record: &PauseRecord) {
        let mut history = Self::get_pause_history(env);
        history.push_back(record.clone());
        env.storage()
            .persistent()
            .set(&DataKey::PauseHistory, &history);
    }

    pub fn set_latest_pause_unpaused_at(env: &Env, timestamp: u64) {
        let mut history = Self::get_pause_history(env);
        let len = history.len();
        if len == 0 {
            return;
        }
        let mut last = history.get(len - 1).unwrap();
        last.unpaused_at = Some(timestamp);
        history.set(len - 1, last);
        env.storage()
            .persistent()
            .set(&DataKey::PauseHistory, &history);
    }
}
