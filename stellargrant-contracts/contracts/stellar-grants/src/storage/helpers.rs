use super::keys::{
    ArbitrationKey, AutoApproveKey, BondKey, CollateralKey, ConditionalReleaseKey, CrowdfundKey,
    DataKey, EscrowKey, GrantKey, GrantTimerKey, InsuranceKey, MilestoneKey, UserKey, VotingKey,
};
use crate::types::{
    AcceptanceCriteria, Amendment, AnalyticsSnapshot, AuditEntry, AutoApproveConfig,
    AutoApproveRecord, BreakerState, ChecklistSubmission, ComplianceAttestation, ContractError,
    ContractVersion, ContributorProfile, CrowdfundCampaign, CrowdfundPledge, DexConfig, Dispute,
    EscrowAccount, EscrowState, EvidenceSchema, FunderLedger, Grant, GrantCategory, GrantTag,
    GrantVersion, HookEvent, HookRegistration, InsuranceClaim, InsurancePolicy, Invoice,
    LicenseRecord, MerkleCommitment, MigrationRecord, Milestone, MilestoneDag, MilestoneNft,
    MultisigProposal, OracleConfig, ParamRecord, PauseRecord, PaymentSplit, PaymentStream,
    ProtocolConfig, ProtocolMetrics, ProtocolModule, PublicReview, QuadraticVoteRecord,
    RateLimitAction, RateLimitRecord, RegistryEntry, RelayAllowance, RelayConfig, ReleaseCondition,
    RenewalProposal, ReviewerProfile, ReviewerRequest, Role, RoleAssignment, RollingWindow,
    ScoringRubric, StructuredEvidence, SyndicateGrant, SyndicateMember, TimerRecord, TokenMetric,
    TransferProposal, VoiceCredits, VotingMechanism,
};
use crate::types::{
    Arbiter, ArbiterVote, ArbitrationCase, BondClaim, CollateralDeposit, CollateralRequirement,
    ExtensionRequest, PerformanceBond, ReferralCode, ReferralRecord, WhitelistEntry, WhitelistMode,
    WhitelistScope,
};
use soroban_sdk::{Address, Bytes, Env, Symbol, Vec};

pub(crate) const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
pub(crate) const PERSISTENT_TTL_EXTEND_TO: u32 = 1_000_000;

pub struct Storage;

impl Storage {
    fn bump(env: &Env, key: &DataKey) {
        env.storage().persistent().extend_ttl(
            key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }

    pub fn increment_grant_counter(env: &Env) -> u64 {
        let key = DataKey::Grant(GrantKey::Counter);
        let mut count: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        count += 1;
        env.storage().persistent().set(&key, &count);
        count
    }

    pub fn get_global_admin(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::GlobalAdmin)
    }

    pub fn set_global_admin(env: &Env, admin: &Address) {
        env.storage().persistent().set(&DataKey::GlobalAdmin, admin);
    }

    pub fn get_council(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::Council)
    }

    pub fn set_council(env: &Env, council: &Address) {
        env.storage().persistent().set(&DataKey::Council, council);
    }

    pub fn get_grant(env: &Env, grant_id: u64) -> Option<Grant> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::Data(grant_id)))
    }

    pub fn get_grant_v(env: &Env, grant_id: u64) -> Grant {
        Self::get_grant(env, grant_id).unwrap_or_else(|| {
            env.panic_with_error(ContractError::GrantNotFound);
        })
    }

    pub fn set_grant(env: &Env, grant_id: u64, grant: &Grant) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::Data(grant_id)), grant);
    }

    pub fn has_grant(env: &Env, grant_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Grant(GrantKey::Data(grant_id)))
    }

    pub fn get_milestone(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Milestone> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::Data(
                grant_id,
                milestone_idx,
            )))
    }

    pub fn get_milestone_v(env: &Env, grant_id: u64, milestone_idx: u32) -> Milestone {
        Self::get_milestone(env, grant_id, milestone_idx).unwrap_or_else(|| {
            env.panic_with_error(ContractError::MilestoneNotFound);
        })
    }

    pub fn set_milestone(env: &Env, grant_id: u64, milestone_idx: u32, milestone: &Milestone) {
        env.storage().persistent().set(
            &DataKey::Milestone(MilestoneKey::Data(grant_id, milestone_idx)),
            milestone,
        );
    }

    pub fn get_contributor(env: &Env, contributor: Address) -> Option<ContributorProfile> {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::Profile(contributor)))
    }

    pub fn set_contributor(env: &Env, contributor: Address, profile: &ContributorProfile) {
        env.storage()
            .persistent()
            .set(&DataKey::User(UserKey::Profile(contributor)), profile);
    }

    pub fn get_escrow_state(env: &Env, grant_id: u64) -> EscrowState {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(EscrowKey::State(grant_id)))
            .unwrap_or_else(|| {
                env.panic_with_error(ContractError::InvalidState);
            })
    }

    pub fn set_escrow_state(env: &Env, grant_id: u64, state: &EscrowState) {
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(EscrowKey::State(grant_id)), state);
    }

    pub fn get_multisig_signers(env: &Env, grant_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Voting(VotingKey::MultisigSigners(grant_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_multisig_signers(env: &Env, grant_id: u64, signers: &Vec<Address>) {
        env.storage().persistent().set(
            &DataKey::Voting(VotingKey::MultisigSigners(grant_id)),
            signers,
        );
    }

    pub fn has_release_approval(env: &Env, grant_id: u64, signer: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Voting(VotingKey::ReleaseApproval(
                grant_id,
                signer.clone(),
            )))
            .unwrap_or(false)
    }

    pub fn set_release_approval(env: &Env, grant_id: u64, signer: &Address, approved: bool) {
        env.storage().persistent().set(
            &DataKey::Voting(VotingKey::ReleaseApproval(grant_id, signer.clone())),
            &approved,
        );
    }

    pub fn get_reviewer_reputation(env: &Env, reviewer: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::ReviewerRep(reviewer)))
            .unwrap_or(1)
    }

    pub fn set_reviewer_reputation(env: &Env, reviewer: Address, reputation: u32) {
        env.storage()
            .persistent()
            .set(&DataKey::User(UserKey::ReviewerRep(reviewer)), &reputation);
    }

    pub fn get_reviewer_stake(env: &Env, grant_id: u64, reviewer: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::ReviewerStake(
                grant_id,
                reviewer.clone(),
            )))
            .unwrap_or(0)
    }

    pub fn set_reviewer_stake(env: &Env, grant_id: u64, reviewer: &Address, amount: i128) {
        env.storage().persistent().set(
            &DataKey::User(UserKey::ReviewerStake(grant_id, reviewer.clone())),
            &amount,
        );
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
            .get(&DataKey::User(UserKey::RegistryIndex))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_contributor_index(env: &Env, index: &Vec<RegistryEntry>) {
        env.storage()
            .persistent()
            .set(&DataKey::User(UserKey::RegistryIndex), index);
    }

    pub fn get_reviewer_allowlist(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::ReviewerAllowlist))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_reviewer_allowlist(env: &Env, list: &Vec<Address>) {
        env.storage()
            .persistent()
            .set(&DataKey::User(UserKey::ReviewerAllowlist), list);
    }

    // ── Immutable Audit Trail (#523) ──────────────────────────────────

    const AUDIT_TTL_THRESHOLD: u32 = 100_000;
    const AUDIT_TTL_EXTEND_TO: u32 = 1_000_000;

    pub fn get_audit_log(env: &Env, grant_id: u64) -> Vec<AuditEntry> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::AuditLog(grant_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn append_audit_entry(env: &Env, grant_id: u64, entry: &AuditEntry) {
        let key = DataKey::Grant(GrantKey::AuditLog(grant_id));
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
            .get(&DataKey::Voting(VotingKey::VoiceCredits(
                voter.clone(),
                grant_id,
            )))
    }

    pub fn set_voice_credits(env: &Env, credits: &VoiceCredits) {
        env.storage().persistent().set(
            &DataKey::Voting(VotingKey::VoiceCredits(
                credits.voter.clone(),
                credits.grant_id,
            )),
            credits,
        );
    }

    pub fn get_voting_mechanism(env: &Env, grant_id: u64) -> VotingMechanism {
        env.storage()
            .persistent()
            .get(&DataKey::Voting(VotingKey::Mechanism(grant_id)))
            .unwrap_or(VotingMechanism::SimpleMajority)
    }

    pub fn set_voting_mechanism(env: &Env, grant_id: u64, mechanism: &VotingMechanism) {
        env.storage()
            .persistent()
            .set(&DataKey::Voting(VotingKey::Mechanism(grant_id)), mechanism);
    }

    pub fn get_qv_votes(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<QuadraticVoteRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Voting(VotingKey::QvVotes(
                grant_id,
                milestone_idx,
            )))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_qv_votes(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        votes: &Vec<QuadraticVoteRecord>,
    ) {
        env.storage().persistent().set(
            &DataKey::Voting(VotingKey::QvVotes(grant_id, milestone_idx)),
            votes,
        );
    }

    // ── Insurance Pool (#538) ─────────────────────────────────────────────────

    pub fn get_insurance_pool(env: &Env, token: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Insurance(InsuranceKey::Pool(token.clone())))
            .unwrap_or(0)
    }

    pub fn set_insurance_pool(env: &Env, token: &Address, balance: i128) {
        env.storage().persistent().set(
            &DataKey::Insurance(InsuranceKey::Pool(token.clone())),
            &balance,
        );
    }

    pub fn get_insurance_policy(env: &Env, grant_id: u64) -> Option<InsurancePolicy> {
        env.storage()
            .persistent()
            .get(&DataKey::Insurance(InsuranceKey::Policy(grant_id)))
    }

    pub fn set_insurance_policy(env: &Env, policy: &InsurancePolicy) {
        env.storage().persistent().set(
            &DataKey::Insurance(InsuranceKey::Policy(policy.grant_id)),
            policy,
        );
    }

    pub fn next_claim_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Insurance(InsuranceKey::ClaimCounter))
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Insurance(InsuranceKey::ClaimCounter), &id);
        id
    }

    pub fn get_insurance_claim(env: &Env, claim_id: u32) -> Option<InsuranceClaim> {
        env.storage()
            .persistent()
            .get(&DataKey::Insurance(InsuranceKey::Claim(claim_id)))
    }

    pub fn set_insurance_claim(env: &Env, claim: &InsuranceClaim) {
        env.storage()
            .persistent()
            .set(&DataKey::Insurance(InsuranceKey::Claim(claim.id)), claim);
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
            .has(&DataKey::Milestone(MilestoneKey::ReputationApplied(
                grant_id,
                milestone_idx,
            )))
    }

    pub fn mark_milestone_reputation_applied(env: &Env, grant_id: u64, milestone_idx: u32) {
        let key = DataKey::Milestone(MilestoneKey::ReputationApplied(grant_id, milestone_idx));
        env.storage().persistent().set(&key, &true);
        Self::bump(env, &key);
    }

    // ── Issue #514: arbiter-based dispute record ──────────────────────────────

    pub fn get_dispute(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Dispute> {
        let key = DataKey::Milestone(MilestoneKey::Dispute(grant_id, milestone_idx));
        let d = env.storage().persistent().get(&key);
        if d.is_some() {
            Self::bump(env, &key);
        }
        d
    }

    pub fn set_dispute(env: &Env, grant_id: u64, milestone_idx: u32, dispute: &Dispute) {
        let key = DataKey::Milestone(MilestoneKey::Dispute(grant_id, milestone_idx));
        env.storage().persistent().set(&key, dispute);
        Self::bump(env, &key);
    }

    pub fn remove_dispute(env: &Env, grant_id: u64, milestone_idx: u32) {
        env.storage()
            .persistent()
            .remove(&DataKey::Milestone(MilestoneKey::Dispute(
                grant_id,
                milestone_idx,
            )));
    }

    // ── Issue #516: ProtocolConfig ────────────────────────────────────────────

    pub fn get_protocol_config(env: &Env) -> Option<ProtocolConfig> {
        let key = DataKey::Config;
        let cfg = env.storage().persistent().get(&key);
        if cfg.is_some() {
            Self::bump(env, &key);
        }
        cfg
    }

    pub fn set_protocol_config(env: &Env, cfg: &ProtocolConfig) {
        let key = DataKey::Config;
        env.storage().persistent().set(&key, cfg);
        Self::bump(env, &key);
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
        Self::bump(env, &key);
    }

    // ── Issue #524: Price oracle configuration ────────────────────────────────

    pub fn get_oracle_config(env: &Env) -> Option<OracleConfig> {
        let key = DataKey::OracleConfig;
        let config = env.storage().persistent().get(&key);
        if config.is_some() {
            Self::bump(env, &key);
        }
        config
    }

    pub fn set_oracle_config(env: &Env, config: &OracleConfig) {
        let key = DataKey::OracleConfig;
        env.storage().persistent().set(&key, config);
        Self::bump(env, &key);
    }

    // ── Issue #529: Escrow Module ─────────────────────────────────────────────

    pub fn get_escrow_account(env: &Env, grant_id: u64) -> Option<EscrowAccount> {
        let key = DataKey::Escrow(EscrowKey::Account(grant_id));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_escrow_account(env: &Env, grant_id: u64, account: &EscrowAccount) {
        let key = DataKey::Escrow(EscrowKey::Account(grant_id));
        env.storage().persistent().set(&key, account);
        Self::bump(env, &key);
    }

    pub fn get_funder_ledger(env: &Env, grant_id: u64, funder: &Address) -> Option<FunderLedger> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(EscrowKey::FunderContrib(
                grant_id,
                funder.clone(),
            )))
    }

    pub fn set_funder_ledger(env: &Env, grant_id: u64, funder: &Address, ledger: &FunderLedger) {
        let key = DataKey::Escrow(EscrowKey::FunderContrib(grant_id, funder.clone()));
        env.storage().persistent().set(&key, ledger);
        Self::bump(env, &key);
    }

    pub fn get_escrow_funders_list(env: &Env, grant_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(EscrowKey::FundersList(grant_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_escrow_funders_list(env: &Env, grant_id: u64, list: &Vec<Address>) {
        let key = DataKey::Escrow(EscrowKey::FundersList(grant_id));
        env.storage().persistent().set(&key, list);
        Self::bump(env, &key);
    }

    // ── Issue #530: Multisig Module ───────────────────────────────────────────

    pub fn next_multisig_proposal_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Voting(VotingKey::ProposalCounter))
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Voting(VotingKey::ProposalCounter), &id);
        id
    }

    pub fn get_multisig_proposal(env: &Env, proposal_id: u32) -> Option<MultisigProposal> {
        let key = DataKey::Voting(VotingKey::Proposal(proposal_id));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_multisig_proposal(env: &Env, proposal: &MultisigProposal) {
        let key = DataKey::Voting(VotingKey::Proposal(proposal.id));
        env.storage().persistent().set(&key, proposal);
        Self::bump(env, &key);
    }

    // ── Issue #540: Protocol Metrics ──────────────────────────────────────────

    pub fn get_protocol_metrics(env: &Env) -> Option<ProtocolMetrics> {
        env.storage().persistent().get(&DataKey::Metrics)
    }

    pub fn set_protocol_metrics(env: &Env, metrics: &ProtocolMetrics) {
        env.storage().persistent().set(&DataKey::Metrics, metrics);
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
        Self::bump(env, &key);
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
        env.storage()
            .persistent()
            .set(&DataKey::RelayConfig, config);
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
            .get(&DataKey::User(UserKey::ReviewerProfile(reviewer.clone())))
    }

    pub fn set_reviewer_profile(env: &Env, profile: &ReviewerProfile) {
        env.storage().persistent().set(
            &DataKey::User(UserKey::ReviewerProfile(profile.reviewer.clone())),
            profile,
        );
    }

    pub fn get_reviewer_request(
        env: &Env,
        grant_id: u64,
        reviewer: &Address,
    ) -> Option<ReviewerRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::ReviewerRequest(
                grant_id,
                reviewer.clone(),
            )))
    }

    pub fn set_reviewer_request(env: &Env, request: &ReviewerRequest) {
        env.storage().persistent().set(
            &DataKey::User(UserKey::ReviewerRequest(
                request.grant_id,
                request.reviewer.clone(),
            )),
            request,
        );
    }

    // ── Issue #571: Grant Tags Module ─────────────────────────────────────────

    pub fn get_grant_tags(env: &Env, grant_id: u64) -> Option<GrantTag> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::Tags(grant_id)))
    }

    pub fn set_grant_tags(env: &Env, tags: &GrantTag) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::Tags(tags.grant_id)), tags);
    }

    pub fn get_tag_index(env: &Env, tag_hash: u32) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::TagIndex(tag_hash)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_tag_index(env: &Env, tag_hash: u32, grant_ids: &Vec<u64>) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::TagIndex(tag_hash)), grant_ids);
    }

    pub fn get_category_list(env: &Env) -> Vec<GrantCategory> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::CategoryList))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_category_list(env: &Env, categories: &Vec<GrantCategory>) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::CategoryList), categories);
    }

    // ── Issue #577: Grant Renewal Module ──────────────────────────────────────

    pub fn get_renewal_proposal(env: &Env, original_grant_id: u64) -> Option<RenewalProposal> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::Renewal(original_grant_id)))
    }

    pub fn set_renewal_proposal(env: &Env, proposal: &RenewalProposal) {
        env.storage().persistent().set(
            &DataKey::Grant(GrantKey::Renewal(proposal.original_grant_id)),
            proposal,
        );
    }

    pub fn get_renewal_history(env: &Env, grant_id: u64) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::RenewalHistory(grant_id)))
    }

    pub fn set_renewal_history(env: &Env, grant_id: u64, original_grant_id: u64) {
        env.storage().persistent().set(
            &DataKey::Grant(GrantKey::RenewalHistory(grant_id)),
            &original_grant_id,
        );
    }

    // ── Issue #576: Token Swap ────────────────────────────────────────────────

    pub fn get_dex_config(env: &Env) -> Option<DexConfig> {
        env.storage().persistent().get(&DataKey::DexConfig)
    }

    pub fn set_dex_config(env: &Env, config: &DexConfig) {
        env.storage().persistent().set(&DataKey::DexConfig, config);
    }

    // ── Issue #581: Milestone Checklist ──────────────────────────────────────

    pub fn get_milestone_checklist(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<Vec<AcceptanceCriteria>> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::Checklist(
                grant_id,
                milestone_idx,
            )))
    }

    pub fn set_milestone_checklist(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        criteria: &Vec<AcceptanceCriteria>,
    ) {
        let key = DataKey::Milestone(MilestoneKey::Checklist(grant_id, milestone_idx));
        env.storage().persistent().set(&key, criteria);
        Self::bump(env, &key);
    }

    pub fn get_checklist_submission(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<ChecklistSubmission> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::Submission(
                grant_id,
                milestone_idx,
            )))
    }

    pub fn set_checklist_submission(env: &Env, submission: &ChecklistSubmission) {
        let key = DataKey::Milestone(MilestoneKey::Submission(
            submission.grant_id,
            submission.milestone_idx,
        ));
        env.storage().persistent().set(&key, submission);
        Self::bump(env, &key);
    }

    // ── Issue #589: Scoring ───────────────────────────────────────────────────

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

    // ── Issue #594: Circuit Breaker ───────────────────────────────────────────

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

    // ── Issue #545: Merkle commitments ───────────────────────────────────────

    pub fn get_merkle_commitment(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<MerkleCommitment> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::MerkleCommit(
                grant_id,
                milestone_idx,
            )))
    }

    pub fn set_merkle_commitment(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        commitment: &MerkleCommitment,
    ) {
        let key = DataKey::Milestone(MilestoneKey::MerkleCommit(grant_id, milestone_idx));
        env.storage().persistent().set(&key, commitment);
        Self::bump(env, &key);
    }

    // ── Issue #544: Rate limits ───────────────────────────────────────────────

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
        Self::bump(env, &key);
    }

    // ── Issue #566: Invoice Billing ───────────────────────────────────────────

    pub fn get_invoice(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Invoice> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::Invoice(
                grant_id,
                milestone_idx,
            )))
    }

    pub fn set_invoice(env: &Env, grant_id: u64, milestone_idx: u32, invoice: &Invoice) {
        let key = DataKey::Milestone(MilestoneKey::Invoice(grant_id, milestone_idx));
        env.storage().persistent().set(&key, invoice);
        Self::bump(env, &key);
    }

    // ── Issue #582: Analytics ─────────────────────────────────────────────────

    pub fn get_rolling_window(env: &Env, metric: &Symbol) -> Option<RollingWindow> {
        env.storage()
            .persistent()
            .get(&DataKey::RollingWindow(metric.clone()))
    }

    pub fn set_rolling_window(env: &Env, metric: &Symbol, window: &RollingWindow) {
        let key = DataKey::RollingWindow(metric.clone());
        env.storage().persistent().set(&key, window);
        Self::bump(env, &key);
    }

    pub fn get_analytics_snapshot(env: &Env) -> Option<AnalyticsSnapshot> {
        env.storage().persistent().get(&DataKey::AnalyticsSnapshot)
    }

    pub fn set_analytics_snapshot(env: &Env, snapshot: &AnalyticsSnapshot) {
        env.storage()
            .persistent()
            .set(&DataKey::AnalyticsSnapshot, snapshot);
    }

    // ── Issue #596: Dynamic Params ────────────────────────────────────────────

    pub fn get_param(env: &Env, key: &Symbol) -> Option<ParamRecord> {
        env.storage().persistent().get(&DataKey::Param(key.clone()))
    }

    pub fn set_param(env: &Env, key: &Symbol, record: &ParamRecord) {
        let data_key = DataKey::Param(key.clone());
        env.storage().persistent().set(&data_key, record);
        Self::bump(env, &data_key);

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
        Self::bump(env, &data_key);
    }

    pub fn list_param_keys(env: &Env) -> Vec<Symbol> {
        env.storage()
            .persistent()
            .get(&DataKey::ParamKeys)
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── Issue #593: RBAC ──────────────────────────────────────────────────────

    pub fn get_role_assignment(
        env: &Env,
        address: &Address,
        role: &Role,
    ) -> Option<RoleAssignment> {
        env.storage()
            .persistent()
            .get(&DataKey::RoleAssignment(address.clone(), role.clone()))
    }

    pub fn set_role_assignment(
        env: &Env,
        address: &Address,
        role: &Role,
        assignment: &RoleAssignment,
    ) {
        let key = DataKey::RoleAssignment(address.clone(), role.clone());
        env.storage().persistent().set(&key, assignment);
        Self::bump(env, &key);
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
        Self::bump(env, &key);
    }

    // ── Crowdfund Module ──────────────────────────────────────────────────────

    pub fn next_crowdfund_id(env: &Env) -> u64 {
        let mut id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Crowdfund(CrowdfundKey::Counter))
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Crowdfund(CrowdfundKey::Counter), &id);
        id
    }

    pub fn get_crowdfund_campaign(env: &Env, id: u64) -> Option<CrowdfundCampaign> {
        let key = DataKey::Crowdfund(CrowdfundKey::Campaign(id));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_crowdfund_campaign(env: &Env, campaign: &CrowdfundCampaign) {
        let key = DataKey::Crowdfund(CrowdfundKey::Campaign(campaign.id));
        env.storage().persistent().set(&key, campaign);
        Self::bump(env, &key);
    }

    pub fn get_crowdfund_pledge(
        env: &Env,
        campaign_id: u64,
        backer: &Address,
    ) -> Option<CrowdfundPledge> {
        let key = DataKey::Crowdfund(CrowdfundKey::Pledge(campaign_id, backer.clone()));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_crowdfund_pledge(env: &Env, pledge: &CrowdfundPledge) {
        let key = DataKey::Crowdfund(CrowdfundKey::Pledge(
            pledge.campaign_id,
            pledge.backer.clone(),
        ));
        env.storage().persistent().set(&key, pledge);
        Self::bump(env, &key);
    }

    pub fn get_crowdfund_backers(env: &Env, campaign_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Crowdfund(CrowdfundKey::Backers(campaign_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_crowdfund_backers(env: &Env, campaign_id: u64, backers: &Vec<Address>) {
        let key = DataKey::Crowdfund(CrowdfundKey::Backers(campaign_id));
        env.storage().persistent().set(&key, backers);
        Self::bump(env, &key);
    }

    // ── Issue #579: IP License Tracking ──────────────────────────────────────

    pub fn get_license_record(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<LicenseRecord> {
        let key = DataKey::Milestone(MilestoneKey::License(grant_id, milestone_idx));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_license_record(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        record: &LicenseRecord,
    ) {
        let key = DataKey::Milestone(MilestoneKey::License(grant_id, milestone_idx));
        env.storage().persistent().set(&key, record);
        Self::bump(env, &key);
    }

    // ── Issue #592: Multi-Recipient Payment Splitting ─────────────────────────

    pub fn get_payment_split(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<PaymentSplit> {
        let key = DataKey::Milestone(MilestoneKey::PaymentSplit(grant_id, milestone_idx));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_payment_split(env: &Env, grant_id: u64, milestone_idx: u32, split: &PaymentSplit) {
        let key = DataKey::Milestone(MilestoneKey::PaymentSplit(grant_id, milestone_idx));
        env.storage().persistent().set(&key, split);
        Self::bump(env, &key);
    }

    // ── Issue #578: Cross-Protocol Grant Syndication ─────────────────────────

    pub fn get_syndicate_grant(env: &Env, grant_id: u64) -> Option<SyndicateGrant> {
        let key = DataKey::Grant(GrantKey::Syndicate(grant_id));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_syndicate_grant(env: &Env, grant_id: u64, syndicate: &SyndicateGrant) {
        let key = DataKey::Grant(GrantKey::Syndicate(grant_id));
        env.storage().persistent().set(&key, syndicate);
        Self::bump(env, &key);
    }

    pub fn get_syndicate_member(
        env: &Env,
        grant_id: u64,
        member: &Address,
    ) -> Option<SyndicateMember> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::SyndicateMember(
                grant_id,
                member.clone(),
            )))
    }

    pub fn set_syndicate_member(
        env: &Env,
        grant_id: u64,
        member: &Address,
        record: &SyndicateMember,
    ) {
        let key = DataKey::Grant(GrantKey::SyndicateMember(grant_id, member.clone()));
        env.storage().persistent().set(&key, record);
        Self::bump(env, &key);
    }

    pub fn remove_syndicate_member(env: &Env, grant_id: u64, member: &Address) {
        env.storage()
            .persistent()
            .remove(&DataKey::Grant(GrantKey::SyndicateMember(
                grant_id,
                member.clone(),
            )));
    }

    pub fn get_syndicate_member_index(env: &Env, grant_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::SyndicateMembers(grant_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_syndicate_member_index(env: &Env, grant_id: u64, members: &Vec<Address>) {
        let key = DataKey::Grant(GrantKey::SyndicateMembers(grant_id));
        env.storage().persistent().set(&key, members);
        Self::bump(env, &key);
    }

    pub fn set_syndicate_payouts(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        payouts: &Vec<(Address, i128)>,
    ) {
        let key = DataKey::Grant(GrantKey::SyndicatePayouts(grant_id, milestone_idx));
        env.storage().persistent().set(&key, payouts);
        Self::bump(env, &key);
    }

    // ── Issue #591: Grant Specification Versioning ───────────────────────────

    pub fn get_grant_version(env: &Env, grant_id: u64, version: u32) -> Option<GrantVersion> {
        let key = DataKey::Grant(GrantKey::SpecVersion(grant_id, version));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_grant_version(env: &Env, grant_id: u64, version: u32, snapshot: &GrantVersion) {
        let key = DataKey::Grant(GrantKey::SpecVersion(grant_id, version));
        env.storage().persistent().set(&key, snapshot);
        Self::bump(env, &key);
    }

    pub fn get_current_version(env: &Env, grant_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::CurrentVersion(grant_id)))
            .unwrap_or(0)
    }

    pub fn set_current_version(env: &Env, grant_id: u64, version: u32) {
        let key = DataKey::Grant(GrantKey::CurrentVersion(grant_id));
        env.storage().persistent().set(&key, &version);
        Self::bump(env, &key);
    }

    pub fn get_amendment(env: &Env, grant_id: u64, version: u32) -> Option<Amendment> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::Amendment(grant_id, version)))
    }

    pub fn set_amendment(env: &Env, grant_id: u64, version: u32, amendment: &Amendment) {
        let key = DataKey::Grant(GrantKey::Amendment(grant_id, version));
        env.storage().persistent().set(&key, amendment);
        Self::bump(env, &key);
    }

    pub fn get_amendment_history(env: &Env, grant_id: u64) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::AmendmentHistory(grant_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_amendment_history(env: &Env, grant_id: u64, history: &Vec<u32>) {
        let key = DataKey::Grant(GrantKey::AmendmentHistory(grant_id));
        env.storage().persistent().set(&key, history);
        Self::bump(env, &key);
    }

    // ── Issue #568: Grant Transfer ────────────────────────────────────────────

    pub fn get_transfer_proposal(env: &Env, grant_id: u64) -> Option<TransferProposal> {
        let key = DataKey::Grant(GrantKey::Transfer(grant_id));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_transfer_proposal(env: &Env, grant_id: u64, proposal: &TransferProposal) {
        let key = DataKey::Grant(GrantKey::Transfer(grant_id));
        env.storage().persistent().set(&key, proposal);
        Self::bump(env, &key);
    }

    pub fn remove_transfer_proposal(env: &Env, grant_id: u64) {
        env.storage()
            .persistent()
            .remove(&DataKey::Grant(GrantKey::Transfer(grant_id)));
    }

    // ── Issue #583: Typed Evidence Schemas ───────────────────────────────────

    pub fn get_evidence_schema(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<EvidenceSchema> {
        let key = DataKey::Milestone(MilestoneKey::EvidenceSchema(grant_id, milestone_idx));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_evidence_schema(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        schema: &EvidenceSchema,
    ) {
        let key = DataKey::Milestone(MilestoneKey::EvidenceSchema(grant_id, milestone_idx));
        env.storage().persistent().set(&key, schema);
        Self::bump(env, &key);
    }

    pub fn get_structured_evidence(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<StructuredEvidence> {
        let key = DataKey::Milestone(MilestoneKey::StructuredEvidence(grant_id, milestone_idx));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_structured_evidence(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        evidence: &StructuredEvidence,
    ) {
        let key = DataKey::Milestone(MilestoneKey::StructuredEvidence(grant_id, milestone_idx));
        env.storage().persistent().set(&key, evidence);
        Self::bump(env, &key);
    }

    // ── Issue #590: Public Review ─────────────────────────────────────────────

    pub fn get_public_reviews(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<PublicReview> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::PublicReviews(
                grant_id,
                milestone_idx,
            )))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_public_reviews(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        reviews: &Vec<PublicReview>,
    ) {
        let key = DataKey::Milestone(MilestoneKey::PublicReviews(grant_id, milestone_idx));
        env.storage().persistent().set(&key, reviews);
        Self::bump(env, &key);
    }

    pub fn get_public_reviewer_record(
        env: &Env,
        reviewer: &Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<PublicReview> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::ReviewerRecord(
                reviewer.clone(),
                grant_id,
                milestone_idx,
            )))
    }

    pub fn set_public_reviewer_record(
        env: &Env,
        reviewer: &Address,
        grant_id: u64,
        milestone_idx: u32,
        review: &PublicReview,
    ) {
        let key = DataKey::Milestone(MilestoneKey::ReviewerRecord(
            reviewer.clone(),
            grant_id,
            milestone_idx,
        ));
        env.storage().persistent().set(&key, review);
        Self::bump(env, &key);
    }

    // ── Issue #595: Milestone DAG ─────────────────────────────────────────────

    pub fn get_milestone_dag(env: &Env, grant_id: u64) -> Option<MilestoneDag> {
        let key = DataKey::Milestone(MilestoneKey::Dag(grant_id));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_milestone_dag(env: &Env, grant_id: u64, dag: &MilestoneDag) {
        let key = DataKey::Milestone(MilestoneKey::Dag(grant_id));
        env.storage().persistent().set(&key, dag);
        Self::bump(env, &key);
    }

    // ── Issue #570: Milestone NFT ─────────────────────────────────────────────

    pub fn next_nft_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::NftCounter))
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Milestone(MilestoneKey::NftCounter), &id);
        id
    }

    pub fn get_milestone_nft(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<MilestoneNft> {
        let key = DataKey::Milestone(MilestoneKey::Nft(grant_id, milestone_idx));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_milestone_nft(env: &Env, nft: &MilestoneNft) {
        let key = DataKey::Milestone(MilestoneKey::Nft(nft.grant_id, nft.milestone_idx));
        env.storage().persistent().set(&key, nft);
        Self::bump(env, &key);
    }

    pub fn get_nft_by_token_id(env: &Env, token_id: u32) -> Option<MilestoneNft> {
        let index: Option<(u64, u32)> = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::NftTokenIndex(token_id)));
        index.and_then(|(grant_id, milestone_idx)| {
            Self::get_milestone_nft(env, grant_id, milestone_idx)
        })
    }

    pub fn set_nft_token_index(env: &Env, token_id: u32, grant_id: u64, milestone_idx: u32) {
        let key = DataKey::Milestone(MilestoneKey::NftTokenIndex(token_id));
        env.storage()
            .persistent()
            .set(&key, &(grant_id, milestone_idx));
        Self::bump(env, &key);
    }

    pub fn get_nfts_by_owner(env: &Env, owner: &Address) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::NftsByOwner(
                owner.clone(),
            )))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_nfts_by_owner(env: &Env, owner: &Address, token_ids: &Vec<u32>) {
        let key = DataKey::Milestone(MilestoneKey::NftsByOwner(owner.clone()));
        env.storage().persistent().set(&key, token_ids);
        Self::bump(env, &key);
    }

    // ── Issue #565: Contributor Portfolio Grant Index ────────────────────────

    pub fn get_contributor_grant_ids(env: &Env, contributor: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::GrantIds(contributor.clone())))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn push_contributor_grant_id(env: &Env, contributor: &Address, grant_id: u64) {
        let key = DataKey::User(UserKey::GrantIds(contributor.clone()));
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        if !ids.contains(grant_id) {
            ids.push_back(grant_id);
            env.storage().persistent().set(&key, &ids);
            Self::bump(env, &key);
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
        Self::bump(env, &key);
    }

    pub fn get_referral_record(env: &Env, referred: &Address) -> Option<ReferralRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::ReferralRecord(referred.clone()))
    }

    pub fn set_referral_record(env: &Env, record: &ReferralRecord) {
        let key = DataKey::ReferralRecord(record.referred.clone());
        env.storage().persistent().set(&key, record);
        Self::bump(env, &key);
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
        Self::bump(env, &key);
    }

    // ── Issue #572: Deadline Extension Requests ──────────────────────────────

    pub fn get_extension_request(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<ExtensionRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::Extension(
                grant_id,
                milestone_idx,
            )))
    }

    pub fn set_extension_request(env: &Env, req: &ExtensionRequest) {
        let key = DataKey::Milestone(MilestoneKey::Extension(req.grant_id, req.milestone_idx));
        env.storage().persistent().set(&key, req);
        Self::bump(env, &key);
    }

    pub fn remove_extension_request(env: &Env, grant_id: u64, milestone_idx: u32) {
        env.storage()
            .persistent()
            .remove(&DataKey::Milestone(MilestoneKey::Extension(
                grant_id,
                milestone_idx,
            )));
    }

    pub fn get_extension_history(env: &Env, grant_id: u64) -> Vec<ExtensionRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(MilestoneKey::ExtensionHistory(
                grant_id,
            )))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn push_extension_history(env: &Env, req: &ExtensionRequest) {
        let key = DataKey::Milestone(MilestoneKey::ExtensionHistory(req.grant_id));
        let mut history = Self::get_extension_history(env, req.grant_id);
        history.push_back(req.clone());
        env.storage().persistent().set(&key, &history);
        Self::bump(env, &key);
    }

    // ── Issue #573: Community Arbitration Pool ───────────────────────────────

    pub fn get_arbiter_pool(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::Pool))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_arbiter_pool(env: &Env, pool: &Vec<Address>) {
        let key = DataKey::Arbitration(ArbitrationKey::Pool);
        env.storage().persistent().set(&key, pool);
        Self::bump(env, &key);
    }

    pub fn get_arbiter(env: &Env, address: &Address) -> Option<Arbiter> {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::Arbiter(
                address.clone(),
            )))
    }

    pub fn set_arbiter(env: &Env, arbiter: &Arbiter) {
        let key = DataKey::Arbitration(ArbitrationKey::Arbiter(arbiter.address.clone()));
        env.storage().persistent().set(&key, arbiter);
        Self::bump(env, &key);
    }

    pub fn get_arbiter_pool_token(env: &Env) -> Option<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::PoolToken))
    }

    pub fn set_arbiter_pool_token(env: &Env, token: &Address) {
        env.storage()
            .persistent()
            .set(&DataKey::Arbitration(ArbitrationKey::PoolToken), token);
    }

    pub fn get_arbiter_active_cases(env: &Env, arbiter: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::ActiveCases(
                arbiter.clone(),
            )))
            .unwrap_or(0)
    }

    pub fn set_arbiter_active_cases(env: &Env, arbiter: &Address, count: u32) {
        let key = DataKey::Arbitration(ArbitrationKey::ActiveCases(arbiter.clone()));
        env.storage().persistent().set(&key, &count);
        Self::bump(env, &key);
    }

    pub fn next_arbitration_case_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::CaseCounter))
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Arbitration(ArbitrationKey::CaseCounter), &id);
        id
    }

    pub fn get_arbitration_case(env: &Env, case_id: u32) -> Option<ArbitrationCase> {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::Case(case_id)))
    }

    pub fn set_arbitration_case(env: &Env, case: &ArbitrationCase) {
        let key = DataKey::Arbitration(ArbitrationKey::Case(case.id));
        env.storage().persistent().set(&key, case);
        Self::bump(env, &key);
    }

    pub fn get_case_id_by_dispute(env: &Env, dispute_id: u32) -> Option<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::CaseByDispute(
                dispute_id,
            )))
    }

    pub fn set_case_id_by_dispute(env: &Env, dispute_id: u32, case_id: u32) {
        let key = DataKey::Arbitration(ArbitrationKey::CaseByDispute(dispute_id));
        env.storage().persistent().set(&key, &case_id);
        Self::bump(env, &key);
    }

    pub fn is_arbitration_settled(env: &Env, case_id: u32) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::Settled(case_id)))
            .unwrap_or(false)
    }

    pub fn set_arbitration_settled(env: &Env, case_id: u32) {
        env.storage().persistent().set(
            &DataKey::Arbitration(ArbitrationKey::Settled(case_id)),
            &true,
        );
    }

    pub fn get_arbiter_vote(env: &Env, case_id: u32, arbiter: &Address) -> Option<ArbiterVote> {
        env.storage()
            .persistent()
            .get(&DataKey::Arbitration(ArbitrationKey::Vote(
                case_id,
                arbiter.clone(),
            )))
    }

    pub fn set_arbiter_vote(env: &Env, case_id: u32, vote: &ArbiterVote) {
        let key = DataKey::Arbitration(ArbitrationKey::Vote(case_id, vote.arbiter.clone()));
        env.storage().persistent().set(&key, vote);
        Self::bump(env, &key);
    }

    // ── Issue #574: Performance Bonds ────────────────────────────────────────

    pub fn get_performance_bond(env: &Env, grant_id: u64) -> Option<PerformanceBond> {
        env.storage()
            .persistent()
            .get(&DataKey::Bond(BondKey::Bond(grant_id)))
    }

    pub fn set_performance_bond(env: &Env, bond: &PerformanceBond) {
        let key = DataKey::Bond(BondKey::Bond(bond.grant_id));
        env.storage().persistent().set(&key, bond);
        Self::bump(env, &key);
    }

    pub fn next_bond_id(env: &Env) -> u32 {
        let mut id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Bond(BondKey::Counter))
            .unwrap_or(0);
        id += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Bond(BondKey::Counter), &id);
        id
    }

    pub fn get_bond_grant(env: &Env, bond_id: u32) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::Bond(BondKey::BondGrant(bond_id)))
    }

    pub fn set_bond_grant(env: &Env, bond_id: u32, grant_id: u64) {
        let key = DataKey::Bond(BondKey::BondGrant(bond_id));
        env.storage().persistent().set(&key, &grant_id);
        Self::bump(env, &key);
    }

    pub fn get_bond_claim(env: &Env, bond_id: u32) -> Option<BondClaim> {
        env.storage()
            .persistent()
            .get(&DataKey::Bond(BondKey::BondClaim(bond_id)))
    }

    pub fn set_bond_claim(env: &Env, claim: &BondClaim) {
        let key = DataKey::Bond(BondKey::BondClaim(claim.bond_id));
        env.storage().persistent().set(&key, claim);
        Self::bump(env, &key);
    }

    // ── Issue #564: Collateral Escrow ─────────────────────────────────────────

    pub fn get_collateral_requirement(env: &Env, grant_id: u64) -> Option<CollateralRequirement> {
        let key = DataKey::Collateral(CollateralKey::Requirement(grant_id));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_collateral_requirement(env: &Env, grant_id: u64, req: &CollateralRequirement) {
        let key = DataKey::Collateral(CollateralKey::Requirement(grant_id));
        env.storage().persistent().set(&key, req);
        Self::bump(env, &key);
    }

    pub fn get_collateral_deposit(
        env: &Env,
        grant_id: u64,
        contributor: &Address,
    ) -> Option<CollateralDeposit> {
        let key = DataKey::Collateral(CollateralKey::Deposit(grant_id, contributor.clone()));
        let v = env.storage().persistent().get(&key);
        if v.is_some() {
            Self::bump(env, &key);
        }
        v
    }

    pub fn set_collateral_deposit(
        env: &Env,
        grant_id: u64,
        contributor: &Address,
        deposit: &CollateralDeposit,
    ) {
        let key = DataKey::Collateral(CollateralKey::Deposit(grant_id, contributor.clone()));
        env.storage().persistent().set(&key, deposit);
        Self::bump(env, &key);
    }

    // ── Issue #512: Whitelist ─────────────────────────────────────────────────

    pub fn get_whitelist_entries(env: &Env, scope: &WhitelistScope) -> Vec<WhitelistEntry> {
        env.storage()
            .persistent()
            .get(&DataKey::WhitelistEntries(scope.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_whitelist_entries(env: &Env, scope: &WhitelistScope, entries: &Vec<WhitelistEntry>) {
        let key = DataKey::WhitelistEntries(scope.clone());
        env.storage().persistent().set(&key, entries);
        Self::bump(env, &key);
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
        Self::bump(env, &key);
    }

    // ── Issue #598: Funder Report ─────────────────────────────────────────────

    pub fn get_funder_grant_index(env: &Env, funder: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::FunderGrants(funder.clone())))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn push_funder_grant_index(env: &Env, funder: &Address, grant_id: u64) {
        let key = DataKey::User(UserKey::FunderGrants(funder.clone()));
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        if !ids.contains(grant_id) {
            ids.push_back(grant_id);
            env.storage().persistent().set(&key, &ids);
            Self::bump(env, &key);
        }
    }

    pub fn get_matching_contribution(env: &Env, funder: &Address) -> Option<i128> {
        env.storage()
            .persistent()
            .get(&DataKey::User(UserKey::MatchingContrib(funder.clone())))
    }

    pub fn get_grant_counter(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::Counter))
            .unwrap_or(0)
    }

    // ── Issue #587: Fork Record ───────────────────────────────────────────────

    pub fn get_fork_record(env: &Env, grant_id: u64) -> Option<super::super::types::ForkRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::Fork(grant_id)))
    }

    pub fn set_fork_record(env: &Env, grant_id: u64, record: &super::super::types::ForkRecord) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::Fork(grant_id)), record);
    }

    pub fn get_fork_children(env: &Env, grant_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::Grant(GrantKey::ForkChildren(grant_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_fork_children(env: &Env, grant_id: u64, children: &Vec<u64>) {
        env.storage()
            .persistent()
            .set(&DataKey::Grant(GrantKey::ForkChildren(grant_id)), children);
    }

    // ── Issue #613: Conditional Release Conditions ────────────────────────

    pub fn get_release_conditions(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Vec<ReleaseCondition> {
        env.storage()
            .persistent()
            .get(&DataKey::ConditionalRelease(
                ConditionalReleaseKey::Conditions(grant_id, milestone_idx),
            ))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_release_conditions(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        conditions: &Vec<ReleaseCondition>,
    ) {
        let key = DataKey::ConditionalRelease(ConditionalReleaseKey::Conditions(
            grant_id,
            milestone_idx,
        ));
        env.storage().persistent().set(&key, conditions);
        Self::bump(env, &key);
    }

    // ── Issue #612: Auto-Approve Config ──────────────────────────────────

    pub fn get_auto_approve_config(env: &Env, grant_id: u64) -> Option<AutoApproveConfig> {
        let key = DataKey::AutoApprove(AutoApproveKey::Config(grant_id));
        env.storage().persistent().get(&key)
    }

    pub fn set_auto_approve_config(env: &Env, grant_id: u64, config: &AutoApproveConfig) {
        let key = DataKey::AutoApprove(AutoApproveKey::Config(grant_id));
        env.storage().persistent().set(&key, config);
        Self::bump(env, &key);
    }

    pub fn get_auto_approve_record(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<AutoApproveRecord> {
        env.storage().persistent().get(&DataKey::AutoApprove(
            AutoApproveKey::Record(grant_id, milestone_idx),
        ))
    }

    pub fn set_auto_approve_record(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        record: &AutoApproveRecord,
    ) {
        let key = DataKey::AutoApprove(AutoApproveKey::Record(grant_id, milestone_idx));
        env.storage().persistent().set(&key, record);
        Self::bump(env, &key);
    }

    // ── Issue #618: Grant Timers ─────────────────────────────────────────

    pub fn get_grant_timers(env: &Env, grant_id: u64) -> Vec<TimerRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::GrantTimer(GrantTimerKey::Timers(grant_id)))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn set_grant_timers(env: &Env, grant_id: u64, timers: &Vec<TimerRecord>) {
        let key = DataKey::GrantTimer(GrantTimerKey::Timers(grant_id));
        env.storage().persistent().set(&key, timers);
        Self::bump(env, &key);
    }
}
