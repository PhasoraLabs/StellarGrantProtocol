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
