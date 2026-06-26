use crate::types::{
    AcceptanceCriteria, Amendment, AnalyticsSnapshot, AuditEntry, BreakerState, CategoryStats, ChecklistSubmission, ComplianceAttestation,
    ContractError, ContractVersion, ContributorProfile, CrowdfundCampaign, CrowdfundPledge, DexConfig, Dispute, EscrowAccount,
    EscrowState, EvidenceSchema, FunderLedger, Grant, GrantCategory, GrantTag, GrantVersion, HookEvent, HookRegistration,
    InsuranceClaim, InsurancePolicy, Invoice, LicenseRecord, MerkleCommitment, MigrationRecord, Milestone,
    MilestoneDag, MilestoneNft, MultisigProposal, OracleConfig, ParamRecord, PauseRecord, PaymentSplit, PaymentStream,
    ProtocolConfig, ProtocolMetrics, ProtocolModule, PublicReview, QuadraticVoteRecord,
    RateLimitAction, RateLimitRecord, RegistryEntry, RelayAllowance, RelayConfig, RenewalProposal,
    ReviewerProfile, ReviewerRequest, Role, RoleAssignment, RollingWindow, ScoringRubric, StructuredEvidence, SyndicateGrant,
    SyndicateMember, TokenMetric, TransferProposal, VoiceCredits, VotingMechanism,
};
use crate::types::{
    Arbiter, ArbiterVote, ArbitrationCase, BondClaim, CollateralDeposit, CollateralRequirement,
    ExtensionRequest, PerformanceBond,
    ReferralCode, ReferralRecord,
    WhitelistEntry, WhitelistMode, WhitelistScope,
};
use soroban_sdk::{contracttype, Address, Bytes, Env, Symbol, Vec};

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
    // Issue #529: escrow module
    EscrowAccount(u64),
    FunderContribution(u64, Address),
    EscrowFundersList(u64),
    // Issue #530: multisig module
    MultisigProposal(u32),
    MultisigProposalCounter,
    // Issue #540: protocol metrics
    ProtocolMetrics,
    TokenMetrics(Address),
    // Issue #548: compliance module
    ComplianceAttestation(Address),
    ComplianceVerifier,
    // Issue #585: relay module
    RelayConfig,
    RelayAllowance(Address),
    RelayNonce(Address),
    // Issue #567: reviewer pool module
    ReviewerProfile(Address),
    ReviewerRequest(u64, Address),
    // Issue #571: grant tags module
    GrantTags(u64),
    TagIndex(u32),
    CategoryList,
    // Issue #577: grant renewal module
    RenewalProposal(u64),
    RenewalHistory(u64),
    // Issue #576: token swap DEX config
    DexConfig,
    // Issue #581: milestone checklist
    MilestoneChecklist(u64, u32),
    ChecklistSubmission(u64, u32),
    // Issue #589: scoring rubric
    ScoringRubric(u32),
    ScoringRubricCounter,
    // Issue #594: circuit breaker
    BreakerState(ProtocolModule),
    // Issue #545: Merkle evidence commitments
    MerkleCommitment(u64, u32),
    // Issue #544: per-address rate limits
    RateLimit(Address, RateLimitAction),
    // Issue #566: invoice billing
    Invoice(u64, u32),
    // Issue #582: analytics
    RollingWindow(Symbol),
    AnalyticsSnapshot,
    // Issue #596: dynamic params
    Param(Symbol),
    ParamHistory(Symbol),
    ParamKeys,
    // Issue #593: RBAC
    RoleAssignment(Address, Role),
    RoleMembers(Role),
    // Crowdfund module
    CrowdfundCampaign(u64),
    CrowdfundPledge(u64, Address),
    CrowdfundBackers(u64),
    CrowdfundCounter,
    // Issue #579: IP License Tracking
    LicenseRecord(u64, u32),
    // Issue #592: Multi-Recipient Payment Splitting
    PaymentSplit(u64, u32),
    // Issue #578: Cross-protocol syndication
    SyndicateGrant(u64),
    SyndicateMember(u64, Address),
    SyndicateMembers(u64),
    SyndicatePayouts(u64, u32),
    // Issue #591: Grant specification versioning
    GrantVersion(u64, u32),
    CurrentVersion(u64),
    Amendment(u64, u32),
    AmendmentHistory(u64),
    // Issue #568: Grant Transfer
    TransferProposal(u64),
    // Issue #583: Typed Evidence Schemas
    EvidenceSchema(u64, u32),
    StructuredEvidence(u64, u32),
    // Issue #590: public review
    PublicReviews(u64, u32),
    PublicReviewerRecord(Address, u64, u32),
    // Issue #595: milestone DAG
    MilestoneDag(u64),
    // Issue #570: milestone NFT
    MilestoneNft(u64, u32),
    NftsByAddress(Address),
    NftCounter,
    NftTokenIndex(u32),
    // Issue #565: contributor portfolio grant index
    ContributorGrantIds(Address),
    // Issue #569: referral system
    ReferralCode(Bytes),
    ReferralRecord(Address),
    ReferralRewards(Address, Address), // (referrer, token) -> pending amount
    // Issue #572: deadline extension requests
    ExtensionRequest(u64, u32),
    ExtensionHistory(u64),
    // Issue #573: community arbitration pool
    ArbiterPool,
    ArbiterPoolToken,
    Arbiter(Address),
    ArbiterActiveCases(Address),
    ArbitrationCase(u32),
    ArbitrationCaseByDispute(u32),
    ArbitrationCaseCounter,
    ArbitrationSettled(u32),
    ArbiterVote(u32, Address),
    // Issue #574: performance bonds
    PerformanceBond(u64),
    BondGrant(u32),
    BondClaim(u32),
    BondCounter,
    // Issue #528: math helpers — no storage keys needed (pure functions)
    // Issue #564: collateral escrow
    CollateralRequirement(u64),
    CollateralDeposit(u64, Address),
    // Issue #512: whitelist
    WhitelistEntries(WhitelistScope),
    WhitelistMode(WhitelistScope),
    // Issue #598: funder report — leverages existing escrow/insurance keys
    FunderGrantIndex(Address),
    MatchingContribution(Address),
    GrantCounterValue,
    // Issue #597: efficient grant index
    IndexByOwner(Address),
    IndexByStatus(u32),
    IndexByToken(Address),
    IndexByContributor(Address),
    GlobalGrantOrder,
    // Issue #587: grant forking
    ForkRecord(u64),
    ForkChildren(u64),
    // Issue #580: notification subscriptions
    NotifSub(Address, u32, u32, u128),
    NotifSubList(u32, u32),
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
        env.storage().persistent().set(&DataKey::IsPaused, &paused);
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
        env.storage().persistent().set(&DataKey::StreamCounter, &id);
        id
    }

    pub fn get_stream(env: &Env, stream_id: u32) -> Option<PaymentStream> {
        env.storage().persistent().get(&DataKey::Stream(stream_id))
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

    // ── Issue #529: Escrow Module ─────────────────────────────────────────────

    pub fn get_escrow_account(env: &Env, grant_id: u64) -> Option<EscrowAccount> {
        let key = DataKey::EscrowAccount(grant_id);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_escrow_account(env: &Env, grant_id: u64, account: &EscrowAccount) {
        let key = DataKey::EscrowAccount(grant_id);
        env.storage().persistent().set(&key, account);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_funder_ledger(env: &Env, grant_id: u64, funder: &Address) -> Option<FunderLedger> {
        env.storage()
            .persistent()
            .get(&DataKey::FunderContribution(grant_id, funder.clone()))
    }

    pub fn set_funder_ledger(env: &Env, grant_id: u64, funder: &Address, ledger: &FunderLedger) {
        let key = DataKey::FunderContribution(grant_id, funder.clone());
        env.storage().persistent().set(&key, ledger);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_escrow_funders_list(env: &Env, grant_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::EscrowFundersList(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_escrow_funders_list(env: &Env, grant_id: u64, list: &Vec<Address>) {
        let key = DataKey::EscrowFundersList(grant_id);
        env.storage().persistent().set(&key, list);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #530: Multisig Module ───────────────────────────────────────────

    pub fn next_multisig_proposal_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::MultisigProposalCounter)
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::MultisigProposalCounter, &id);
        id
    }

    pub fn get_multisig_proposal(env: &Env, proposal_id: u32) -> Option<MultisigProposal> {
        let key = DataKey::MultisigProposal(proposal_id);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_multisig_proposal(env: &Env, proposal: &MultisigProposal) {
        let key = DataKey::MultisigProposal(proposal.id);
        env.storage().persistent().set(&key, proposal);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #540: Protocol Metrics ──────────────────────────────────────────

    pub fn get_protocol_metrics(env: &Env) -> Option<ProtocolMetrics> {
        env.storage().persistent().get(&DataKey::ProtocolMetrics)
    }

    pub fn set_protocol_metrics(env: &Env, metrics: &ProtocolMetrics) {
        env.storage()
            .persistent()
            .set(&DataKey::ProtocolMetrics, metrics);
    }

    pub fn get_token_metrics(env: &Env, token: &Address) -> Option<TokenMetric> {
        env.storage()
            .persistent()
            .get(&DataKey::TokenMetrics(token.clone()))
    }

    pub fn set_token_metrics(env: &Env, metrics: &TokenMetric) {
        env.storage()
            .persistent()
            .set(&DataKey::TokenMetrics(metrics.token.clone()), metrics);
    }

    // ── Issue #548: Compliance Module ─────────────────────────────────────────

    pub fn get_compliance_attestation(
        env: &Env,
        address: &Address,
    ) -> Option<ComplianceAttestation> {
        env.storage()
            .persistent()
            .get(&DataKey::ComplianceAttestation(address.clone()))
    }

    pub fn set_compliance_attestation(env: &Env, attestation: &ComplianceAttestation) {
        let key = DataKey::ComplianceAttestation(attestation.subject.clone());
        env.storage().persistent().set(&key, attestation);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn remove_compliance_attestation(env: &Env, address: &Address) {
        env.storage()
            .persistent()
            .remove(&DataKey::ComplianceAttestation(address.clone()));
    }

    pub fn get_compliance_verifier(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::ComplianceVerifier)
    }

    pub fn set_compliance_verifier(env: &Env, verifier: &Address) {
        env.storage()
            .persistent()
            .set(&DataKey::ComplianceVerifier, verifier);
    }

    // ── Issue #585: Relay Module ──────────────────────────────────────────────

    pub fn get_relay_config(env: &Env) -> Option<RelayConfig> {
        env.storage().persistent().get(&DataKey::RelayConfig)
    }

    pub fn set_relay_config(env: &Env, config: &RelayConfig) {
        env.storage().persistent().set(&DataKey::RelayConfig, config);
    }

    pub fn get_relay_allowance(env: &Env, address: &Address) -> Option<RelayAllowance> {
        env.storage()
            .persistent()
            .get(&DataKey::RelayAllowance(address.clone()))
    }

    pub fn set_relay_allowance(env: &Env, allowance: &RelayAllowance) {
        env.storage().persistent().set(
            &DataKey::RelayAllowance(allowance.address.clone()),
            allowance,
        );
    }

    pub fn get_relay_nonce(env: &Env, address: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::RelayNonce(address.clone()))
            .unwrap_or(0)
    }

    pub fn set_relay_nonce(env: &Env, address: &Address, nonce: u32) {
        env.storage()
            .persistent()
            .set(&DataKey::RelayNonce(address.clone()), &nonce);
    }

    // ── Issue #567: Reviewer Pool Module ──────────────────────────────────────

    pub fn get_reviewer_profile(env: &Env, reviewer: &Address) -> Option<ReviewerProfile> {
        env.storage()
            .persistent()
            .get(&DataKey::ReviewerProfile(reviewer.clone()))
    }

    pub fn set_reviewer_profile(env: &Env, profile: &ReviewerProfile) {
        env.storage().persistent().set(
            &DataKey::ReviewerProfile(profile.reviewer.clone()),
            profile,
        );
    }

    pub fn get_reviewer_request(env: &Env, grant_id: u64, reviewer: &Address) -> Option<ReviewerRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::ReviewerRequest(grant_id, reviewer.clone()))
    }

    pub fn set_reviewer_request(env: &Env, request: &ReviewerRequest) {
        env.storage().persistent().set(
            &DataKey::ReviewerRequest(request.grant_id, request.reviewer.clone()),
            request,
        );
    }

    // ── Issue #571: Grant Tags Module ─────────────────────────────────────────

    pub fn get_grant_tags(env: &Env, grant_id: u64) -> Option<GrantTag> {
        env.storage().persistent().get(&DataKey::GrantTags(grant_id))
    }

    pub fn set_grant_tags(env: &Env, tags: &GrantTag) {
        env.storage()
            .persistent()
            .set(&DataKey::GrantTags(tags.grant_id), tags);
    }

    pub fn get_tag_index(env: &Env, tag_hash: u32) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::TagIndex(tag_hash))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_tag_index(env: &Env, tag_hash: u32, grant_ids: &Vec<u64>) {
        env.storage()
            .persistent()
            .set(&DataKey::TagIndex(tag_hash), grant_ids);
    }

    pub fn get_category_list(env: &Env) -> Vec<GrantCategory> {
        env.storage()
            .persistent()
            .get(&DataKey::CategoryList)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_category_list(env: &Env, categories: &Vec<GrantCategory>) {
        env.storage().persistent().set(&DataKey::CategoryList, categories);
    }

    // ── Issue #577: Grant Renewal Module ──────────────────────────────────────

    pub fn get_renewal_proposal(env: &Env, original_grant_id: u64) -> Option<RenewalProposal> {
        env.storage()
            .persistent()
            .get(&DataKey::RenewalProposal(original_grant_id))
    }

    pub fn set_renewal_proposal(env: &Env, proposal: &RenewalProposal) {
        env.storage().persistent().set(
            &DataKey::RenewalProposal(proposal.original_grant_id),
            proposal,
        );
    }

    pub fn get_renewal_history(env: &Env, grant_id: u64) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::RenewalHistory(grant_id))
    }

    pub fn set_renewal_history(env: &Env, grant_id: u64, original_grant_id: u64) {
        env.storage()
            .persistent()
            .set(&DataKey::RenewalHistory(grant_id), &original_grant_id);
    }

    // ── Issue #576: Token Swap ──────────────────────────────────────────────────

    pub fn get_dex_config(env: &Env) -> Option<DexConfig> {
        env.storage().persistent().get(&DataKey::DexConfig)
    }

    pub fn set_dex_config(env: &Env, config: &DexConfig) {
        env.storage().persistent().set(&DataKey::DexConfig, config);
    }

    // ── Issue #581: Milestone Checklist ─────────────────────────────────────────

    pub fn get_milestone_checklist(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Vec<AcceptanceCriteria>> {
        env.storage()
            .persistent()
            .get(&DataKey::MilestoneChecklist(grant_id, milestone_idx))
    }

    pub fn set_milestone_checklist(env: &Env, grant_id: u64, milestone_idx: u32, criteria: &Vec<AcceptanceCriteria>) {
        let key = DataKey::MilestoneChecklist(grant_id, milestone_idx);
        env.storage().persistent().set(&key, criteria);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_checklist_submission(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<ChecklistSubmission> {
        env.storage()
            .persistent()
            .get(&DataKey::ChecklistSubmission(grant_id, milestone_idx))
    }

    pub fn set_checklist_submission(env: &Env, submission: &ChecklistSubmission) {
        let key = DataKey::ChecklistSubmission(submission.grant_id, submission.milestone_idx);
        env.storage().persistent().set(&key, submission);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #589: Scoring ─────────────────────────────────────────────────────

    pub fn next_rubric_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ScoringRubricCounter)
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::ScoringRubricCounter, &id);
        id
    }

    pub fn get_scoring_rubric(env: &Env, rubric_id: u32) -> Option<ScoringRubric> {
        env.storage()
            .persistent()
            .get(&DataKey::ScoringRubric(rubric_id))
    }

    pub fn set_scoring_rubric(env: &Env, rubric: &ScoringRubric) {
        env.storage()
            .persistent()
            .set(&DataKey::ScoringRubric(rubric.id), rubric);
    }

    // ── Issue #594: Circuit Breaker ─────────────────────────────────────────────

    pub fn get_breaker_state(env: &Env, module: &ProtocolModule) -> Option<BreakerState> {
        env.storage()
            .persistent()
            .get(&DataKey::BreakerState(module.clone()))
    }

    pub fn set_breaker_state(env: &Env, state: &BreakerState) {
        env.storage()
            .persistent()
            .set(&DataKey::BreakerState(state.module.clone()), state);
    }

    pub fn remove_breaker(env: &Env, module: &ProtocolModule) {
        env.storage()
            .persistent()
            .remove(&DataKey::BreakerState(module.clone()));
    }

    // ── Issue #545: Merkle commitments ────────────────────────────────────────

    pub fn get_merkle_commitment(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<MerkleCommitment> {
        env.storage()
            .persistent()
            .get(&DataKey::MerkleCommitment(grant_id, milestone_idx))
    }

    pub fn set_merkle_commitment(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        commitment: &MerkleCommitment,
    ) {
        let key = DataKey::MerkleCommitment(grant_id, milestone_idx);
        env.storage().persistent().set(&key, commitment);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #544: Rate limits ─────────────────────────────────────────────────

    pub fn get_rate_limit_record(
        env: &Env,
        address: &Address,
        action: &RateLimitAction,
    ) -> Option<RateLimitRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::RateLimit(address.clone(), action.clone()))
    }

    pub fn set_rate_limit_record(
        env: &Env,
        address: &Address,
        action: &RateLimitAction,
        record: &RateLimitRecord,
    ) {
        let key = DataKey::RateLimit(address.clone(), action.clone());
        env.storage().persistent().set(&key, record);
        Self::bump_persistent_ttl(env, &key);
    }
}

impl Storage {
    // ── Issue #566: Invoice Billing ─────────────────────────────────────────────

    pub fn get_invoice(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Invoice> {
        env.storage()
            .persistent()
            .get(&DataKey::Invoice(grant_id, milestone_idx))
    }

    pub fn set_invoice(env: &Env, grant_id: u64, milestone_idx: u32, invoice: &Invoice) {
        let key = DataKey::Invoice(grant_id, milestone_idx);
        env.storage().persistent().set(&key, invoice);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #582: Analytics ────────────────────────────────────────────────────

    pub fn get_rolling_window(env: &Env, metric: &Symbol) -> Option<RollingWindow> {
        env.storage()
            .persistent()
            .get(&DataKey::RollingWindow(metric.clone()))
    }

    pub fn set_rolling_window(env: &Env, metric: &Symbol, window: &RollingWindow) {
        let key = DataKey::RollingWindow(metric.clone());
        env.storage().persistent().set(&key, window);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_analytics_snapshot(env: &Env) -> Option<AnalyticsSnapshot> {
        env.storage().persistent().get(&DataKey::AnalyticsSnapshot)
    }

    pub fn set_analytics_snapshot(env: &Env, snapshot: &AnalyticsSnapshot) {
        env.storage()
            .persistent()
            .set(&DataKey::AnalyticsSnapshot, snapshot);
    }

    // ── Issue #596: Dynamic Params ────────────────────────────────────────────────

    pub fn get_param(env: &Env, key: &Symbol) -> Option<ParamRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Param(key.clone()))
    }

    pub fn set_param(env: &Env, key: &Symbol, record: &ParamRecord) {
        let data_key = DataKey::Param(key.clone());
        env.storage().persistent().set(&data_key, record);
        Self::bump_persistent_ttl(env, &data_key);

        // Add to param keys list if not exists
        let mut keys = Self::list_param_keys(env);
        if !keys.contains(key.clone()) {
            keys.push_back(key.clone());
            env.storage().persistent().set(&DataKey::ParamKeys, &keys);
        }
    }

    pub fn get_param_history(env: &Env, key: &Symbol) -> Vec<ParamRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::ParamHistory(key.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_param_history(env: &Env, key: &Symbol, history: &Vec<ParamRecord>) {
        let data_key = DataKey::ParamHistory(key.clone());
        env.storage().persistent().set(&data_key, history);
        Self::bump_persistent_ttl(env, &data_key);
    }

    pub fn list_param_keys(env: &Env) -> Vec<Symbol> {
        env.storage()
            .persistent()
            .get(&DataKey::ParamKeys)
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── Issue #593: RBAC ──────────────────────────────────────────────────────────

    pub fn get_role_assignment(env: &Env, address: &Address, role: &Role) -> Option<RoleAssignment> {
        env.storage()
            .persistent()
            .get(&DataKey::RoleAssignment(address.clone(), role.clone()))
    }

    pub fn set_role_assignment(env: &Env, address: &Address, role: &Role, assignment: &RoleAssignment) {
        let key = DataKey::RoleAssignment(address.clone(), role.clone());
        env.storage().persistent().set(&key, assignment);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_role_members(env: &Env, role: &Role) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::RoleMembers(role.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_role_members(env: &Env, role: &Role, members: &Vec<Address>) {
        let key = DataKey::RoleMembers(role.clone());
        env.storage().persistent().set(&key, members);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Crowdfund Module ──────────────────────────────────────────────────────

    pub fn next_crowdfund_id(env: &Env) -> u64 {
        let mut id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::CrowdfundCounter)
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::CrowdfundCounter, &id);
        id
    }

    pub fn get_crowdfund_campaign(env: &Env, id: u64) -> Option<CrowdfundCampaign> {
        let key = DataKey::CrowdfundCampaign(id);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_crowdfund_campaign(env: &Env, campaign: &CrowdfundCampaign) {
        let key = DataKey::CrowdfundCampaign(campaign.id);
        env.storage().persistent().set(&key, campaign);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_crowdfund_pledge(
        env: &Env,
        campaign_id: u64,
        backer: &Address,
    ) -> Option<CrowdfundPledge> {
        let key = DataKey::CrowdfundPledge(campaign_id, backer.clone());
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_crowdfund_pledge(env: &Env, pledge: &CrowdfundPledge) {
        let key = DataKey::CrowdfundPledge(pledge.campaign_id, pledge.backer.clone());
        env.storage().persistent().set(&key, pledge);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_crowdfund_backers(env: &Env, campaign_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::CrowdfundBackers(campaign_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_crowdfund_backers(env: &Env, campaign_id: u64, backers: &Vec<Address>) {
        let key = DataKey::CrowdfundBackers(campaign_id);
        env.storage().persistent().set(&key, backers);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #579: IP License Tracking ──────────────────────────────────────

    pub fn get_license_record(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<LicenseRecord> {
        let key = DataKey::LicenseRecord(grant_id, milestone_idx);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_license_record(env: &Env, grant_id: u64, milestone_idx: u32, record: &LicenseRecord) {
        let key = DataKey::LicenseRecord(grant_id, milestone_idx);
        env.storage().persistent().set(&key, record);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #592: Multi-Recipient Payment Splitting ─────────────────────────

    pub fn get_payment_split(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<PaymentSplit> {
        let key = DataKey::PaymentSplit(grant_id, milestone_idx);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_payment_split(env: &Env, grant_id: u64, milestone_idx: u32, split: &PaymentSplit) {
        let key = DataKey::PaymentSplit(grant_id, milestone_idx);
        env.storage().persistent().set(&key, split);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #578: Cross-Protocol Grant Syndication ─────────────────────────

    pub fn get_syndicate_grant(env: &Env, grant_id: u64) -> Option<SyndicateGrant> {
        let key = DataKey::SyndicateGrant(grant_id);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_syndicate_grant(env: &Env, grant_id: u64, syndicate: &SyndicateGrant) {
        let key = DataKey::SyndicateGrant(grant_id);
        env.storage().persistent().set(&key, syndicate);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_syndicate_member(env: &Env, grant_id: u64, member: &Address) -> Option<SyndicateMember> {
        env.storage()
            .persistent()
            .get(&DataKey::SyndicateMember(grant_id, member.clone()))
    }

    pub fn set_syndicate_member(env: &Env, grant_id: u64, member: &Address, record: &SyndicateMember) {
        let key = DataKey::SyndicateMember(grant_id, member.clone());
        env.storage().persistent().set(&key, record);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn remove_syndicate_member(env: &Env, grant_id: u64, member: &Address) {
        env.storage()
            .persistent()
            .remove(&DataKey::SyndicateMember(grant_id, member.clone()));
    }

    pub fn get_syndicate_member_index(env: &Env, grant_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::SyndicateMembers(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_syndicate_member_index(env: &Env, grant_id: u64, members: &Vec<Address>) {
        let key = DataKey::SyndicateMembers(grant_id);
        env.storage().persistent().set(&key, members);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn set_syndicate_payouts(env: &Env, grant_id: u64, milestone_idx: u32, payouts: &Vec<(Address, i128)>) {
        let key = DataKey::SyndicatePayouts(grant_id, milestone_idx);
        env.storage().persistent().set(&key, payouts);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #591: Grant Specification Versioning ───────────────────────────

    pub fn get_grant_version(env: &Env, grant_id: u64, version: u32) -> Option<GrantVersion> {
        let key = DataKey::GrantVersion(grant_id, version);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_grant_version(env: &Env, grant_id: u64, version: u32, snapshot: &GrantVersion) {
        let key = DataKey::GrantVersion(grant_id, version);
        env.storage().persistent().set(&key, snapshot);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_current_version(env: &Env, grant_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::CurrentVersion(grant_id))
            .unwrap_or(0)
    }

    pub fn set_current_version(env: &Env, grant_id: u64, version: u32) {
        let key = DataKey::CurrentVersion(grant_id);
        env.storage().persistent().set(&key, &version);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_amendment(env: &Env, grant_id: u64, version: u32) -> Option<Amendment> {
        env.storage()
            .persistent()
            .get(&DataKey::Amendment(grant_id, version))
    }

    pub fn set_amendment(env: &Env, grant_id: u64, version: u32, amendment: &Amendment) {
        let key = DataKey::Amendment(grant_id, version);
        env.storage().persistent().set(&key, amendment);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_amendment_history(env: &Env, grant_id: u64) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::AmendmentHistory(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_amendment_history(env: &Env, grant_id: u64, history: &Vec<u32>) {
        let key = DataKey::AmendmentHistory(grant_id);
        env.storage().persistent().set(&key, history);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #568: Grant Transfer ────────────────────────────────────────────

    pub fn get_transfer_proposal(env: &Env, grant_id: u64) -> Option<TransferProposal> {
        let key = DataKey::TransferProposal(grant_id);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_transfer_proposal(env: &Env, grant_id: u64, proposal: &TransferProposal) {
        let key = DataKey::TransferProposal(grant_id);
        env.storage().persistent().set(&key, proposal);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn remove_transfer_proposal(env: &Env, grant_id: u64) {
        env.storage().persistent().remove(&DataKey::TransferProposal(grant_id));
    }

    // ── Issue #583: Typed Evidence Schemas ───────────────────────────────────

    pub fn get_evidence_schema(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<EvidenceSchema> {
        let key = DataKey::EvidenceSchema(grant_id, milestone_idx);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_evidence_schema(env: &Env, grant_id: u64, milestone_idx: u32, schema: &EvidenceSchema) {
        let key = DataKey::EvidenceSchema(grant_id, milestone_idx);
        env.storage().persistent().set(&key, schema);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_structured_evidence(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<StructuredEvidence> {
        let key = DataKey::StructuredEvidence(grant_id, milestone_idx);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_structured_evidence(env: &Env, grant_id: u64, milestone_idx: u32, evidence: &StructuredEvidence) {
        let key = DataKey::StructuredEvidence(grant_id, milestone_idx);
        env.storage().persistent().set(&key, evidence);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #590: Public Review ─────────────────────────────────────────────

    pub fn get_public_reviews(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<PublicReview> {
        env.storage()
            .persistent()
            .get(&DataKey::PublicReviews(grant_id, milestone_idx))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_public_reviews(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        reviews: &Vec<PublicReview>,
    ) {
        let key = DataKey::PublicReviews(grant_id, milestone_idx);
        env.storage().persistent().set(&key, reviews);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_public_reviewer_record(
        env: &Env,
        reviewer: &Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<PublicReview> {
        env.storage()
            .persistent()
            .get(&DataKey::PublicReviewerRecord(
                reviewer.clone(),
                grant_id,
                milestone_idx,
            ))
    }

    pub fn set_public_reviewer_record(
        env: &Env,
        reviewer: &Address,
        grant_id: u64,
        milestone_idx: u32,
        review: &PublicReview,
    ) {
        let key = DataKey::PublicReviewerRecord(reviewer.clone(), grant_id, milestone_idx);
        env.storage().persistent().set(&key, review);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #595: Milestone DAG ─────────────────────────────────────────────

    pub fn get_milestone_dag(env: &Env, grant_id: u64) -> Option<MilestoneDag> {
        let key = DataKey::MilestoneDag(grant_id);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_milestone_dag(env: &Env, grant_id: u64, dag: &MilestoneDag) {
        let key = DataKey::MilestoneDag(grant_id);
        env.storage().persistent().set(&key, dag);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #570: Milestone NFT ─────────────────────────────────────────────

    pub fn next_nft_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::NftCounter)
            .unwrap_or(0);
        id += 1;
        env.storage().persistent().set(&DataKey::NftCounter, &id);
        id
    }

    pub fn get_milestone_nft(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<MilestoneNft> {
        let key = DataKey::MilestoneNft(grant_id, milestone_idx);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_milestone_nft(env: &Env, nft: &MilestoneNft) {
        let key = DataKey::MilestoneNft(nft.grant_id, nft.milestone_idx);
        env.storage().persistent().set(&key, nft);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_nft_by_token_id(env: &Env, token_id: u32) -> Option<MilestoneNft> {
        let index: Option<(u64, u32)> = env
            .storage()
            .persistent()
            .get(&DataKey::NftTokenIndex(token_id));
        index.and_then(|(grant_id, milestone_idx)| Self::get_milestone_nft(env, grant_id, milestone_idx))
    }

    pub fn set_nft_token_index(env: &Env, token_id: u32, grant_id: u64, milestone_idx: u32) {
        let key = DataKey::NftTokenIndex(token_id);
        env.storage().persistent().set(&key, &(grant_id, milestone_idx));
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_nfts_by_owner(env: &Env, owner: &Address) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::NftsByAddress(owner.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_nfts_by_owner(env: &Env, owner: &Address, token_ids: &Vec<u32>) {
        let key = DataKey::NftsByAddress(owner.clone());
        env.storage().persistent().set(&key, token_ids);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #565: Contributor Portfolio Grant Index ────────────────────────

    pub fn get_contributor_grant_ids(env: &Env, contributor: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::ContributorGrantIds(contributor.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn push_contributor_grant_id(env: &Env, contributor: &Address, grant_id: u64) {
        let key = DataKey::ContributorGrantIds(contributor.clone());
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        if !ids.contains(grant_id) {
            ids.push_back(grant_id);
            env.storage().persistent().set(&key, &ids);
            Self::bump_persistent_ttl(env, &key);
        }
    }

    // ── Issue #569: Referral System ──────────────────────────────────────────

    pub fn get_referral_code(env: &Env, code_hash: &Bytes) -> Option<ReferralCode> {
        env.storage()
            .persistent()
            .get(&DataKey::ReferralCode(code_hash.clone()))
    }

    pub fn set_referral_code(env: &Env, code: &ReferralCode) {
        let key = DataKey::ReferralCode(code.code_hash.clone());
        env.storage().persistent().set(&key, code);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_referral_record(env: &Env, referred: &Address) -> Option<ReferralRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::ReferralRecord(referred.clone()))
    }

    pub fn set_referral_record(env: &Env, record: &ReferralRecord) {
        let key = DataKey::ReferralRecord(record.referred.clone());
        env.storage().persistent().set(&key, record);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_referral_rewards(env: &Env, referrer: &Address, token: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::ReferralRewards(referrer.clone(), token.clone()))
            .unwrap_or(0)
    }

    pub fn set_referral_rewards(env: &Env, referrer: &Address, token: &Address, amount: i128) {
        let key = DataKey::ReferralRewards(referrer.clone(), token.clone());
        env.storage().persistent().set(&key, &amount);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #572: Deadline Extension Requests ──────────────────────────────

    pub fn get_extension_request(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<ExtensionRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::ExtensionRequest(grant_id, milestone_idx))
    }

    pub fn set_extension_request(env: &Env, req: &ExtensionRequest) {
        let key = DataKey::ExtensionRequest(req.grant_id, req.milestone_idx);
        env.storage().persistent().set(&key, req);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn remove_extension_request(env: &Env, grant_id: u64, milestone_idx: u32) {
        env.storage()
            .persistent()
            .remove(&DataKey::ExtensionRequest(grant_id, milestone_idx));
    }

    pub fn get_extension_history(env: &Env, grant_id: u64) -> Vec<ExtensionRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::ExtensionHistory(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn push_extension_history(env: &Env, req: &ExtensionRequest) {
        let key = DataKey::ExtensionHistory(req.grant_id);
        let mut history = Self::get_extension_history(env, req.grant_id);
        history.push_back(req.clone());
        env.storage().persistent().set(&key, &history);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #573: Community Arbitration Pool ───────────────────────────────

    pub fn get_arbiter_pool(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::ArbiterPool)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_arbiter_pool(env: &Env, pool: &Vec<Address>) {
        env.storage().persistent().set(&DataKey::ArbiterPool, pool);
        Self::bump_persistent_ttl(env, &DataKey::ArbiterPool);
    }

    pub fn get_arbiter(env: &Env, address: &Address) -> Option<Arbiter> {
        env.storage()
            .persistent()
            .get(&DataKey::Arbiter(address.clone()))
    }

    pub fn set_arbiter(env: &Env, arbiter: &Arbiter) {
        let key = DataKey::Arbiter(arbiter.address.clone());
        env.storage().persistent().set(&key, arbiter);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_arbiter_pool_token(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::ArbiterPoolToken)
    }

    pub fn set_arbiter_pool_token(env: &Env, token: &Address) {
        env.storage()
            .persistent()
            .set(&DataKey::ArbiterPoolToken, token);
    }

    pub fn get_arbiter_active_cases(env: &Env, arbiter: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ArbiterActiveCases(arbiter.clone()))
            .unwrap_or(0)
    }

    pub fn set_arbiter_active_cases(env: &Env, arbiter: &Address, count: u32) {
        let key = DataKey::ArbiterActiveCases(arbiter.clone());
        env.storage().persistent().set(&key, &count);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn next_arbitration_case_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ArbitrationCaseCounter)
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::ArbitrationCaseCounter, &id);
        id
    }

    pub fn get_arbitration_case(env: &Env, case_id: u32) -> Option<ArbitrationCase> {
        env.storage()
            .persistent()
            .get(&DataKey::ArbitrationCase(case_id))
    }

    pub fn set_arbitration_case(env: &Env, case: &ArbitrationCase) {
        let key = DataKey::ArbitrationCase(case.id);
        env.storage().persistent().set(&key, case);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_case_id_by_dispute(env: &Env, dispute_id: u32) -> Option<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::ArbitrationCaseByDispute(dispute_id))
    }

    pub fn set_case_id_by_dispute(env: &Env, dispute_id: u32, case_id: u32) {
        let key = DataKey::ArbitrationCaseByDispute(dispute_id);
        env.storage().persistent().set(&key, &case_id);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn is_arbitration_settled(env: &Env, case_id: u32) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::ArbitrationSettled(case_id))
            .unwrap_or(false)
    }

    pub fn set_arbitration_settled(env: &Env, case_id: u32) {
        env.storage()
            .persistent()
            .set(&DataKey::ArbitrationSettled(case_id), &true);
    }

    pub fn get_arbiter_vote(env: &Env, case_id: u32, arbiter: &Address) -> Option<ArbiterVote> {
        env.storage()
            .persistent()
            .get(&DataKey::ArbiterVote(case_id, arbiter.clone()))
    }

    pub fn set_arbiter_vote(env: &Env, case_id: u32, vote: &ArbiterVote) {
        let key = DataKey::ArbiterVote(case_id, vote.arbiter.clone());
        env.storage().persistent().set(&key, vote);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #574: Performance Bonds ────────────────────────────────────────

    pub fn get_performance_bond(env: &Env, grant_id: u64) -> Option<PerformanceBond> {
        env.storage()
            .persistent()
            .get(&DataKey::PerformanceBond(grant_id))
    }

    pub fn set_performance_bond(env: &Env, bond: &PerformanceBond) {
        let key = DataKey::PerformanceBond(bond.grant_id);
        env.storage().persistent().set(&key, bond);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn next_bond_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::BondCounter)
            .unwrap_or(0);
        id += 1;
        env.storage().persistent().set(&DataKey::BondCounter, &id);
        id
    }

    pub fn get_bond_grant(env: &Env, bond_id: u32) -> Option<u64> {
        env.storage().persistent().get(&DataKey::BondGrant(bond_id))
    }

    pub fn set_bond_grant(env: &Env, bond_id: u32, grant_id: u64) {
        let key = DataKey::BondGrant(bond_id);
        env.storage().persistent().set(&key, &grant_id);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_bond_claim(env: &Env, bond_id: u32) -> Option<BondClaim> {
        env.storage()
            .persistent()
            .get(&DataKey::BondClaim(bond_id))
    }

    pub fn set_bond_claim(env: &Env, claim: &BondClaim) {
        let key = DataKey::BondClaim(claim.bond_id);
        env.storage().persistent().set(&key, claim);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #564: Collateral Escrow ───────────────────────────────────────

    pub fn get_collateral_requirement(env: &Env, grant_id: u64) -> Option<CollateralRequirement> {
        let key = DataKey::CollateralRequirement(grant_id);
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_collateral_requirement(env: &Env, grant_id: u64, req: &CollateralRequirement) {
        let key = DataKey::CollateralRequirement(grant_id);
        env.storage().persistent().set(&key, req);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_collateral_deposit(
        env: &Env,
        grant_id: u64,
        contributor: &Address,
    ) -> Option<CollateralDeposit> {
        let key = DataKey::CollateralDeposit(grant_id, contributor.clone());
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        v
    }

    pub fn set_collateral_deposit(
        env: &Env,
        grant_id: u64,
        contributor: &Address,
        deposit: &CollateralDeposit,
    ) {
        let key = DataKey::CollateralDeposit(grant_id, contributor.clone());
        env.storage().persistent().set(&key, deposit);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #512: Whitelist ───────────────────────────────────────────────

    pub fn get_whitelist_entries(env: &Env, scope: &WhitelistScope) -> Vec<WhitelistEntry> {
        env.storage()
            .persistent()
            .get(&DataKey::WhitelistEntries(scope.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_whitelist_entries(env: &Env, scope: &WhitelistScope, entries: &Vec<WhitelistEntry>) {
        let key = DataKey::WhitelistEntries(scope.clone());
        env.storage().persistent().set(&key, entries);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn push_whitelist_entry(env: &Env, scope: &WhitelistScope, entry: &WhitelistEntry) {
        let mut entries = Self::get_whitelist_entries(env, scope);
        entries.push_back(entry.clone());
        Self::set_whitelist_entries(env, scope, &entries);
    }

    pub fn get_whitelist_mode(env: &Env, scope: &WhitelistScope) -> Option<WhitelistMode> {
        env.storage()
            .persistent()
            .get(&DataKey::WhitelistMode(scope.clone()))
    }

    pub fn set_whitelist_mode(env: &Env, scope: &WhitelistScope, mode: WhitelistMode) {
        let key = DataKey::WhitelistMode(scope.clone());
        env.storage().persistent().set(&key, &mode);
        Self::bump_persistent_ttl(env, &key);
    }

    // ── Issue #598: Funder Report ───────────────────────────────────────────

    pub fn get_funder_grant_index(env: &Env, funder: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::FunderGrantIndex(funder.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn push_funder_grant_index(env: &Env, funder: &Address, grant_id: u64) {
        let key = DataKey::FunderGrantIndex(funder.clone());
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        if !ids.contains(grant_id) {
            ids.push_back(grant_id);
            env.storage().persistent().set(&key, &ids);
            Self::bump_persistent_ttl(env, &key);
        }
    }

    pub fn get_matching_contribution(env: &Env, funder: &Address) -> Option<i128> {
        env.storage()
            .persistent()
            .get(&DataKey::MatchingContribution(funder.clone()))
    }

    pub fn get_grant_counter(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::GrantCounter)
            .unwrap_or(0)
    }

    // ── Issue #587: Fork Record ───────────────────────────────────────────────

    pub fn get_fork_record(env: &Env, grant_id: u64) -> Option<super::types::ForkRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::ForkRecord(grant_id))
    }

    pub fn set_fork_record(env: &Env, grant_id: u64, record: &super::types::ForkRecord) {
        env.storage()
            .persistent()
            .set(&DataKey::ForkRecord(grant_id), record);
    }

    pub fn get_fork_children(env: &Env, grant_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::ForkChildren(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_fork_children(env: &Env, grant_id: u64, children: &Vec<u64>) {
        env.storage()
            .persistent()
            .set(&DataKey::ForkChildren(grant_id), children);
    }
}
