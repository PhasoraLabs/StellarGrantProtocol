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
    pub require_compliance: Option<ComplianceLevel>,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComplianceStatus {
    Unverified = 0,
    Pending = 1,
    Approved = 2,
    Rejected = 3,
    Expired = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
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

// ── Issue #519: Protocol Treasury Management ─────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreasurySnapshot {
    pub token: Address,
    pub balance: i128,
    pub taken_at: u64,
}

// ── Issue #532: Protocol-Wide DAO Governance ──────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DaoProposalStatus {
    Active = 0,
    Passed = 1,
    Rejected = 2,
    Executed = 3,
    Cancelled = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DaoProposalType {
    /// Replace the protocol's runtime configuration with the encoded `ProtocolConfig`.
    UpdateConfig(ProtocolConfig),
    /// Rotate the global admin address.
    ChangeAdmin(Address),
    /// Move `amount` of `token` out of the treasury to `to`.
    TreasuryWithdrawal(Address, Address, i128),
    /// Generic, free-form proposal (e.g. "add new feature") with no on-chain
    /// execution payload — passing it is a signal to the team/maintainers.
    Generic,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaoProposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub proposal_type: DaoProposalType,
    pub status: DaoProposalStatus,
    pub votes_for: u64,
    pub votes_against: u64,
    pub created_at: u64,
    pub voting_deadline: u64,
    pub executed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaoVote {
    pub voter: Address,
    pub proposal_id: u64,
    pub support: bool,
    pub weight: u64,
    pub cast_at: u64,
}

// ── Issue #533: Competitive Bounty-Mode Grants ────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BountyStatus {
    Open = 0,
    UnderReview = 1,
    Awarded = 2,
    Cancelled = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountyGrant {
    pub id: u64,
    pub owner: Address,
    pub title: String,
    pub description: String,
    pub token: Address,
    pub prize_amount: i128,
    pub status: BountyStatus,
    pub submission_deadline: u64,
    pub winner: Option<Address>,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountySubmission {
    pub bounty_id: u64,
    pub submitter: Address,
    pub proof_url: String,
    pub submitted_at: u64,
}
