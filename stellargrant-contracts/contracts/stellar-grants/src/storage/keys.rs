use crate::types::{ProtocolModule, RateLimitAction, Role, WhitelistScope};
use soroban_sdk::{contracttype, Address, Bytes, Symbol};

// ── Domain sub-enums ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum GrantKey {
    Data(u64),
    Counter,
    CounterValue,
    AuditLog(u64),
    Tags(u64),
    TagIndex(u32),
    CategoryList,
    SpecVersion(u64, u32),
    CurrentVersion(u64),
    Amendment(u64, u32),
    AmendmentHistory(u64),
    Transfer(u64),
    Renewal(u64),
    RenewalHistory(u64),
    Fork(u64),
    ForkChildren(u64),
    Syndicate(u64),
    SyndicateMember(u64, Address),
    SyndicateMembers(u64),
    SyndicatePayouts(u64, u32),
    OwnerIndex(Address),
    StatusIndex(u32),
    TokenIndex(Address),
    ContribIndex(Address),
    GlobalOrder,
}

#[contracttype]
#[derive(Clone)]
pub enum MilestoneKey {
    Data(u64, u32),
    Checklist(u64, u32),
    Submission(u64, u32),
    Dag(u64),
    Nft(u64, u32),
    NftCounter,
    NftsByOwner(Address),
    NftTokenIndex(u32),
    ReputationApplied(u64, u32),
    MerkleCommit(u64, u32),
    Extension(u64, u32),
    ExtensionHistory(u64),
    Invoice(u64, u32),
    PaymentSplit(u64, u32),
    EvidenceSchema(u64, u32),
    StructuredEvidence(u64, u32),
    PublicReviews(u64, u32),
    ReviewerRecord(Address, u64, u32),
    License(u64, u32),
    Dispute(u64, u32),
}

#[contracttype]
#[derive(Clone)]
pub enum EscrowKey {
    Account(u64),
    State(u64),
    FunderContrib(u64, Address),
    FundersList(u64),
}

#[contracttype]
#[derive(Clone)]
pub enum UserKey {
    Profile(Address),
    RegistryIndex,
    GrantIds(Address),
    ReviewerProfile(Address),
    ReviewerRequest(u64, Address),
    ReviewerRep(Address),
    ReviewerStake(u64, Address),
    ReviewerAllowlist,
    FunderGrants(Address),
    MatchingContrib(Address),
}

#[contracttype]
#[derive(Clone)]
pub enum VotingKey {
    VoiceCredits(Address, u64),
    Mechanism(u64),
    QvVotes(u64, u32),
    MultisigSigners(u64),
    ReleaseApproval(u64, Address),
    Proposal(u32),
    ProposalCounter,
}

#[contracttype]
#[derive(Clone)]
pub enum InsuranceKey {
    Pool(Address),
    Policy(u64),
    Claim(u32),
    ClaimCounter,
}

#[contracttype]
#[derive(Clone)]
pub enum CrowdfundKey {
    Campaign(u64),
    Pledge(u64, Address),
    Backers(u64),
    Counter,
}

#[contracttype]
#[derive(Clone)]
pub enum ArbitrationKey {
    Pool,
    PoolToken,
    Arbiter(Address),
    ActiveCases(Address),
    Case(u32),
    CaseByDispute(u32),
    CaseCounter,
    Settled(u32),
    Vote(u32, Address),
}

#[contracttype]
#[derive(Clone)]
pub enum BondKey {
    Bond(u64),
    BondGrant(u32),
    BondClaim(u32),
    Counter,
}

#[contracttype]
#[derive(Clone)]
pub enum CollateralKey {
    Requirement(u64),
    Deposit(u64, Address),
}

#[contracttype]
#[derive(Clone)]
pub enum ProvenanceKey {
    Record(u32),
    Counter,
    Index(Address),
    ByGrant(u64),
}

#[contracttype]
#[derive(Clone)]
pub enum ReviewerRewardKey {
    Pool(Address),
    Participation(Address, u64),
    RewardRecord(Address, Address),
}

#[contracttype]
#[derive(Clone)]
pub enum MatchingKey {
    Round(u32),
    Contribution(u32, Address, u64),
    Pool(u32),
    Counter,
}

// ── Structured DataKey ────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    // Domain sub-enums
    Grant(GrantKey),
    Milestone(MilestoneKey),
    Escrow(EscrowKey),
    User(UserKey),
    Voting(VotingKey),
    Insurance(InsuranceKey),
    Crowdfund(CrowdfundKey),
    Arbitration(ArbitrationKey),
    Bond(BondKey),
    Collateral(CollateralKey),
    ConditionalRelease(ConditionalReleaseKey),
    AutoApprove(AutoApproveKey),
    GrantTimer(GrantTimerKey),

    // Streaming
    Stream(u32),
    StreamCounter,

    // Protocol singletons
    Admin,
    GlobalAdmin,
    Treasury,
    Council,
    IdentityOracle,
    MinReviewerStake,
    ContractVersion,
    MigrationLog,
    IsPaused,
    PauseHistory,
    Config,
    OracleConfig,
    Metrics,
    AnalyticsSnapshot,
    ParamKeys,
    ComplianceVerifier,
    ScoringRubricCounter,
    DexConfig,
    RelayConfig,

    // Per-address
    TokenMetrics(Address),
    FeesCollected(Address),
    RelayAllowance(Address),
    RelayNonce(Address),
    ComplianceAttestation(Address),
    ReferralRecord(Address),
    ReferralRewards(Address, Address),

    // Domain-keyed singletons
    HookRegistry(u32),
    ScoringRubric(u32),
    BreakerState(ProtocolModule),
    RateLimit(Address, RateLimitAction),
    RollingWindow(Symbol),
    Param(Symbol),
    ParamHistory(Symbol),
    RoleAssignment(Address, Role),
    RoleMembers(Role),
    ReferralCode(Bytes),
    WhitelistEntries(WhitelistScope),
    WhitelistMode(WhitelistScope),

    // Notifications
    NotifSub(Address, u32, u32, u128),
    NotifSubList(u32, u32),

    // Issue #609: Lockup
    Lockup(u64, u32),

    // Issue #619: Data export timestamps
    GrantLastUpdated(u64),

    // Issue #619: Data export state fingerprint
    GlobalLastUpdated,

    // Per-grant pause
    GrantPaused(u64),

    EscrowReleaseRequest(u64, u32),

    BridgeRelayer(Address),
    CrossChainProof(u64, u32),

    // Migration guard
    V2KeysMigrated,
}

// ── Legacy DataKey (v1) — used only by migrate_storage_keys_v2 ───────────────
//
// Variant names and positions MUST match the original DataKey exactly so that
// Soroban's discriminant-based XDR encoding resolves to the same storage keys.

#[contracttype]
#[derive(Clone)]
pub enum LegacyDataKey {
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
    ContractVersion,
    MigrationLog,
    ContributorIndex,
    ReviewerAllowlist,
    AuditLog(u64),
    IsPaused,
    PauseHistory,
    Stream(u32),
    StreamCounter,
    VoiceCredits(Address, u64),
    VotingMechanism(u64),
    QvVotes(u64, u32),
    InsurancePool(Address),
    InsurancePolicy(u64),
    InsuranceClaim(u32),
    InsuranceClaimCounter,
    HookRegistry(u32),
    MilestoneReputationApplied(u64, u32),
    DisputeRecord(u64, u32),
    ProtocolConfig,
    FeesCollected(Address),
    OracleConfig,
    EscrowAccount(u64),
    FunderContribution(u64, Address),
    EscrowFundersList(u64),
    MultisigProposal(u32),
    MultisigProposalCounter,
    ProtocolMetrics,
    TokenMetrics(Address),
    ComplianceAttestation(Address),
    ComplianceVerifier,
    RelayConfig,
    RelayAllowance(Address),
    RelayNonce(Address),
    ReviewerProfile(Address),
    ReviewerRequest(u64, Address),
    GrantTags(u64),
    TagIndex(u32),
    CategoryList,
    RenewalProposal(u64),
    RenewalHistory(u64),
    DexConfig,
    MilestoneChecklist(u64, u32),
    ChecklistSubmission(u64, u32),
    ScoringRubric(u32),
    ScoringRubricCounter,
    BreakerState(ProtocolModule),
    MerkleCommitment(u64, u32),
    RateLimit(Address, RateLimitAction),
    Invoice(u64, u32),
    RollingWindow(Symbol),
    AnalyticsSnapshot,
    Param(Symbol),
    ParamHistory(Symbol),
    ParamKeys,
    RoleAssignment(Address, Role),
    RoleMembers(Role),
    CrowdfundCampaign(u64),
    CrowdfundPledge(u64, Address),
    CrowdfundBackers(u64),
    CrowdfundCounter,
    LicenseRecord(u64, u32),
    PaymentSplit(u64, u32),
    SyndicateGrant(u64),
    SyndicateMember(u64, Address),
    SyndicateMembers(u64),
    SyndicatePayouts(u64, u32),
    GrantVersion(u64, u32),
    CurrentVersion(u64),
    Amendment(u64, u32),
    AmendmentHistory(u64),
    TransferProposal(u64),
    EvidenceSchema(u64, u32),
    StructuredEvidence(u64, u32),
    PublicReviews(u64, u32),
    PublicReviewerRecord(Address, u64, u32),
    MilestoneDag(u64),
    MilestoneNft(u64, u32),
    NftsByAddress(Address),
    NftCounter,
    NftTokenIndex(u32),
    ContributorGrantIds(Address),
    ReferralCode(Bytes),
    ReferralRecord(Address),
    ReferralRewards(Address, Address),
    ExtensionRequest(u64, u32),
    ExtensionHistory(u64),
    ArbiterPool,
    ArbiterPoolToken,
    Arbiter(Address),
    ArbiterActiveCases(Address),
    ArbitrationCase(u32),
    ArbitrationCaseByDispute(u32),
    ArbitrationCaseCounter,
    ArbitrationSettled(u32),
    ArbiterVote(u32, Address),
    PerformanceBond(u64),
    BondGrant(u32),
    BondClaim(u32),
    BondCounter,
    CollateralRequirement(u64),
    CollateralDeposit(u64, Address),
    WhitelistEntries(WhitelistScope),
    WhitelistMode(WhitelistScope),
    FunderGrantIndex(Address),
    MatchingContribution(Address),
    GrantCounterValue,
    IndexByOwner(Address),
    IndexByStatus(u32),
    IndexByToken(Address),
    IndexByContributor(Address),
    GlobalGrantOrder,
    ForkRecord(u64),
    ForkChildren(u64),
    NotifSub(Address, u32, u32, u128),
    NotifSubList(u32, u32),
}
