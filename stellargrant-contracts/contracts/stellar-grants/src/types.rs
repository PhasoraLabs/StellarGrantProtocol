use soroban_sdk::{contracttype, Address, Bytes, Map, String, Vec};

pub use crate::errors::ContractError;

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationRecord {
    pub from_version: u32,
    pub to_version: u32,
    pub run_by: Address,
    pub run_at: u64,
    pub success: bool,
    pub notes: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub deployed_at: u64,
    pub deployer: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryEntry {
    pub address: Address,
    pub registered_at: u64,
    pub is_active: bool,
    pub entry_type: RegistryEntryType,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RegistryEntryType {
    Contributor = 0,
    Reviewer = 1,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ContributionType {
    GrantCreated = 0,
    MilestoneDelivered = 1,
    MilestoneReviewed = 2,
    GrantFunded = 3,
    DisputeResolved = 4,
    BountyDelivered = 5,
    ArbitrationProvided = 6,
    ReviewerServiceProvided = 7,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceRecord {
    pub id: u32,
    pub contribution_type: ContributionType,
    pub actor: Address,
    pub grant_id: u64,
    pub milestone_idx: Option<u32>,
    pub amount: Option<i128>,
    pub token: Option<Address>,
    pub timestamp: u64,
    pub ledger_sequence: u32,
    pub co_contributors: Vec<Address>,
    pub tags: Vec<String>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum MilestoneState {
    Pending = 0,
    Submitted = 1,
    Approved = 2,
    Rejected = 3,
    Paid = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Milestone {
    pub idx: u32,
    pub description: String,
    pub amount: i128,
    pub state: MilestoneState,
    pub votes: Map<Address, bool>,
    pub approvals: u32,
    pub rejections: u32,
    pub reasons: Map<Address, String>,
    pub status_updated_at: u64,
    pub proof_url: Option<String>,
    pub submission_timestamp: u64,
    /// Optional milestone deadline (ledger timestamp). Updated by approved extensions (#572).
    pub deadline: Option<u64>,
}

#[contracttype]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GrantStatus {
    Active = 1,
    Cancelled = 2,
    Completed = 3,
}

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantFund {
    pub funder: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub id: u64,
    pub owner: Address,
    pub title: String,
    pub description: String,
    pub token: Address,
    pub status: GrantStatus,
    pub total_amount: i128,
    pub milestone_amount: i128,
    pub reviewers: Vec<Address>,
    pub total_milestones: u32,
    pub milestones_paid_out: u32,
    pub escrow_balance: i128,
    pub funders: Vec<GrantFund>,
    pub reason: Option<String>,
    pub timestamp: u64,
    pub require_compliance: Option<u32>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorProfile {
    pub contributor: Address,
    pub name: String,
    pub bio: String,
    pub skills: Vec<String>,
    pub github_url: String,
    pub registration_timestamp: u64,
    pub reputation_score: u64,
    pub grants_count: u32,
    pub total_earned: i128,
    pub milestones_completed: u32,
    pub milestones_rejected: u32,
    pub last_action_at: u64,
}

#[contracttype]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EscrowMode {
    Standard = 1,
    HighSecurity = 2,
}

#[contracttype]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EscrowLifecycleState {
    Funding = 1,
    AwaitingMultisig = 2,
    Released = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EscrowState {
    pub mode: EscrowMode,
    pub lifecycle: EscrowLifecycleState,
    pub quorum_ready: bool,
    pub approvals_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneSubmission {
    pub idx: u32,
    pub description: String,
    pub proof: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AuditAction {
    GrantCreated = 0,
    GrantFunded = 1,
    MilestoneSubmitted = 2,
    MilestoneApproved = 3,
    MilestoneRejected = 4,
    MilestonePaid = 5,
    DisputeRaised = 6,
    DisputeResolved = 7,
    GrantCancelled = 8,
    GrantCompleted = 9,
    AdminChanged = 10,
    ContractPaused = 11,
    ContractUnpaused = 12,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    pub action: AuditAction,
    pub actor: Address,
    pub grant_id: u64,
    pub milestone_idx: Option<u32>,
    pub amount: Option<i128>,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

// ── Emergency Pause (#521) ───────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PauseRecord {
    pub paused_by: Address,
    pub paused_at: u64,
    pub unpaused_at: Option<u64>,
    pub reason: String,
}

// ── Streaming Payments (#531) ─────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamStatus {
    Active = 0,
    Cancelled = 1,
    Completed = 2,
    Paused = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentStream {
    pub id: u32,
    pub grant_id: u64,
    pub sender: Address,
    pub recipient: Address,
    pub token: Address,
    pub rate_per_ledger: i128,
    pub deposited: i128,
    pub withdrawn: i128,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub status: StreamStatus,
    pub created_at: u64,
    /// Ledger at which stream was paused (0 if not paused).
    pub paused_at_ledger: u32,
}

// ── Quadratic Voting (#537) ────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VotingMechanism {
    SimpleMajority = 0,
    Quadratic = 1,
    Weighted = 2,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceCredits {
    pub voter: Address,
    pub grant_id: u64,
    pub total_credits: u32,
    pub spent_credits: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuadraticVoteRecord {
    pub voter: Address,
    pub milestone_idx: u32,
    pub votes_cast: u32,
    pub credits_spent: u32,
    pub in_favor: bool,
}

// ── Grant Insurance Pool (#538) ───────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InsuranceClaimStatus {
    Submitted = 0,
    UnderReview = 1,
    Approved = 2,
    Rejected = 3,
    Paid = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsurancePolicy {
    pub grant_id: u64,
    pub policyholder: Address,
    pub token: Address,
    pub coverage_amount: i128,
    pub premium_paid: i128,
    pub issued_at: u64,
    pub expires_at: u64,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsuranceClaim {
    pub id: u32,
    pub policy_grant_id: u64,
    pub claimant: Address,
    pub claimed_amount: i128,
    pub reason: String,
    pub status: InsuranceClaimStatus,
    pub submitted_at: u64,
    pub resolved_at: Option<u64>,
    pub payout_amount: Option<i128>,
}

// ── External Callback Hooks (#539) ────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HookEvent {
    GrantCreated = 0,
    MilestoneApproved = 1,
    MilestonePaid = 2,
    DisputeRaised = 3,
    DisputeResolved = 4,
    ContributorRegistered = 5,
    BountyAwarded = 6,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookRegistration {
    pub event: HookEvent,
    pub target_contract: Address,
    pub registered_by: Address,
    pub registered_at: u64,
    pub is_active: bool,
    pub max_gas_budget: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookCallResult {
    pub hook_index: u32,
    pub success: bool,
    pub error_code: Option<u32>,
}

/// Opaque byte payload passed to hook callbacks.
#[allow(dead_code)]
pub type HookPayload = Bytes;

// ── Issue #514: Dispute Resolution Module ────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DisputeStatus {
    Open = 0,
    UnderReview = 1,
    ResolvedForContributor = 2,
    ResolvedForFunder = 3,
    Cancelled = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dispute {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub raised_by: Address,
    pub reason: String,
    pub status: DisputeStatus,
    pub arbiters: Vec<Address>,
    pub votes_contributor: u32,
    pub votes_funder: u32,
    pub raised_at: u64,
    pub resolved_at: Option<u64>,
}

// ── Issue #515: Contributor Reputation ───────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ReputationTier {
    Unranked = 0,
    Bronze = 1,
    Silver = 2,
    Gold = 3,
    Platinum = 4,
}

// ── Issue #516: Runtime Protocol Configuration ───────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolConfig {
    pub quorum_threshold_bps: u32,
    pub max_reviewers: u32,
    pub min_stake_amount: i128,
    pub protocol_fee_bps: u32,
    pub max_milestones_per_grant: u32,
    pub dispute_window_ledgers: u32,
    pub max_grant_title_len: u32,
    pub max_grant_desc_len: u32,
    /// Grants with total_amount above this threshold require multisig release (0 = disabled).
    pub multisig_threshold: i128,
    /// Multiplier applied to all default per-action rate limits (1 = use defaults).
    pub rate_limit_multiplier: u32,
    /// Share of the protocol fee paid to referrers, in basis points (#569). Default 1000 = 10%.
    pub referral_fee_bps: u32,
    /// Decay configuration for reputation scores (#575)
    pub decay_config: DecayConfig,
    /// Share of protocol fee directed to reviewer reward pool, in basis points. Default 2000 = 20%.
    pub reviewer_reward_pool_bps: u32,
    /// Bonus in basis points for fast votes (within 1/3 of review window). Default 500 = 5%.
    pub fast_bonus_bps: u32,
    /// Share of protocol fee directed to the staker revenue pool. Default 2000 = 20%.
    pub revenue_share_pool_bps: u32,
}

// ── Issue #631: Revenue Sharing ─────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueEpoch {
    pub id: u32,
    pub start_at: u64,
    pub end_at: u64,
    pub total_revenue: i128,
    pub token: Address,
    pub total_stake_weight: i128,
    pub finalized: bool,
    pub claimed_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StakerEpochRecord {
    pub staker: Address,
    pub epoch_id: u32,
    pub stake_weight: i128,
    pub claimable: i128,
    pub claimed: bool,
    pub claimed_at: Option<u64>,
}

// ── Issue #XXX: Reviewer Reward System ───────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewParticipation {
    pub reviewer: Address,
    pub grant_id: u64,
    pub votes_cast: u32,
    pub fast_votes: u32,
    pub alignment_score: u32,
    pub last_vote_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerRewardRecord {
    pub reviewer: Address,
    pub token: Address,
    pub pending_amount: i128,
    pub total_earned: i128,
    pub last_claimed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerRewardPool {
    pub token: Address,
    pub balance: i128,
    pub total_deposited: i128,
    pub total_paid_out: i128,
}

// ── Issue #517: Protocol Fee Collection ──────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeRecord {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub gross_amount: i128,
    pub fee_amount: i128,
    pub net_amount: i128,
    pub token: Address,
    pub collected_at: u64,
}

// ── Issue #524: Price Oracle Integration ───────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleConfig {
    pub oracle_address: Address,
    pub base_token: Address,
    pub staleness_threshold: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriceQuote {
    pub token: Address,
    pub price_in_base: i128,
    pub fetched_at: u64,
    pub is_stale: bool,
}

// ── Issue #529: Escrow Module ─────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EscrowAccount {
    pub owner: Address,
    pub token: Address,
    pub balance: i128,
    pub total_deposited: i128,
    pub total_released: i128,
    /// True when a dispute is open; blocks release but not deposit.
    pub locked: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunderLedger {
    pub funder: Address,
    pub contributed: i128,
    pub refunded: i128,
    pub last_contribution_at: u64,
}

// ── Issue #530: M-of-N Multi-Signature Fund Release ───────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureStatus {
    Pending = 0,
    Signed = 1,
    Rejected = 2,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultisigSigner {
    pub address: Address,
    pub weight: u32,
    pub status: SignatureStatus,
    pub signed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultisigProposal {
    pub id: u32,
    pub grant_id: u64,
    pub action_payload: Bytes,
    pub signers: Vec<MultisigSigner>,
    pub threshold: u32,
    pub total_weight_signed: u32,
    pub executed: bool,
    pub expired_at: u64,
    pub created_by: Address,
    pub created_at: u64,
}

// ── Issue #540: Protocol-Wide On-Chain Metrics ────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenMetric {
    pub token: Address,
    pub total_locked: i128,
    pub total_paid_out: i128,
    pub total_refunded: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolMetrics {
    pub total_grants_created: u32,
    pub total_grants_active: u32,
    pub total_grants_completed: u32,
    pub total_grants_cancelled: u32,
    pub total_milestones_approved: u32,
    pub total_milestones_rejected: u32,
    pub total_milestones_paid: u32,
    pub total_contributors_registered: u32,
    pub total_disputes_raised: u32,
    pub total_disputes_resolved: u32,
    pub total_bounties_created: u32,
    pub total_bounties_awarded: u32,
    pub last_updated: u64,
}

// ── Issue #548: KYC/AML Compliance Integration Hooks ─────────────────────────

#[contracttype]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ComplianceStatus {
    Unverified = 0,
    Pending = 1,
    Approved = 2,
    Rejected = 3,
    Expired = 4,
}

#[contracttype]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ComplianceLevel {
    None = 0,
    Basic = 1,
    Standard = 2,
    Enhanced = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComplianceAttestation {
    pub subject: Address,
    pub status: ComplianceStatus,
    pub level: ComplianceLevel,
    pub attested_by: Address,
    pub attested_at: u64,
    pub expires_at: u64,
    pub jurisdiction: String,
}

// ── Issue #585: Fee Relayer for Gasless Contributor UX ──────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RelayableAction {
    ContributorRegister = 0,
    MilestoneSubmit = 1,
    ClaimVested = 2,
    WithdrawStream = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayConfig {
    pub enabled: bool,
    pub max_relays_per_address_per_day: u32,
    pub relayer_address: Address,
    pub reimbursement_per_relay: i128,
    pub allowed_actions: Vec<RelayableAction>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayAllowance {
    pub address: Address,
    pub daily_relays_used: u32,
    pub window_start: u64,
    pub total_relayed: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayRecord {
    pub sender: Address,
    pub relayer: Address,
    pub action: RelayableAction,
    pub nonce: u32,
    pub relayed_at: u64,
    pub reimbursement_paid: i128,
}

// ── Issue #567: Decentralized Reviewer Recruitment Marketplace ──────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ReviewerAvailability {
    Available = 0,
    Busy = 1,
    OnLeave = 2,
    Inactive = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerProfile {
    pub reviewer: Address,
    pub display_name: String,
    pub expertise_tags: Vec<String>,
    pub hourly_rate: Option<i128>,
    pub reviews_completed: u32,
    pub average_turnaround_ledgers: u32,
    pub availability: ReviewerAvailability,
    pub registered_at: u64,
    pub reputation_score: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ReviewerRequestStatus {
    Pending = 0,
    Accepted = 1,
    Declined = 2,
    Expired = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerRequest {
    pub grant_id: u64,
    pub reviewer: Address,
    pub requested_by: Address,
    pub message: String,
    pub status: ReviewerRequestStatus,
    pub requested_at: u64,
    pub expires_at: u64,
}

// ── Issue #571: Taxonomy, Category, and Tag System for Grants ──────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantCategory {
    pub id: u32,
    pub name: String,
    pub subcategories: Vec<String>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantTag {
    pub grant_id: u64,
    pub category_id: Option<u32>,
    pub subcategory: Option<String>,
    pub freeform_tags: Vec<String>,
    pub tagged_by: Address,
    pub tagged_at: u64,
}

// ── Issue #577: Automatic and Manual Grant Renewal ────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RenewalStatus {
    Proposed = 0,
    ReviewerApproved = 1,
    Active = 2,
    Declined = 3,
    Expired = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenewalProposal {
    pub original_grant_id: u64,
    pub proposed_by: Address,
    pub new_title: String,
    pub new_description: String,
    pub new_total_amount: i128,
    pub new_num_milestones: u32,
    pub inherit_reviewers: bool,
    pub inherit_contributor: bool,
    pub status: RenewalStatus,
    pub reviewer_votes: u32,
    pub proposed_at: u64,
    pub expires_at: u64,
    pub new_grant_id: Option<u64>,
}

// ── Issue #576: In-Escrow Token Swap ──────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DexConfig {
    pub dex_contract: Address,
    pub max_slippage_bps: u32,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapRoute {
    pub from_token: Address,
    pub to_token: Address,
    pub intermediary: Option<Address>,
    pub min_out: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapResult {
    pub amount_in: i128,
    pub amount_out: i128,
    pub slippage_actual_bps: u32,
    pub dex_contract: Address,
    pub swapped_at: u64,
}

// ── Issue #581: Structured Milestone Acceptance Checklists ────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CriterionStatus {
    Pending = 0,
    CheckedByContributor = 1,
    ApprovedByReviewer = 2,
    RejectedByReviewer = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriteria {
    pub idx: u32,
    pub description: soroban_sdk::String,
    pub is_required: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChecklistSubmission {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub criteria: soroban_sdk::Vec<AcceptanceCriteria>,
    pub statuses: soroban_sdk::Vec<CriterionStatus>,
    pub evidence_urls: soroban_sdk::Vec<Option<soroban_sdk::String>>,
    pub submitted_at: u64,
    pub all_required_met: bool,
}

// ── Issue #589: Pluggable Grant and Contributor Scoring Engine ────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ScoringDimension {
    DeliverySpeed = 0,
    ApprovalRate = 1,
    ReputationScore = 2,
    TotalEarned = 3,
    DisputeRate = 4,
    ReviewerSatisfaction = 5,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoringWeight {
    pub dimension: ScoringDimension,
    pub weight_bps: u32,
    pub invert: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoringRubric {
    pub id: u32,
    pub name: soroban_sdk::String,
    pub weights: soroban_sdk::Vec<ScoringWeight>,
    pub created_by: Address,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreResult {
    pub subject: Address,
    pub rubric_id: u32,
    pub total_score: u32,
    pub dimension_scores: soroban_sdk::Vec<(ScoringDimension, u32)>,
    pub computed_at: u64,
}

// ── Issue #594: Per-Module Fine-Grained Circuit Breakers ──────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ProtocolModule {
    Grants = 0,
    Streaming = 1,
    Bounty = 2,
    Dao = 3,
    Staking = 4,
    Vesting = 5,
    YieldEscrow = 6,
    MatchingPool = 7,
    Crowdfund = 8,
    Insurance = 9,
    Relay = 10,
    TokenSwap = 11,
    Oracle = 12,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakerState {
    pub module: ProtocolModule,
    pub tripped: bool,
    pub tripped_by: Option<Address>,
    pub tripped_at: Option<u64>,
    pub reason: Option<soroban_sdk::String>,
    pub auto_reset_ledger: Option<u32>,
}

// ── Delegation (#603) ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DelegationScope {
    Global,
    PerGrant(u64),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delegation {
    pub delegator: Address,
    pub delegate: Address,
    pub scope: DelegationScope,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub revoked: bool,
    pub uses_remaining: Option<u32>,
}

// ── Badges (#603) ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BadgeType {
    FirstMilestone = 0,
    TenMilestones = 1,
    FiftyMilestones = 2,
    BronzeContributor = 3,
    SilverContributor = 4,
    GoldContributor = 5,
    PlatinumContributor = 6,
    DisputeWinner = 7,
    PerfectGrant = 8,
    BountyChampion = 9,
    EarlyAdopter = 10,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadgeCriteria {
    pub badge_type: BadgeType,
    pub required_milestones: Option<u32>,
    pub required_reputation: Option<u32>,
    pub required_grants: Option<u32>,
    pub one_time: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadgeRecord {
    pub badge_type: BadgeType,
    pub recipient: Address,
    pub awarded_at: u64,
    pub grant_id: Option<u64>,
    pub milestone_idx: Option<u32>,
}

// ── Refund policies (#603) ────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RefundPolicyType {
    FullRefund = 0,
    ProportionalToRemaining = 1,
    TimeWeighted = 2,
    PenaltyOnCancel = 3,
    NoRefund = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefundPolicy {
    pub grant_id: u64,
    pub policy_type: RefundPolicyType,
    pub penalty_bps: u32,
    pub grace_period_ledgers: u32,
    pub min_refund_pct_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefundCalculation {
    pub gross_escrow: i128,
    pub funder_refund: i128,
    pub contributor_compensation: i128,
    pub penalty_amount: i128,
    pub policy_applied: RefundPolicyType,
}

// ── State snapshots (#603) ────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SnapshotTrigger {
    DisputeRaised = 0,
    AdminRequest = 1,
    MilestoneSubmission = 2,
    PreUpgrade = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateSnapshot {
    pub id: u32,
    pub grant_id: u64,
    pub trigger: SnapshotTrigger,
    pub grant_status: GrantStatus,
    pub escrow_balance: i128,
    pub milestones_paid_out: u32,
    pub total_milestones: u32,
    pub milestone_states: Vec<MilestoneState>,
    pub captured_at: u64,
    pub captured_at_ledger: u32,
    pub captured_by: Address,
}

// ── Issue #545: Merkle evidence commitments ───────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleCommitment {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub root: Bytes,
    pub leaf_count: u32,
    pub committed_by: Address,
    pub committed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleProof {
    pub leaf: Bytes,
    pub leaf_index: u32,
    pub siblings: Vec<Bytes>,
}

// ── Issue #541: Grant templates ───────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GrantArchetype {
    ResearchGrant = 0,
    DevelopmentBounty = 1,
    CommunityProject = 2,
    ProtocolIntegration = 3,
    CustomTemplate = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantTemplate {
    pub archetype: GrantArchetype,
    pub num_milestones: u32,
    pub review_window_ledgers: u32,
    pub min_reviewers: u32,
    pub quorum_threshold_bps: u32,
    pub voting_mechanism: VotingMechanism,
    pub requires_staking: bool,
    pub multisig_required: bool,
    pub sequential_milestones: bool,
    pub insurance_opt_in: bool,
}

// ── Issue #544: Rate limiting ─────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RateLimitAction {
    GrantCreate = 0,
    MilestoneSubmit = 1,
    ContributorRegister = 2,
    DisputeRaise = 3,
    BountyCreate = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateLimitRecord {
    pub address: Address,
    pub action: RateLimitAction,
    pub count: u32,
    pub window_start: u64,
    pub window_duration: u64,
    pub max_per_window: u32,
}

// ── Issue #566: Invoice-Style Milestone Billing ───────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum InvoiceStatus {
    Draft = 0,
    Submitted = 1,
    Approved = 2,
    Rejected = 3,
    Paid = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineItem {
    pub description: soroban_sdk::String,
    pub quantity: u32,
    pub unit_price: i128,
    pub total: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invoice {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub invoice_number: soroban_sdk::String,
    pub contributor: Address,
    pub line_items: soroban_sdk::Vec<LineItem>,
    pub subtotal: i128,
    pub tax_bps: u32,
    pub total: i128,
    pub currency_token: Address,
    pub status: InvoiceStatus,
    pub submitted_at: u64,
    pub approved_at: Option<u64>,
    pub notes: Option<soroban_sdk::String>,
}

// ── Issue #582: Advanced Protocol Analytics ───────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollingWindow {
    pub metric_key: soroban_sdk::Symbol,
    pub window_size: u32,
    pub values: soroban_sdk::Vec<i128>,
    pub timestamps: soroban_sdk::Vec<u64>,
    pub sum: i128,
    pub count: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryStats {
    pub category_id: u32,
    pub total_grants: u32,
    pub completed_grants: u32,
    pub total_funded: i128,
    pub avg_completion_ledgers: u32,
    pub success_rate_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyticsSnapshot {
    pub avg_milestone_completion_ledgers: u32,
    pub avg_reviewer_turnaround_ledgers: u32,
    pub overall_success_rate_bps: u32,
    pub top_category_id: Option<u32>,
    pub tvl_7day_growth_bps: i32,
    pub snapshot_at: u64,
}

// ── Issue #596: Dynamic On-Chain Protocol Parameter Store ─────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ParamType {
    U32 = 0,
    I128 = 1,
    Bool = 2,
    Address = 3,
    StringValue = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamValue {
    pub param_type: ParamType,
    pub u32_val: Option<u32>,
    pub i128_val: Option<i128>,
    pub bool_val: Option<bool>,
    pub address_val: Option<Address>,
    pub string_val: Option<soroban_sdk::String>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamRecord {
    pub key: soroban_sdk::Symbol,
    pub value: ParamValue,
    pub set_by: Address,
    pub set_at: u64,
    pub description: soroban_sdk::String,
    pub requires_dao_vote: bool,
}

// ── Issue #593: Role-Based Access Control (RBAC) Framework ────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Role {
    SuperAdmin = 0,
    ProtocolAdmin = 1,
    TreasuryManager = 2,
    ComplianceOfficer = 3,
    DisputeArbiter = 4,
    OracleOperator = 5,
    ReviewerModerator = 6,
    EmergencyPauser = 7,
    Relayer = 8,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleAssignment {
    pub holder: Address,
    pub role: Role,
    pub granted_by: Address,
    pub granted_at: u64,
    pub expires_at: Option<u64>,
    pub is_active: bool,
}

// ── Issue #579: IP License Tracking ──────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum LicenseType {
    OpenSource = 0,
    Proprietary = 1,
    CreativeCommons = 2,
    Custom = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpRights {
    pub commercial_use: bool,
    pub modification: bool,
    pub distribution: bool,
    pub sublicense: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LicenseRecord {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub spdx_id: String,
    pub license_type: LicenseType,
    pub rights: IpRights,
    pub restrictions: String,
    pub attached_by: Address,
    pub attached_at: u64,
}

// ── Issue #592: Multi-Recipient Payment Splitting ─────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitRecipient {
    pub recipient: Address,
    pub share_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentSplit {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub recipients: Vec<SplitRecipient>,
    pub registered_by: Address,
    pub registered_at: u64,
}

// ── Issue #578: Cross-Protocol Grant Syndication ─────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SyndicateStatus {
    Forming = 0,
    Active = 1,
    Closed = 2,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyndicateMember {
    pub member: Address,
    pub committed_amount: i128,
    pub deposited_amount: i128,
    pub share_bps: u32,
    pub is_lead: bool,
    pub joined_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyndicateGrant {
    pub grant_id: u64,
    pub lead: Address,
    pub target_total: i128,
    pub token: Address,
    pub status: SyndicateStatus,
    pub member_count: u32,
    pub min_commitment: i128,
    pub max_members: u32,
    pub formation_deadline: u64,
}

// ── Issue #591: Grant Specification Versioning ───────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AmendmentStatus {
    Proposed = 0,
    Approved = 1,
    Rejected = 2,
    Withdrawn = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Amendment {
    pub grant_id: u64,
    pub version: u32,
    pub proposed_by: Address,
    pub changed_fields: Vec<String>,
    pub previous_values: Vec<String>,
    pub new_values: Vec<String>,
    pub rationale: String,
    pub status: AmendmentStatus,
    pub reviewer_votes: Map<Address, bool>,
    pub proposed_at: u64,
    pub resolved_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantVersion {
    pub grant_id: u64,
    pub version: u32,
    pub title: String,
    pub description: String,
    pub total_amount: i128,
    pub total_milestones: u32,
    pub created_at: u64,
    pub amendment_id: Option<u32>,
}

// ── Issue #568: Grant Ownership and Role Transfer ─────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TransferableRole {
    Owner = 0,
    Reviewer = 1,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferProposal {
    pub grant_id: u64,
    pub current_holder: Address,
    pub proposed_new_holder: Address,
    pub role: TransferableRole,
    pub reviewer_to_replace: Option<Address>,
    pub proposed_at: u64,
}

// ── Issue #583: Typed Evidence Schemas ───────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum EvidenceFieldType {
    Url = 0,
    Text = 1,
    Number = 2,
    Percentage = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceField {
    pub name: String,
    pub field_type: EvidenceFieldType,
    pub required: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceSchema {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub fields: Vec<EvidenceField>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredEvidence {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub values: Map<String, String>,
    pub submitted_by: Address,
    pub submitted_at: u64,
}

// ── Issue #590: Public Crowdsourced Review Module ────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PublicReviewSignal {
    Positive = 0,
    Neutral = 1,
    Negative = 2,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicReview {
    pub reviewer: Address,
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub signal: PublicReviewSignal,
    pub comment: String,
    pub reviewer_reputation: u32,
    pub submitted_at: u64,
    pub helpful_votes: u32,
}

// ── Issue #595: Milestone Dependency Graph ────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneDependency {
    pub milestone_idx: u32,
    pub depends_on: Vec<u32>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneDag {
    pub grant_id: u64,
    pub dependencies: Vec<MilestoneDependency>,
    pub is_valid: bool,
}

// ── Issue #597: Fork Record ───────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForkRecord {
    pub original_grant_id: u64,
    pub forked_grant_id: u64,
    pub forked_by: Address,
    pub forked_at: u64,
    pub inherited_fields: Vec<soroban_sdk::String>,
    pub overridden_fields: Vec<soroban_sdk::String>,
}

// ── Issue #580: Notification types ────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum NotificationEvent {
    NewGrant = 0,
    MilestoneSubmitted = 1,
    MilestoneApproved = 2,
    MilestoneRejected = 3,
    DisputeRaised = 4,
    GrantCompleted = 5,
    BountyPosted = 6,
    ReviewerRequested = 7,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionScope {
    Global,
    PerGrant(u64),
    PerContributor(Address),
    PerTag(u128),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    pub subscriber: Address,
    pub event: NotificationEvent,
    pub scope: SubscriptionScope,
    pub subscribed_at: u64,
    pub is_active: bool,
}

// ── Issue #575: Decay Config types ────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DecayType {
    None = 0,
    Linear = 1,
    Exponential = 2,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecayConfig {
    pub enabled: bool,
    pub decay_type: DecayType,
    pub half_life_ledgers: u32,
    pub linear_decay_per_day: u32,
    pub decay_floor: u32,
    pub inactivity_threshold_ledgers: u32,
}

// ── Issue #565: Contributor Public Portfolio ──────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantSummary {
    pub grant_id: u64,
    pub title: String,
    pub milestones_completed: u32,
    pub total_milestones: u32,
    pub total_earned: i128,
    pub token: Address,
    pub completed_at: Option<u64>,
    pub status: GrantStatus,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorPortfolio {
    pub contributor: Address,
    pub display_name: String,
    pub bio: String,
    pub reputation_score: u32,
    pub reputation_tier: ReputationTier,
    pub total_earned_usd_equivalent: Option<i128>,
    pub grants_completed: u32,
    pub grants_active: u32,
    pub milestones_approved: u32,
    pub milestones_rejected: u32,
    pub badges: Vec<BadgeType>,
    pub recent_grants: Vec<GrantSummary>,
    pub member_since: u64,
}

// ── Issue #XXX: Quadratic Funding Matching Rounds ──────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchingContribution {
    pub contributor: Address,
    pub grant_id: u64,
    pub amount: i128,
    pub contributed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchingAllocation {
    pub grant_id: u64,
    pub direct_contributions: i128,
    pub match_amount: i128,
    pub unique_contributors: u32,
    pub qf_score: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchingRound {
    pub id: u32,
    pub token: Address,
    pub matching_pool: i128,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub eligible_grant_ids: Vec<u64>,
    pub allocations: Vec<MatchingAllocation>,
    pub finalized: bool,
    pub distributed: bool,
    pub created_by: Address,
}

// ── Issue #XXX: Multi-Grant Portfolio Management ──────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioFilter {
    pub owner: Option<Address>,
    pub status: Option<GrantStatus>,
    pub token: Option<Address>,
    pub category_id: Option<u32>,
    pub min_amount: Option<i128>,
    pub max_amount: Option<i128>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioStats {
    pub owner: Address,
    pub total_grants: u32,
    pub active_grants: u32,
    pub completed_grants: u32,
    pub total_funded: i128,
    pub total_paid_out: i128,
    pub total_in_escrow: i128,
    pub unique_contributors: u32,
    pub unique_reviewers: u32,
    pub avg_completion_rate_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantPortfolio {
    pub owner: Address,
    pub grant_ids: Vec<u64>,
    pub stats: PortfolioStats,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchResult {
    pub successful: u32,
    pub failed: u32,
    pub total: u32,
}

// ── Issue #570: NFT Certificate per Approved Milestone ───────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NftMetadata {
    pub name: String,
    pub description: String,
    pub grant_title: String,
    pub image_uri: String,
    pub attributes: Vec<(String, String)>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneNft {
    pub token_id: u32,
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub owner: Address,
    pub minted_at: u64,
    pub minted_at_ledger: u32,
    pub metadata: NftMetadata,
    pub is_transferable: bool,
    pub proof_hash: Bytes,
}

// ── Crowdfund Module ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CrowdfundStatus {
    Active = 0,
    Succeeded = 1,
    Failed = 2,
    Cancelled = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundCampaign {
    pub id: u64,
    pub owner: Address,
    pub title: String,
    pub description: String,
    pub token: Address,
    pub target_amount: i128,
    pub total_pledged: i128,
    pub deadline: u64,
    pub status: CrowdfundStatus,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundPledge {
    pub campaign_id: u64,
    pub backer: Address,
    pub amount: i128,
    pub pledged_at: u64,
    pub refunded: bool,
}

// ── Issue #569: Referral and Growth Incentive System ─────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralCode {
    pub code_hash: Bytes, // SHA-256 of the plaintext code
    pub referrer: Address,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub max_uses: Option<u32>,
    pub uses: u32,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralRecord {
    pub referred: Address,
    pub referrer: Address,
    pub code_hash: Bytes,
    pub referred_at: u64,
    pub first_action_at: Option<u64>,
    pub reward_paid: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralReward {
    pub referrer: Address,
    pub token: Address,
    pub amount: i128,
    pub earned_at: u64,
    pub for_action: String,
}

// ── Issue #572: Deadline Extension Request Workflow ──────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ExtensionStatus {
    Pending = 0,
    Approved = 1,
    Denied = 2,
    Withdrawn = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionRequest {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub requested_by: Address,
    pub original_deadline: u64,
    pub new_deadline: u64,
    pub reason: String,
    pub status: ExtensionStatus,
    pub votes_approve: u32,
    pub votes_deny: u32,
    pub reviewer_votes: Map<Address, bool>,
    pub requested_at: u64,
    pub resolved_at: Option<u64>,
}

// ── Issue #573: Community Arbitration Pool ───────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arbiter {
    pub address: Address,
    pub stake: i128,
    pub cases_decided: u32,
    pub cases_correct: u32,
    pub is_active: bool,
    pub joined_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbiterVote {
    pub arbiter: Address,
    pub favor_contributor: bool,
    pub confidence: u32, // 1-100, affects reward weight
    pub voted_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbitrationCase {
    pub id: u32,
    pub dispute_id: u32,
    pub panel: Vec<Address>, // 3 or 5 randomly selected arbiters
    pub votes: Vec<ArbiterVote>,
    pub outcome: Option<bool>, // true = contributor wins, false = funder wins
    pub finalized: bool,
    pub assigned_at: u64,
    pub deadline: u64,
}

// ── Issue #574: Surety Bonds for High-Value Grant Delivery ───────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BondStatus {
    Pending = 0,  // awaiting guarantor deposit
    Active = 1,   // bond posted, grant in progress
    Released = 2, // returned to guarantor on completion
    Claimed = 3,  // paid out to funder after default
    Expired = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformanceBond {
    pub id: u32,
    pub grant_id: u64,
    pub principal: Address, // contributor
    pub guarantor: Address, // bond backer
    pub bond_amount: i128,
    pub token: Address,
    pub status: BondStatus,
    pub posted_at: Option<u64>,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BondClaim {
    pub bond_id: u32,
    pub claimed_by: Address,
    pub claim_reason: String,
    pub payout_amount: i128,
    pub claimed_at: u64,
}

// ── Issue #528: Math helpers module (no types needed, pure functions) ───────

// ── Issue #564: Collateral Escrow Module ────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CollateralStatus {
    Required = 0,
    Deposited = 1,
    Released = 2,
    Forfeited = 3,
    PartiallyForfeited = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollateralRequirement {
    pub grant_id: u64,
    pub token: Address,
    pub amount: i128,
    pub forfeit_on_abandon_bps: u32,
    pub forfeit_on_dispute_loss_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollateralDeposit {
    pub grant_id: u64,
    pub contributor: Address,
    pub token: Address,
    pub amount: i128,
    pub status: CollateralStatus,
    pub deposited_at: u64,
    pub forfeited_amount: i128,
}

// ── Issue #598: Funder Report Module ────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunderGrantSummary {
    pub grant_id: u64,
    pub grant_title: soroban_sdk::String,
    pub token: Address,
    pub funded_amount: i128,
    pub paid_out_amount: i128,
    pub refunded_amount: i128,
    pub in_escrow: i128,
    pub yield_earned: Option<i128>,
    pub funded_at: u64,
    pub grant_status: GrantStatus,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunderTokenSummary {
    pub token: Address,
    pub total_committed: i128,
    pub total_paid_out: i128,
    pub total_refunded: i128,
    pub total_in_escrow: i128,
    pub total_yield_earned: i128,
    pub net_deployed: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunderReport {
    pub funder: Address,
    pub report_at: u64,
    pub total_grants_funded: u32,
    pub active_grants: u32,
    pub completed_grants: u32,
    pub token_summaries: soroban_sdk::Vec<FunderTokenSummary>,
    pub grant_summaries: soroban_sdk::Vec<FunderGrantSummary>,
    pub matching_contributions: i128,
    pub insurance_premiums_paid: i128,
}

// ── Issue #512: Whitelist Module ─────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum WhitelistMode {
    Open = 0,
    Restricted = 1,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhitelistScope {
    GlobalReviewer,
    GlobalContributor,
    GrantReviewer(u64),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhitelistEntry {
    pub address: Address,
    pub added_by: Address,
    pub added_at: u64,
    pub scope: WhitelistScope,
}
