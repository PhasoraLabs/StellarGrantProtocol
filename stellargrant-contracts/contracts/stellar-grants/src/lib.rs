#![no_std]
#![allow(clippy::too_many_arguments)]
#![allow(
    dead_code,
    deprecated,
    unused_assignments,
    unused_imports,
    unused_mut,
    unused_variables,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::manual_checked_ops,
    clippy::manual_range_contains,
    clippy::manual_saturating_arithmetic,
    clippy::match_like_matches_macro,
    clippy::match_result_ok,
    clippy::needless_borrows_for_generic_args,
    clippy::redundant_closure,
    clippy::unnecessary_cast,
    clippy::unnecessary_map_or,
    clippy::while_let_loop
)]
mod access_control;
mod analytics;
mod arbitration_pool;
mod audit;
mod badge;
mod checklist;
mod circuit_breaker;
mod collateral;
mod compliance;
mod config;
mod constants;
mod contributor_verification;
mod cross_contract;
mod crowdfund;
mod data_export;
mod dispute;
mod emergency;
mod errors;
mod escrow;
mod escrow_multisig;
mod events;
mod evidence_schema;
mod factory;
mod fees;
mod fork;
mod funder_report;
mod governance;
mod grant_bridge;
mod grant_index;
mod grant_pause;
mod grant_renewal;
mod grant_tags;
mod grant_transfer;
mod hooks;
mod insurance;
mod interfaces;
mod invoice;
mod license;
mod lockup;
mod matching;
pub mod math;
pub mod merkle;
mod metrics;
mod migration;
mod milestone_deps;
mod milestone_extension;
mod milestone_nft;
mod milestone_template;
mod multi_grant;
mod multisig;
mod notification;
mod open_review;
mod oracle;
mod pagination;
mod params;
mod performance_bond;
mod portfolio;
mod provenance;
mod quadratic;
mod rate_limit;
mod reentrancy;
mod referral;
mod registry;
mod relay;
mod reputation;
mod reputation_decay;
mod revenue_share;
mod reviewer_pool;
mod reviewer_reward;
pub mod reviewer_sla;
mod scoring;
mod split_payment;
mod storage;
mod streaming;
mod syndication;
mod token_swap;
mod types;
mod versioning;
mod whitelist;

pub use errors::ContractError;
pub use events::Events;
pub use storage::Storage;
pub use types::{
    AcceptanceCriteria,
    Amendment,
    AmendmentStatus,
    AnalyticsSnapshot,
    // Issue #569/#572/#573/#574: growth, extension, arbitration, and bond modules
    Arbiter,
    ArbiterVote,
    ArbitrationCase,
    AuditAction,
    AuditEntry,
    BatchResult,
    BondClaim,
    BondStatus,
    BreakerState,
    CategoryStats,
    ChecklistSubmission,
    // Issue #564: collateral escrow
    CollateralDeposit,
    CollateralRequirement,
    CollateralStatus,
    ComplianceAttestation,
    ComplianceLevel,
    ComplianceStatus,
    ContractVersion,
    ContributionType,
    ContributorPortfolio,
    CriterionStatus,
    CrowdfundCampaign,
    CrowdfundPledge,
    CrowdfundStatus,
    DecayConfig,
    DecayType,
    DexConfig,
    Dispute,
    DisputeStatus,
    EscrowAccount,
    EscrowLifecycleState,
    EscrowMode,
    EscrowState,
    EvidenceField,
    EvidenceFieldType,
    EvidenceSchema,
    // Issue #619: data export
    ExportGrant,
    ExportGrantPage,
    ExportMilestone,
    ExportMilestonePage,
    ExtensionRequest,
    ExtensionStatus,
    FeeRecord,
    ForkRecord,
    // Issue #598: funder report
    FunderGrantSummary,
    FunderLedger,
    FunderReport,
    FunderTokenSummary,
    Grant,
    GrantArchetype,
    GrantCategory,
    GrantFund,
    GrantPortfolio,
    GrantStatus,
    GrantSummary,
    GrantTag,
    GrantTemplate,
    GrantVersion,
    HookCallResult,
    HookEvent,
    HookRegistration,
    InsuranceClaim,
    InsurancePolicy,
    Invoice,
    InvoiceStatus,
    IpRights,
    LicenseRecord,
    LicenseType,
    LineItem,
    // Issue #609: lockup
    LockupRecord,
    LockupStatus,
    MatchingAllocation,
    MatchingContribution,
    MatchingRound,
    MerkleCommitment,
    MerkleProof,
    MigrationRecord,
    Milestone,
    MilestoneDag,
    MilestoneDependency,
    MilestoneNft,
    MilestoneState,
    MilestoneSubmission,
    MultisigProposal,
    MultisigSigner,
    NftMetadata,
    NotificationEvent,
    OracleConfig,
    ParamRecord,
    ParamType,
    ParamValue,
    PauseRecord,
    PaymentSplit,
    PaymentStream,
    PerformanceBond,
    PortfolioFilter,
    PortfolioStats,
    PriceQuote,
    ProtocolConfig,
    ProtocolMetrics,
    ProtocolModule,
    ProvenanceRecord,
    PublicReview,
    PublicReviewSignal,
    QuadraticVoteRecord,
    RateLimitAction,
    ReferralCode,
    ReferralRecord,
    ReferralReward,
    RegistryEntry,
    RegistryEntryType,
    RelayAllowance,
    RelayConfig,
    RelayRecord,
    RelayableAction,
    RenewalProposal,
    RenewalStatus,
    ReputationTier,
    RevenueEpoch,
    ReviewParticipation,
    ReviewerAvailability,
    ReviewerProfile,
    ReviewerRequest,
    ReviewerRequestStatus,
    ReviewerRewardPool,
    ReviewerRewardRecord,
    Role,
    RoleAssignment,
    RollingWindow,
    ScoreResult,
    ScoringDimension,
    ScoringRubric,
    ScoringWeight,
    SignatureStatus,
    SplitRecipient,
    StakerEpochRecord,
    StructuredEvidence,
    Subscription,
    SubscriptionScope,
    SwapResult,
    SwapRoute,
    SyndicateGrant,
    SyndicateMember,
    SyndicateStatus,
    TokenMetric,
    TransferProposal,
    TransferableRole,
    VerificationAttestation,
    VerificationLevel,
    VerificationStatus,
    VoiceCredits,
    VotingMechanism,
    // Issue #512: whitelist
    WhitelistEntry,
    WhitelistMode,
    WhitelistScope,
};

use metrics::MetricField;
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Map, String, Vec};

#[contract]
pub struct StellarGrantsContract;

#[contractimpl]
impl StellarGrantsContract {
    /// Initialize the contract and record the initial contract version.
    pub fn initialize(env: Env, deployer: Address) -> Result<(), ContractError> {
        deployer.require_auth();
        migration::initialize_version(&env, &deployer, 1, 0, 0)?;
        Ok(())
    }

    /// Configure or rotate a single global admin address.
    pub fn set_global_admin(
        env: Env,
        caller: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        if let Some(current_admin) = Storage::get_global_admin(&env) {
            if current_admin != caller {
                return Err(ContractError::Unauthorized);
            }
        }
        Storage::set_global_admin(&env, &new_admin);
        Ok(())
    }

    /// Allows a grant developer/owner to create a new milestone-based grant.
    #[allow(clippy::too_many_arguments)]
    pub fn grant_create(
        env: Env,
        owner: Address,
        title: String,
        description: String,
        token: Address,
        total_amount: i128,
        milestone_amount: i128,
        num_milestones: u32,
        reviewers: soroban_sdk::Vec<Address>,
    ) -> Result<u64, ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Grants)?;
        owner.require_auth();
        rate_limit::check_and_increment(&env, &owner, RateLimitAction::GrantCreate)?;

        internal_grant_create(
            &env,
            &owner,
            title,
            description,
            &token,
            total_amount,
            milestone_amount,
            num_milestones,
            reviewers,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn grant_create_high_security(
        env: Env,
        owner: Address,
        title: String,
        description: String,
        token: Address,
        total_amount: i128,
        milestone_amount: i128,
        num_milestones: u32,
        reviewers: soroban_sdk::Vec<Address>,
        multisig_signers: soroban_sdk::Vec<Address>,
    ) -> Result<u64, ContractError> {
        if multisig_signers.is_empty() {
            return Err(ContractError::InvalidInput);
        }

        let grant_id = Self::grant_create(
            env.clone(),
            owner,
            title,
            description,
            token,
            total_amount,
            milestone_amount,
            num_milestones,
            reviewers,
        )?;

        Storage::set_escrow_state(
            &env,
            grant_id,
            &EscrowState {
                mode: EscrowMode::HighSecurity,
                lifecycle: EscrowLifecycleState::Funding,
                quorum_ready: false,
                approvals_count: 0,
            },
        );
        Storage::set_multisig_signers(&env, grant_id, &multisig_signers);

        Ok(grant_id)
    }

    /// Register a contributor profile on-chain and add to global registry.
    pub fn contributor_register(
        env: Env,
        contributor: Address,
        name: String,
        bio: String,
        skills: soroban_sdk::Vec<String>,
        github_url: String,
    ) -> Result<(), ContractError> {
        contributor.require_auth();
        rate_limit::check_and_increment(&env, &contributor, RateLimitAction::ContributorRegister)?;

        if name.is_empty() || name.len() > constants::MAX_TITLE_LEN {
            return Err(ContractError::InvalidInput);
        }
        if bio.len() > constants::MAX_BIO_LEN {
            return Err(ContractError::InvalidInput);
        }

        if Storage::get_contributor(&env, contributor.clone()).is_some() {
            return Err(ContractError::AlreadyRegistered);
        }

        let profile = crate::types::ContributorProfile {
            contributor: contributor.clone(),
            name: name.clone(),
            bio,
            skills,
            github_url,
            registration_timestamp: env.ledger().timestamp(),
            reputation_score: 0,
            grants_count: 0,
            total_earned: 0,
            milestones_completed: 0,
            milestones_rejected: 0,
            last_action_at: env.ledger().timestamp(),
        };

        Storage::set_contributor(&env, contributor.clone(), &profile);

        // Register in global index and emit contributor_registered event
        registry::register_contributor(&env, &contributor, &name)?;

        metrics::increment(&env, MetricField::ContributorsRegistered, 1);

        Ok(())
    }

    /// Cancel a grant and refund remaining balance to funders
    pub fn grant_cancel(
        env: Env,
        grant_id: u64,
        owner: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        Self::cancel_grant(env, grant_id, owner, reason)
    }

    /// Cancel a grant and refund escrowed funds. Callable by grant owner or global admin.
    pub fn cancel_grant(
        env: Env,
        grant_id: u64,
        caller: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        reentrancy::with_non_reentrant(&env, || {
            let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            let caller_is_owner = grant.owner == caller;
            let caller_is_admin = Storage::get_global_admin(&env) == Some(caller.clone());
            if !caller_is_owner && !caller_is_admin {
                return Err(ContractError::Unauthorized);
            }

            if grant.status != GrantStatus::Active {
                return Err(ContractError::InvalidState);
            }

            if grant.milestones_paid_out >= grant.total_milestones {
                return Err(ContractError::InvalidState);
            }

            // Issue #564: forfeit collateral on grant abandonment.
            if let Some(req) = collateral::get_requirement(&env, grant_id) {
                let forfeit_reason = String::from_str(&env, "grant cancelled by owner");
                let _ = collateral::forfeit(
                    &env,
                    grant_id,
                    &grant.owner,
                    req.forfeit_on_abandon_bps,
                    forfeit_reason,
                );
            }

            let total_refundable = grant.escrow_balance;
            if total_refundable > 0 {
                escrow::refund_all(&env, grant_id)?;
            }

            let mut grant =
                Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
            let old_status = grant.status;
            grant.status = GrantStatus::Cancelled;
            grant.escrow_balance = 0;
            grant.reason = Some(reason.clone());
            grant.timestamp = env.ledger().timestamp();

            grant_index::on_status_changed(&env, grant_id, old_status, GrantStatus::Cancelled);

            Storage::set_grant(&env, grant_id, &grant);

            Events::emit_grant_cancelled(&env, grant_id, caller.clone(), reason, total_refundable);

            audit::log(
                &env,
                grant_id,
                AuditAction::GrantCancelled,
                &caller,
                None,
                Some(total_refundable),
            );

            metrics::increment(&env, MetricField::GrantsCancelled, 1);
            if total_refundable > 0 {
                metrics::record_token_refund(&env, &grant.token, total_refundable);
            }

            Ok(())
        })
    }

    /// Mark a grant as completed when all milestones are approved and refund the remaining balance
    pub fn grant_complete(env: Env, grant_id: u64) -> Result<(), ContractError> {
        reentrancy::with_non_reentrant(&env, || {
            let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            if grant.status != GrantStatus::Active {
                return Err(ContractError::InvalidState);
            }

            let mut escrow_state = Storage::get_escrow_state(&env, grant_id);
            if escrow_state.lifecycle == EscrowLifecycleState::Released {
                return Err(ContractError::GrantAlreadyReleased);
            }

            let _ =
                Self::compute_total_paid_if_quorum_ready(&env, grant_id, grant.total_milestones)?;
            escrow_state.quorum_ready = true;

            if escrow_state.mode == EscrowMode::Standard {
                Self::finalize_grant_release(&env, grant_id)?;
                return Ok(());
            }

            escrow_state.lifecycle = EscrowLifecycleState::AwaitingMultisig;
            Storage::set_escrow_state(&env, grant_id, &escrow_state);
            Ok(())
        })
    }

    pub fn sign_release(env: Env, grant_id: u64, signer: Address) -> Result<(), ContractError> {
        signer.require_auth();
        reentrancy::with_non_reentrant(&env, || {
            let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
            if grant.status != GrantStatus::Active {
                return Err(ContractError::InvalidState);
            }

            let mut escrow_state = Storage::get_escrow_state(&env, grant_id);
            if escrow_state.mode != EscrowMode::HighSecurity {
                return Err(ContractError::InvalidState);
            }
            if escrow_state.lifecycle == EscrowLifecycleState::Released {
                return Err(ContractError::GrantAlreadyReleased);
            }

            let signers = Storage::get_multisig_signers(&env, grant_id);
            if !signers.contains(signer.clone()) {
                return Err(ContractError::NotMultisigSigner);
            }
            if Storage::has_release_approval(&env, grant_id, &signer) {
                return Err(ContractError::AlreadySignedRelease);
            }

            Storage::set_release_approval(&env, grant_id, &signer, true);
            escrow_state.approvals_count += 1;
            Storage::set_escrow_state(&env, grant_id, &escrow_state);

            let approvals_complete = escrow_state.approvals_count >= signers.len();
            if approvals_complete && escrow_state.quorum_ready {
                Self::finalize_grant_release(&env, grant_id)?;
            } else if approvals_complete {
                escrow_state.lifecycle = EscrowLifecycleState::AwaitingMultisig;
                Storage::set_escrow_state(&env, grant_id, &escrow_state);
            }

            Ok(())
        })
    }

    fn compute_total_paid_if_quorum_ready(
        env: &Env,
        grant_id: u64,
        total_milestones: u32,
    ) -> Result<i128, ContractError> {
        let mut total_paid: i128 = 0;
        let mut approved_count = 0;
        for milestone_idx in 0..total_milestones {
            if let Some(milestone) = Storage::get_milestone(env, grant_id, milestone_idx) {
                if milestone.state != MilestoneState::Approved {
                    return Err(ContractError::NotAllMilestonesApproved);
                }
                total_paid += milestone.amount;
                approved_count += 1;
            } else {
                return Err(ContractError::NotAllMilestonesApproved);
            }
        }
        if approved_count != total_milestones {
            return Err(ContractError::NotAllMilestonesApproved);
        }
        Ok(total_paid)
    }

    fn finalize_grant_release(env: &Env, grant_id: u64) -> Result<(), ContractError> {
        let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if grant.status != GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        crate::grant_pause::require_not_paused(env, grant_id)?;

        // Compliance gate: if the grant requires KYC, check the owner/contributor.
        if let Some(required_level) = grant.require_compliance {
            compliance::require_compliant_u32(env, &grant.owner, required_level)?;
        }

        let total_paid =
            Self::compute_total_paid_if_quorum_ready(env, grant_id, grant.total_milestones)?;
        if grant.escrow_balance < total_paid {
            return Err(ContractError::InvalidInput);
        }
        let remaining_balance = math::safe_sub(grant.escrow_balance, total_paid)?;

        if total_paid > 0 {
            // Pay each milestone individually so that registered splits are honoured.
            let protocol_cfg = config::get_config(env);
            let mut owner_amount: i128 = 0;
            for idx in 0..grant.total_milestones {
                let ms = Storage::get_milestone(env, grant_id, idx)
                    .ok_or(ContractError::MilestoneNotFound)?;
                if ms.amount > protocol_cfg.kyc_payout_threshold {
                    contributor_verification::require_verified(
                        env,
                        &grant.owner,
                        VerificationLevel::FullKyc,
                    )?;
                }
                if split_payment::has_split(env, grant_id, idx) {
                    split_payment::execute_split(env, grant_id, idx, ms.amount)?;
                } else {
                    owner_amount = owner_amount.saturating_add(ms.amount);
                }
            }
            if owner_amount > 0 {
                let config: ProtocolConfig = env.storage().persistent().get(&storage::keys::DataKey::Config).unwrap();
                if config.multisig_threshold > 0 && owner_amount >= config.multisig_threshold {
                    crate::escrow_multisig::create_request(env, grant_id, 0, owner_amount, grant.owner.clone())?;
                } else {
                    escrow::release(env, grant_id, &grant.owner, owner_amount)?;
                }
            }
        }
        if remaining_balance > 0 {
            escrow::refund_all(env, grant_id)?;
        }

        // Re-load grant after escrow mutations.
        let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
        grant.status = GrantStatus::Completed;
        grant.escrow_balance = 0;
        grant.milestones_paid_out = grant.total_milestones;
        grant.timestamp = env.ledger().timestamp();
        Storage::set_grant(env, grant_id, &grant);

        let mut escrow_state = Storage::get_escrow_state(env, grant_id);
        escrow_state.lifecycle = EscrowLifecycleState::Released;
        escrow_state.quorum_ready = true;
        Storage::set_escrow_state(env, grant_id, &escrow_state);

        metrics::increment(env, MetricField::GrantsCompleted, 1);
        metrics::update_token_locked(env, &grant.token, -total_paid);

        // Issue #574: return any active performance bond to the guarantor.
        performance_bond::release_bond(env, grant_id)?;

        // Issue #564: release any collateral deposit back to the contributor.
        let _ = collateral::release(env, grant_id, &grant.owner);

        Events::emit_grant_completed(env, grant_id, total_paid, remaining_balance);
        Ok(())
    }

    /// Allows authorized reviewers to vote on submitted milestones.
    /// Delegates all voting logic to governance::cast_vote.
    pub fn milestone_vote(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        reviewer: Address,
        approve: bool,
        feedback: Option<String>,
    ) -> Result<bool, ContractError> {
        emergency::require_not_paused(&env)?;
        crate::grant_pause::require_not_paused(&env, grant_id)?;
        circuit_breaker::require_open(&env, ProtocolModule::Grants)?;
        reviewer.require_auth();

        let mut grant = Storage::get_grant_v(&env, grant_id);
        let mut milestone = Storage::get_milestone_v(&env, grant_id, milestone_idx);

        if approve && !checklist::all_required_approved(&env, grant_id, milestone_idx) {
            return Err(ContractError::RequiredCriteriaNotMet);
        }

        let result = governance::cast_vote(
            &env,
            &mut grant,
            &mut milestone,
            &reviewer,
            approve,
            feedback,
        )?;

        Storage::set_milestone(&env, grant_id, milestone_idx, &milestone);

        provenance::record(
            &env,
            ContributionType::MilestoneReviewed,
            &reviewer,
            grant_id,
            Some(milestone_idx),
            None,
            Some(grant.token.clone()),
            soroban_sdk::Vec::new(&env),
        );

        reviewer_reward::record_participation(&env, &reviewer, grant_id, false);

        if result.quorum_reached {
            if result.approved {
                Self::update_contributor_reputation(
                    &env,
                    grant_id,
                    milestone_idx,
                    &grant.owner,
                    grant.milestone_amount,
                );
                audit::log(
                    &env,
                    grant_id,
                    AuditAction::MilestoneApproved,
                    &reviewer,
                    Some(milestone_idx),
                    Some(milestone.amount),
                );
                metrics::increment(&env, MetricField::MilestonesApproved, 1);
                if hooks::has_hooks(&env, HookEvent::MilestoneApproved) {
                    hooks::trigger(&env, HookEvent::MilestoneApproved, Bytes::new(&env));
                }
                // Mint soulbound NFT certificate for the contributor (#570)
                let meta = NftMetadata {
                    name: milestone.description.clone(),
                    description: milestone.description.clone(),
                    grant_title: grant.title.clone(),
                    image_uri: String::from_str(&env, ""),
                    attributes: soroban_sdk::Vec::new(&env),
                };
                let _ = milestone_nft::mint(&env, grant_id, milestone_idx, &grant.owner, meta);
                // Track this grant in the contributor's portfolio index (#565)
                Storage::push_contributor_grant_id(&env, &grant.owner, grant_id);
            } else {
                audit::log(
                    &env,
                    grant_id,
                    AuditAction::MilestoneRejected,
                    &reviewer,
                    Some(milestone_idx),
                    None,
                );
                metrics::increment(&env, MetricField::MilestonesRejected, 1);
            }
        }

        Ok(result.quorum_reached)
    }

    /// Allows authorized reviewers to reject milestones with a reason.
    pub fn milestone_reject(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        reviewer: Address,
        reason: String,
    ) -> Result<bool, ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Grants)?;
        reviewer.require_auth();

        let grant = Storage::get_grant_v(&env, grant_id);
        let mut milestone = Storage::get_milestone_v(&env, grant_id, milestone_idx);

        if milestone.state != MilestoneState::Submitted {
            env.panic_with_error(ContractError::MilestoneNotSubmitted);
        }

        if !grant.reviewers.contains(reviewer.clone()) {
            env.panic_with_error(ContractError::Unauthorized);
        }

        if milestone.votes.contains_key(reviewer.clone()) {
            env.panic_with_error(ContractError::AlreadyVoted);
        }

        let reputation = Storage::get_reviewer_reputation(&env, reviewer.clone());
        milestone.votes.set(reviewer.clone(), false);
        milestone.rejections += reputation;
        milestone.reasons.set(reviewer.clone(), reason.clone());

        let total_weight = milestone.reviewer_count_snapshot;

        let majority_threshold = (total_weight / 2) + 1;
        let majority_rejected = milestone.rejections >= majority_threshold;

        if majority_rejected {
            milestone.state = MilestoneState::Rejected;
            milestone.status_updated_at = env.ledger().timestamp();

            for (voter, voted_approve) in milestone.votes.iter() {
                if !voted_approve {
                    let mut rep = Storage::get_reviewer_reputation(&env, voter.clone());
                    rep += 1;
                    Storage::set_reviewer_reputation(&env, voter.clone(), rep);
                }
            }

            Events::milestone_status_changed(
                &env,
                grant_id,
                milestone_idx,
                MilestoneState::Rejected,
            );
        }

        Storage::set_milestone(&env, grant_id, milestone_idx, &milestone);
        Events::milestone_rejected(&env, grant_id, milestone_idx, reviewer, reason);

        Ok(majority_rejected)
    }

    /// Allows a grant recipient to submit a completed milestone for reviewer evaluation.
    pub fn milestone_submit(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        recipient: Address,
        description: String,
        proof_url: String,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        crate::grant_pause::require_not_paused(&env, grant_id)?;
        circuit_breaker::require_open(&env, ProtocolModule::Grants)?;
        recipient.require_auth();
        rate_limit::check_and_increment(&env, &recipient, RateLimitAction::MilestoneSubmit)?;

        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

        if grant.status != GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        if grant.owner != recipient {
            return Err(ContractError::Unauthorized);
        }

        // Issue #574: a required bond must be posted before any milestone submission.
        require_bond_posted(&env, grant_id)?;
        require_bond_posted(&env, grant_id)?;

        // Issue #564: a required collateral deposit must be posted before submission.
        collateral::require_deposited(&env, grant_id, &recipient)?;

        apply_milestone_submission(
            &env,
            grant_id,
            &grant,
            milestone_idx,
            description,
            proof_url,
            &recipient,
        )
    }

    /// Submits multiple milestones in one transaction.
    pub fn milestone_submit_batch(
        env: Env,
        grant_id: u64,
        recipient: Address,
        submissions: Vec<MilestoneSubmission>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        crate::grant_pause::require_not_paused(&env, grant_id)?;
        recipient.require_auth();

        let batch_len = submissions.len();
        if batch_len == 0 {
            return Err(ContractError::BatchEmpty);
        }
        if batch_len > constants::MAX_BATCH_SIZE {
            return Err(ContractError::BatchTooLarge);
        }

        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

        if grant.status != GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        if grant.owner != recipient {
            return Err(ContractError::Unauthorized);
        }

        // Issue #574: a required bond must be posted before any milestone submission.
        require_bond_posted(&env, grant_id)?;
        require_bond_posted(&env, grant_id)?;

        // Issue #564: a required collateral deposit must be posted before submission.
        collateral::require_deposited(&env, grant_id, &recipient)?;

        for sub in submissions.iter() {
            apply_milestone_submission(
                &env,
                grant_id,
                &grant,
                sub.idx,
                sub.description.clone(),
                sub.proof.clone(),
                &recipient,
            )?;
        }

        Ok(())
    }

    /// Allows a funder to deposit tokens into escrow for a specific grant.
    pub fn grant_fund(
        env: Env,
        grant_id: u64,
        funder: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Grants)?;
        funder.require_auth();
        reentrancy::with_non_reentrant(&env, || {
            if amount <= 0 {
                return Err(ContractError::ZeroAmount);
            }

            let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            if grant.status != GrantStatus::Active {
                return Err(ContractError::InvalidState);
            }

            escrow::deposit(&env, grant_id, &funder, amount)?;

            let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            Events::emit_grant_funded(&env, grant_id, funder.clone(), amount, grant.escrow_balance);

            audit::log(
                &env,
                grant_id,
                AuditAction::GrantFunded,
                &funder,
                None,
                Some(amount),
            );

            provenance::record(
                &env,
                ContributionType::GrantFunded,
                &funder,
                grant_id,
                None,
                Some(amount),
                Some(grant.token.clone()),
                soroban_sdk::Vec::new(&env),
            );

            metrics::update_token_locked(&env, &grant.token, amount);

            Ok(())
        })
    }

    /// Retrieve a grant by its ID
    pub fn get_grant(env: Env, grant_id: u64) -> Result<Grant, ContractError> {
        Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)
    }

    pub fn get_milestone(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<Milestone, ContractError> {
        let grant = Storage::get_grant_v(&env, grant_id);

        if milestone_idx >= grant.total_milestones {
            env.panic_with_error(ContractError::MilestoneIndexOutOfBounds);
        }

        let milestone = Storage::get_milestone_v(&env, grant_id, milestone_idx);
        Ok(milestone)
    }

    /// Retrieve all reviewer feedback for a milestone
    pub fn get_milestone_feedback(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<soroban_sdk::Map<Address, String>, ContractError> {
        let milestone = Self::get_milestone(env, grant_id, milestone_idx)?;
        Ok(milestone.reasons)
    }

    /// Return the full immutable audit log for a grant.
    pub fn get_audit_log(env: Env, grant_id: u64) -> Vec<AuditEntry> {
        audit::get_log(&env, grant_id)
    }

    // ── Contract Version Query (#527) ───────────────────────────────────

    /// Query the stored contract version.
    pub fn get_contract_version(env: Env) -> Option<ContractVersion> {
        migration::get_version(&env)
    }

    /// Run a versioned schema migration. Admin only.
    pub fn run_migration(
        env: Env,
        admin: Address,
        target_version: ContractVersion,
    ) -> Result<MigrationRecord, ContractError> {
        admin.require_auth();
        migration::run_migration(&env, &admin, target_version)
    }

    /// Return the full migration history log.
    pub fn migration_history(env: Env) -> Vec<MigrationRecord> {
        migration::migration_history(&env)
    }

    // ── Global Registry (#520) ──────────────────────────────────────────

    /// Paginated list of all registered contributors.
    pub fn get_contributors_page(env: Env, offset: u32, limit: u32) -> Vec<RegistryEntry> {
        registry::get_contributors_page(&env, offset, limit)
    }

    /// Total count of registered contributors.
    pub fn contributor_count(env: Env) -> u32 {
        registry::contributor_count(&env)
    }

    /// Check if an address is on the approved reviewer allowlist.
    pub fn is_approved_reviewer(env: Env, address: Address) -> bool {
        registry::is_approved_reviewer(&env, &address)
    }

    /// Add an address to the approved reviewer allowlist. Admin only.
    pub fn approve_reviewer(
        env: Env,
        admin: Address,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        registry::approve_reviewer(&env, &admin, &reviewer)
    }

    /// Remove an address from the approved reviewer allowlist. Admin only.
    pub fn revoke_reviewer(
        env: Env,
        admin: Address,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        registry::revoke_reviewer(&env, &admin, &reviewer)
    }

    // ── Reviewer Staking (#42) ──────────────────────────────────────

    /// Admin sets the minimum stake required for reviewers and the treasury address.
    pub fn set_staking_config(
        env: Env,
        admin: Address,
        min_stake: i128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if min_stake <= 0 {
            return Err(ContractError::InvalidInput);
        }
        env.storage()
            .persistent()
            .set(&storage::DataKey::MinReviewerStake, &min_stake);
        env.storage()
            .persistent()
            .set(&storage::DataKey::Treasury, &treasury);
        Ok(())
    }

    /// Reviewer stakes tokens to participate in a grant's review quorum.
    pub fn stake_to_review(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        amount: i128,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if grant.status != GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        let min_stake = Storage::get_min_reviewer_stake(&env);
        if amount < min_stake {
            return Err(ContractError::InsufficientStake);
        }

        escrow::transfer_token(
            &env,
            &grant.token,
            &reviewer,
            &env.current_contract_address(),
            amount,
        );

        let current = Storage::get_reviewer_stake(&env, grant_id, &reviewer);
        Storage::set_reviewer_stake(&env, grant_id, &reviewer, current + amount);

        Ok(())
    }

    /// Admin slashes a malicious reviewer's stake, sending it to treasury.
    pub fn slash_reviewer(
        env: Env,
        admin: Address,
        grant_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        let stake = Storage::get_reviewer_stake(&env, grant_id, &reviewer);
        if stake <= 0 {
            return Err(ContractError::StakeNotFound);
        }

        let treasury = Storage::get_treasury(&env).ok_or(ContractError::InvalidInput)?;
        escrow::transfer_token(
            &env,
            &grant.token,
            &env.current_contract_address(),
            &treasury,
            stake,
        );

        Storage::set_reviewer_stake(&env, grant_id, &reviewer, 0);

        Ok(())
    }

    /// Reviewer unstakes tokens after a grant lifecycle completes.
    pub fn unstake(env: Env, reviewer: Address, grant_id: u64) -> Result<(), ContractError> {
        reviewer.require_auth();

        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if grant.status == GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        let stake = Storage::get_reviewer_stake(&env, grant_id, &reviewer);
        if stake <= 0 {
            return Err(ContractError::StakeNotFound);
        }

        escrow::transfer_token(
            &env,
            &grant.token,
            &env.current_contract_address(),
            &reviewer,
            stake,
        );

        Storage::set_reviewer_stake(&env, grant_id, &reviewer, 0);

        Ok(())
    }

    /// Reviewer claims all pending rewards for a specific token.
    pub fn claim_reviewer_rewards(
        env: Env,
        reviewer: Address,
        token: Address,
    ) -> Result<i128, ContractError> {
        reviewer.require_auth();
        reviewer_reward::claim_rewards(&env, &reviewer, &token)
    }

    /// Get pending reviewer rewards for a specific token.
    pub fn get_reviewer_rewards(
        env: Env,
        reviewer: Address,
        token: Address,
    ) -> Option<ReviewerRewardRecord> {
        reviewer_reward::get_reward_record(&env, &reviewer, &token)
    }

    /// Get reviewer reward pool balance for a token.
    pub fn get_reviewer_reward_pool_balance(env: Env, token: Address) -> i128 {
        reviewer_reward::pool_balance(&env, &token)
    }

    // ── Multi-Grant Portfolio Management ───────────────────────────────────────

    /// Get aggregated portfolio statistics for a grant owner.
    pub fn get_portfolio_stats(env: Env, owner: Address) -> PortfolioStats {
        multi_grant::get_portfolio_stats(&env, &owner)
    }

    /// Get all grant IDs matching a filter, paginated.
    pub fn filter_grants(env: Env, filter: PortfolioFilter, offset: u32, limit: u32) -> Vec<u64> {
        multi_grant::filter_grants(&env, filter, offset, limit)
    }

    /// Add a reviewer to multiple grants in one call.
    pub fn batch_add_reviewer(
        env: Env,
        owner: Address,
        grant_ids: Vec<u64>,
        reviewer: Address,
    ) -> Result<BatchResult, ContractError> {
        owner.require_auth();
        multi_grant::batch_add_reviewer(&env, &owner, grant_ids, &reviewer)
    }

    /// Remove a reviewer from multiple grants.
    pub fn batch_remove_reviewer(
        env: Env,
        owner: Address,
        grant_ids: Vec<u64>,
        reviewer: Address,
    ) -> Result<BatchResult, ContractError> {
        owner.require_auth();
        multi_grant::batch_remove_reviewer(&env, &owner, grant_ids, &reviewer)
    }

    /// Get total escrow balance across all grants for an owner and token.
    pub fn get_total_escrow_balance(env: Env, owner: Address, token: Address) -> i128 {
        multi_grant::total_escrow_balance(&env, &owner, &token)
    }

    /// Get the n most recently active grants for an owner.
    pub fn get_recent_grants(env: Env, owner: Address, n: u32) -> Vec<u64> {
        multi_grant::recent_grants(&env, &owner, n)
    }

    /// Get full grant portfolio view for an owner.
    pub fn get_grant_portfolio(env: Env, owner: Address) -> GrantPortfolio {
        multi_grant::get_portfolio(&env, &owner)
    }

    // ── Quadratic Funding Matching Rounds ──────────────────────────────────────

    /// Create a new QF matching round with a pool of matching funds.
    pub fn create_matching_round(
        env: Env,
        admin: Address,
        token: Address,
        matching_pool: i128,
        duration_ledgers: u32,
        eligible_grant_ids: Vec<u64>,
    ) -> Result<u32, ContractError> {
        admin.require_auth();
        matching::create_round(
            &env,
            &admin,
            &token,
            matching_pool,
            duration_ledgers,
            eligible_grant_ids,
        )
    }

    /// Contribute to a grant within an active matching round.
    pub fn contribute_to_matching_round(
        env: Env,
        contributor: Address,
        round_id: u32,
        grant_id: u64,
        amount: i128,
    ) -> Result<(), ContractError> {
        contributor.require_auth();
        matching::contribute(&env, &contributor, round_id, grant_id, amount)
    }

    /// Compute QF allocations after a round ends.
    pub fn compute_qf_allocations(
        env: Env,
        round_id: u32,
    ) -> Result<Vec<MatchingAllocation>, ContractError> {
        matching::compute_allocations(&env, round_id)
    }

    /// Distribute match amounts to eligible grants' escrows.
    pub fn distribute_matching_rewards(env: Env, round_id: u32) -> Result<(), ContractError> {
        matching::distribute(&env, round_id)
    }

    /// Get a specific matching round.
    pub fn get_matching_round(env: Env, round_id: u32) -> Result<MatchingRound, ContractError> {
        matching::get_round(&env, round_id)
    }

    /// Get a contributor's contribution to a grant in a round.
    pub fn get_matching_contribution(
        env: Env,
        round_id: u32,
        contributor: Address,
        grant_id: u64,
    ) -> Option<MatchingContribution> {
        matching::get_contribution(&env, round_id, &contributor, grant_id)
    }

    /// Get all allocations for a round after computation.
    pub fn get_matching_allocations(env: Env, round_id: u32) -> Vec<MatchingAllocation> {
        matching::get_allocations(&env, round_id)
    }

    // ── Milestone Templates ────────────────────────────────────────────────────
    
    pub fn save_template(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: crate::types::TemplateCategory,
        default_amount_pct: u32,
        is_public: bool,
    ) -> Result<u64, ContractError> {
        milestone_template::save_template(&env, owner, name, description, category, default_amount_pct, is_public)
    }
    
    pub fn create_milestones_from_template(
        env: Env,
        caller: Address,
        template_ids: Vec<u64>,
        total_amount: i128,
    ) -> Result<Vec<(String, i128)>, ContractError> {
        milestone_template::create_from_templates(&env, caller, template_ids, total_amount)
    }

    // ── KYC Integration (#43) ───────────────────────────────────────

    /// Admin sets the identity oracle contract address for KYC verification.
    pub fn set_identity_oracle(
        env: Env,
        admin: Address,
        oracle: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&storage::DataKey::IdentityOracle, &oracle);
        Ok(())
    }

    // ── Emergency Pause (#521) ──────────────────────────────────────

    /// Pause the contract. Global admin only.
    pub fn pause(env: Env, admin: Address, reason: String) -> Result<(), ContractError> {
        emergency::pause(&env, &admin, reason)
    }

    /// Unpause the contract. Global admin only.
    pub fn unpause(env: Env, admin: Address) -> Result<(), ContractError> {
        emergency::unpause(&env, &admin)
    }

    /// Returns true if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        emergency::is_paused(&env)
    }

    /// Return the full history of pause/unpause events.
    pub fn pause_history(env: Env) -> Vec<PauseRecord> {
        emergency::pause_history(&env)
    }

    // ── Bulk Funding (#44) ──────────────────────────────────────────

    /// Fund multiple grants in a single transaction.
    pub fn fund_batch(
        env: Env,
        funder: Address,
        grants: Vec<(u64, i128)>,
    ) -> Result<(), ContractError> {
        funder.require_auth();

        let batch_len = grants.len();
        if batch_len == 0 {
            return Err(ContractError::BatchEmpty);
        }
        if batch_len > constants::MAX_BATCH_SIZE {
            return Err(ContractError::BatchTooLarge);
        }

        for item in grants.iter() {
            let (grant_id, amount) = item;
            if amount <= 0 {
                return Err(ContractError::ZeroAmount);
            }

            let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            if grant.status != GrantStatus::Active {
                return Err(ContractError::InvalidState);
            }

            escrow::deposit(&env, grant_id, &funder, amount)?;

            let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            Events::emit_grant_funded(&env, grant_id, funder.clone(), amount, grant.escrow_balance);
            metrics::update_token_locked(&env, &grant.token, amount);
        }

        Ok(())
    }

    // ── Streaming Payments (#531) ───────────────────────────────────────────

    /// Create a new payment stream for a grant milestone.
    pub fn create_stream(
        env: Env,
        sender: Address,
        recipient: Address,
        grant_id: u64,
        token: Address,
        rate_per_ledger: i128,
        duration_ledgers: u32,
    ) -> Result<u32, ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Streaming)?;
        streaming::create_stream(
            &env,
            &sender,
            &recipient,
            grant_id,
            &token,
            rate_per_ledger,
            duration_ledgers,
        )
    }

    /// Recipient withdraws accrued tokens from a stream.
    pub fn withdraw_stream(
        env: Env,
        recipient: Address,
        stream_id: u32,
    ) -> Result<i128, ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Streaming)?;
        streaming::withdraw_stream(&env, &recipient, stream_id)
    }

    /// Cancel a stream, splitting remaining deposit between sender and recipient.
    pub fn cancel_stream(
        env: Env,
        sender: Address,
        stream_id: u32,
    ) -> Result<(i128, i128), ContractError> {
        streaming::cancel_stream(&env, &sender, stream_id)
    }

    /// Pause an active stream.
    pub fn pause_stream(env: Env, sender: Address, stream_id: u32) -> Result<(), ContractError> {
        streaming::pause_stream(&env, &sender, stream_id)
    }

    /// Resume a paused stream.
    pub fn resume_stream(env: Env, sender: Address, stream_id: u32) -> Result<(), ContractError> {
        streaming::resume_stream(&env, &sender, stream_id)
    }

    /// Get stream details by id.
    pub fn get_stream(env: Env, stream_id: u32) -> Result<PaymentStream, ContractError> {
        streaming::get_stream(&env, stream_id)
    }

    // ── Quadratic Voting (#537) ─────────────────────────────────────────────

    /// Allocate voice credits to a reviewer for a grant.
    pub fn allocate_voice_credits(
        env: Env,
        admin: Address,
        voter: Address,
        grant_id: u64,
        credits: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if Storage::get_global_admin(&env) != Some(admin.clone()) {
            return Err(ContractError::Unauthorized);
        }
        quadratic::allocate_credits(&env, &voter, grant_id, credits)
    }

    /// Cast a quadratic vote on a milestone.
    pub fn cast_qv_vote(
        env: Env,
        voter: Address,
        grant_id: u64,
        milestone_idx: u32,
        votes: u32,
        in_favor: bool,
    ) -> Result<QuadraticVoteRecord, ContractError> {
        emergency::require_not_paused(&env)?;
        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if !grant.reviewers.contains(voter.clone()) {
            return Err(ContractError::Unauthorized);
        }
        quadratic::cast_qv_vote(&env, &voter, grant_id, milestone_idx, votes, in_favor)
    }

    /// Return remaining voice credits for a voter on a grant.
    pub fn remaining_voice_credits(env: Env, voter: Address, grant_id: u64) -> u32 {
        quadratic::remaining_credits(&env, &voter, grant_id)
    }

    /// Check if a milestone is approved by QV tally.
    pub fn is_qv_approved(env: Env, grant_id: u64, milestone_idx: u32) -> bool {
        quadratic::is_approved_qv(&env, grant_id, milestone_idx)
    }

    /// Return all QV vote records for a milestone.
    pub fn get_qv_votes(env: Env, grant_id: u64, milestone_idx: u32) -> Vec<QuadraticVoteRecord> {
        quadratic::get_qv_votes(&env, grant_id, milestone_idx)
    }

    // ── Grant Insurance Pool (#538) ─────────────────────────────────────────

    /// Purchase insurance for a grant.
    pub fn purchase_insurance(
        env: Env,
        policyholder: Address,
        grant_id: u64,
        token: Address,
        coverage_amount: i128,
    ) -> Result<InsurancePolicy, ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Insurance)?;
        insurance::purchase_policy(&env, &policyholder, grant_id, &token, coverage_amount)
    }

    /// File an insurance claim for a grant.
    pub fn file_insurance_claim(
        env: Env,
        claimant: Address,
        grant_id: u64,
        claimed_amount: i128,
        reason: String,
    ) -> Result<u32, ContractError> {
        emergency::require_not_paused(&env)?;
        insurance::file_claim(&env, &claimant, grant_id, claimed_amount, reason)
    }

    /// Approve and pay out a claim. Admin only.
    pub fn approve_insurance_claim(
        env: Env,
        admin: Address,
        claim_id: u32,
        payout_amount: i128,
    ) -> Result<(), ContractError> {
        if Storage::get_global_admin(&env) != Some(admin.clone()) {
            return Err(ContractError::Unauthorized);
        }
        insurance::approve_claim(&env, &admin, claim_id, payout_amount)
    }

    /// Reject a claim. Admin only.
    pub fn reject_insurance_claim(
        env: Env,
        admin: Address,
        claim_id: u32,
    ) -> Result<(), ContractError> {
        if Storage::get_global_admin(&env) != Some(admin.clone()) {
            return Err(ContractError::Unauthorized);
        }
        insurance::reject_claim(&env, &admin, claim_id)
    }

    /// Return insurance pool balance for a token.
    pub fn insurance_pool_balance(env: Env, token: Address) -> i128 {
        insurance::pool_balance(&env, &token)
    }

    /// Return the insurance policy for a grant.
    pub fn get_insurance_policy(env: Env, grant_id: u64) -> Option<InsurancePolicy> {
        insurance::get_policy(&env, grant_id)
    }

    /// Return a claim by id.
    pub fn get_insurance_claim(env: Env, claim_id: u32) -> Result<InsuranceClaim, ContractError> {
        insurance::get_claim(&env, claim_id)
    }

    // ── External Callback Hooks (#539) ──────────────────────────────────────

    /// Register an external contract hook for an event. Admin only.
    pub fn register_hook(
        env: Env,
        admin: Address,
        event: HookEvent,
        target_contract: Address,
        max_gas_budget: u32,
    ) -> Result<u32, ContractError> {
        if Storage::get_global_admin(&env) != Some(admin.clone()) {
            return Err(ContractError::Unauthorized);
        }
        hooks::register_hook(&env, &admin, event, target_contract, max_gas_budget)
    }

    /// Deactivate a registered hook. Admin only.
    pub fn deactivate_hook(
        env: Env,
        admin: Address,
        event: HookEvent,
        hook_index: u32,
    ) -> Result<(), ContractError> {
        if Storage::get_global_admin(&env) != Some(admin.clone()) {
            return Err(ContractError::Unauthorized);
        }
        hooks::deactivate_hook(&env, &admin, event, hook_index)
    }

    /// Return all registered hooks for an event.
    pub fn get_hooks(env: Env, event: HookEvent) -> Vec<HookRegistration> {
        hooks::get_hooks(&env, event)
    }

    /// Check if any active hooks are registered for an event.
    pub fn has_hooks(env: Env, event: HookEvent) -> bool {
        hooks::has_hooks(&env, event)
    }

    // ── Issue #545: Merkle evidence commitments ───────────────────────────────

    pub fn commit_evidence_root(
        env: Env,
        contributor: Address,
        grant_id: u64,
        milestone_idx: u32,
        root: Bytes,
        leaf_count: u32,
    ) -> Result<(), ContractError> {
        merkle::commit_evidence_root(
            &env,
            &contributor,
            grant_id,
            milestone_idx,
            root,
            leaf_count,
        )
    }

    pub fn verify_evidence_proof(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        proof: MerkleProof,
    ) -> bool {
        merkle::verify_proof(&env, grant_id, milestone_idx, &proof)
    }

    pub fn get_merkle_commitment(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<MerkleCommitment> {
        merkle::get_commitment(&env, grant_id, milestone_idx)
    }

    // ── Issue #541: Grant templates ───────────────────────────────────────────

    pub fn create_from_template(
        env: Env,
        owner: Address,
        archetype: GrantArchetype,
        title: String,
        description: String,
        token: Address,
        total_amount: i128,
        reviewers: soroban_sdk::Vec<Address>,
    ) -> Result<u64, ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Grants)?;
        owner.require_auth();
        rate_limit::check_and_increment(&env, &owner, RateLimitAction::GrantCreate)?;
        factory::create_from_template(
            &env,
            &owner,
            archetype,
            title,
            description,
            &token,
            total_amount,
            reviewers,
        )
    }

    pub fn create_from_custom_template(
        env: Env,
        owner: Address,
        template: GrantTemplate,
        title: String,
        description: String,
        token: Address,
        total_amount: i128,
        reviewers: soroban_sdk::Vec<Address>,
    ) -> Result<u64, ContractError> {
        emergency::require_not_paused(&env)?;
        circuit_breaker::require_open(&env, ProtocolModule::Grants)?;
        owner.require_auth();
        rate_limit::check_and_increment(&env, &owner, RateLimitAction::GrantCreate)?;
        factory::create_from_custom_template(
            &env,
            &owner,
            template,
            title,
            description,
            &token,
            total_amount,
            reviewers,
        )
    }

    pub fn template_for(archetype: GrantArchetype) -> GrantTemplate {
        factory::template_for(archetype)
    }

    pub fn list_archetypes(env: Env) -> soroban_sdk::Vec<GrantTemplate> {
        factory::list_archetypes(&env)
    }

    pub fn validate_grant_template(template: GrantTemplate) -> Result<(), ContractError> {
        factory::validate_template(&template)
    }

    // ── Issue #544: Rate limit admin ──────────────────────────────────────────

    pub fn reset_rate_limit(
        env: Env,
        admin: Address,
        address: Address,
        action: RateLimitAction,
    ) -> Result<(), ContractError> {
        rate_limit::reset_record(&env, &admin, &address, action)
    }

    // ── Issue #514: Dispute Resolution Entry Points ───────────────────────────

    pub fn dispute_raise(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        caller: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        rate_limit::check_and_increment(&env, &caller, RateLimitAction::DisputeRaise)?;
        emergency::require_not_paused(&env)?;
        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        dispute::raise_dispute(&env, &grant, milestone_idx, &caller, reason)?;
        metrics::increment(&env, MetricField::DisputesRaised, 1);
        Ok(())
    }

    pub fn dispute_assign_arbiter(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        admin: Address,
        arbiter: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if Storage::get_global_admin(&env) != Some(admin.clone()) {
            return Err(ContractError::Unauthorized);
        }
        let mut d = Storage::get_dispute(&env, grant_id, milestone_idx)
            .ok_or(ContractError::InvalidState)?;
        dispute::assign_arbiter(&env, &mut d, &admin, &arbiter)
    }

    pub fn dispute_arbiter_vote(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        arbiter: Address,
        favor_contributor: bool,
    ) -> Result<(), ContractError> {
        arbiter.require_auth();
        let mut d = Storage::get_dispute(&env, grant_id, milestone_idx)
            .ok_or(ContractError::InvalidState)?;
        dispute::arbiter_vote(&env, &mut d, &arbiter, favor_contributor)
    }

    pub fn dispute_resolve(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        caller: Address,
    ) -> Result<DisputeStatus, ContractError> {
        caller.require_auth();
        if Storage::get_global_admin(&env) != Some(caller.clone()) {
            return Err(ContractError::Unauthorized);
        }
        let mut grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        let mut d = Storage::get_dispute(&env, grant_id, milestone_idx)
            .ok_or(ContractError::InvalidState)?;
        let outcome = dispute::resolve_dispute(&env, &mut grant, &mut d)?;
        Storage::set_grant(&env, grant_id, &grant);

        // Issue #564: forfeit collateral when dispute resolution is against contributor.
        if outcome == DisputeStatus::ResolvedForFunder {
            if let Some(req) = collateral::get_requirement(&env, grant_id) {
                let reason = String::from_str(&env, "dispute lost");
                let _ = collateral::forfeit(
                    &env,
                    grant_id,
                    &grant.owner,
                    req.forfeit_on_dispute_loss_bps,
                    reason,
                );
            }
        }
        metrics::increment(&env, MetricField::DisputesResolved, 1);
        Ok(outcome)
    }

    pub fn dispute_cancel(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        caller: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let mut d = Storage::get_dispute(&env, grant_id, milestone_idx)
            .ok_or(ContractError::InvalidState)?;
        dispute::cancel_dispute(&env, &mut d, &caller)
    }

    pub fn get_dispute_record(env: Env, grant_id: u64, milestone_idx: u32) -> Option<Dispute> {
        Storage::get_dispute(&env, grant_id, milestone_idx)
    }

    // ── Issue #516: Runtime Protocol Configuration Entry Points ──────────────

    pub fn update_config(
        env: Env,
        admin: Address,
        new_config: ProtocolConfig,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        config::set_config(&env, &admin, new_config)
    }

    pub fn get_protocol_config(env: Env) -> ProtocolConfig {
        config::get_config(&env)
    }

    // ── Issue #632: Contributor Verification Entry Points ───────────────────

    pub fn verification_set_verifier(
        env: Env,
        admin: Address,
        verifier: Address,
    ) -> Result<(), ContractError> {
        contributor_verification::set_verifier(&env, &admin, &verifier)
    }

    pub fn verification_attest(
        env: Env,
        verifier: Address,
        subject: Address,
        level: VerificationLevel,
        expires_at: Option<u64>,
        attestation_hash: Bytes,
    ) -> Result<(), ContractError> {
        contributor_verification::attest(
            &env,
            &verifier,
            &subject,
            level,
            expires_at,
            attestation_hash,
        )
    }

    pub fn verification_revoke(
        env: Env,
        caller: Address,
        subject: Address,
    ) -> Result<(), ContractError> {
        contributor_verification::revoke(&env, &caller, &subject)
    }

    pub fn verification_is_verified(
        env: Env,
        address: Address,
        required_level: VerificationLevel,
    ) -> bool {
        contributor_verification::is_verified(&env, &address, required_level)
    }

    pub fn verification_require_verified(
        env: Env,
        address: Address,
        required_level: VerificationLevel,
    ) -> Result<(), ContractError> {
        contributor_verification::require_verified(&env, &address, required_level)
    }

    pub fn verification_get_attestation(
        env: Env,
        address: Address,
    ) -> Option<VerificationAttestation> {
        contributor_verification::get_attestation(&env, &address)
    }

    // ── Issue #517: Protocol Fee Management Entry Points ─────────────────────

    pub fn get_fees_collected(env: Env, token: Address) -> i128 {
        fees::total_fees_collected(&env, &token)
    }

    // ── Issue #631: Revenue Sharing Entry Points ────────────────────────────

    pub fn finalize_epoch(env: Env, caller: Address, epoch_id: u32) -> Result<(), ContractError> {
        revenue_share::finalize_epoch(&env, &caller, epoch_id)
    }

    pub fn claim_revenue_share(
        env: Env,
        staker: Address,
        epoch_id: u32,
    ) -> Result<i128, ContractError> {
        revenue_share::claim(&env, &staker, epoch_id)
    }

    pub fn compute_revenue_claim(env: Env, staker: Address, epoch_id: u32) -> i128 {
        revenue_share::compute_claim(&env, &staker, epoch_id)
    }

    pub fn current_revenue_epoch(env: Env) -> RevenueEpoch {
        revenue_share::current_epoch(&env)
    }

    pub fn unclaimed_revenue_epochs(env: Env, staker: Address) -> Vec<u32> {
        revenue_share::unclaimed_epochs(&env, &staker)
    }

    // ── Issue #529: Escrow Module ─────────────────────────────────────────────

    /// Return the escrow account state for a grant.
    pub fn get_escrow_account(env: Env, grant_id: u64) -> Result<EscrowAccount, ContractError> {
        escrow::get_account(&env, grant_id)
    }

    /// Return the funder ledger for a contributor in a grant.
    pub fn get_funder_ledger(env: Env, grant_id: u64, funder: Address) -> Option<FunderLedger> {
        escrow::get_funder_ledger(&env, grant_id, &funder)
    }

    /// Refund a specific funder's net contribution from escrow after grant ends. Funder only.
    pub fn refund_funder(env: Env, funder: Address, grant_id: u64) -> Result<i128, ContractError> {
        funder.require_auth();
        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if grant.status == GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }
        escrow::refund(&env, grant_id, &funder)
    }

    /// Lock escrow for a grant (e.g., when a dispute is open). Admin only.
    pub fn lock_escrow(env: Env, admin: Address, grant_id: u64) -> Result<(), ContractError> {
        admin.require_auth();
        if Storage::get_global_admin(&env) != Some(admin) {
            return Err(ContractError::Unauthorized);
        }
        escrow::lock(&env, grant_id)
    }

    /// Unlock escrow for a grant after dispute resolution. Admin only.
    pub fn unlock_escrow(env: Env, admin: Address, grant_id: u64) -> Result<(), ContractError> {
        admin.require_auth();
        if Storage::get_global_admin(&env) != Some(admin) {
            return Err(ContractError::Unauthorized);
        }
        escrow::unlock(&env, grant_id)
    }

    /// Expire a stale multisig proposal past its TTL. Anyone can call.
    pub fn expire_multisig_proposal(env: Env, proposal_id: u32) -> Result<(), ContractError> {
        multisig::expire_proposal(&env, proposal_id)
    }

    // ── Issue #530: Multisig Fund Release ─────────────────────────────────────

    /// Create a multisig proposal for a grant action. Grant owner or admin only.
    pub fn create_multisig_proposal(
        env: Env,
        creator: Address,
        grant_id: u64,
        action_payload: Bytes,
        signers: Vec<Address>,
        threshold: u32,
        ttl_ledgers: u32,
    ) -> Result<u32, ContractError> {
        creator.require_auth();
        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        let is_owner = grant.owner == creator;
        let is_admin = Storage::get_global_admin(&env) == Some(creator.clone());
        if !is_owner && !is_admin {
            return Err(ContractError::Unauthorized);
        }
        multisig::create_proposal(
            &env,
            &creator,
            grant_id,
            action_payload,
            signers,
            threshold,
            ttl_ledgers,
        )
    }

    /// Sign (or veto) a multisig proposal.
    pub fn sign_proposal(
        env: Env,
        signer: Address,
        proposal_id: u32,
        approve: bool,
    ) -> Result<u32, ContractError> {
        signer.require_auth();
        multisig::sign(&env, &signer, proposal_id, approve)
    }

    /// Execute a multisig proposal once threshold is met.
    /// For GrantWithdraw proposals, triggers the grant release.
    pub fn execute_multisig_proposal(
        env: Env,
        caller: Address,
        proposal_id: u32,
    ) -> Result<Bytes, ContractError> {
        caller.require_auth();
        let payload = multisig::execute(&env, &caller, proposal_id)?;
        // Dispatch GrantWithdraw if payload encodes a grant_id.
        if let Some(grant_id) = multisig::decode_grant_withdraw(&payload) {
            Self::finalize_grant_release(&env, grant_id)?;
        }
        Ok(payload)
    }

    /// Return a multisig proposal by id.
    pub fn get_multisig_proposal(
        env: Env,
        proposal_id: u32,
    ) -> Result<MultisigProposal, ContractError> {
        multisig::get_proposal(&env, proposal_id)
    }

    /// Helper to encode a GrantWithdraw action payload from a grant_id.
    pub fn encode_grant_withdraw_payload(env: Env, grant_id: u64) -> Bytes {
        multisig::encode_grant_withdraw(&env, grant_id)
    }

    // ── Issue #540: Protocol Metrics ──────────────────────────────────────────

    /// Return the aggregated protocol-wide metrics snapshot.
    pub fn get_protocol_metrics(env: Env) -> ProtocolMetrics {
        metrics::get_metrics(&env)
    }

    /// Return token-specific locked/paid/refunded totals.
    pub fn get_token_metrics(env: Env, token: Address) -> TokenMetric {
        metrics::get_token_metrics(&env, &token)
    }

    /// Reset all protocol metrics. Admin only (for testnet/migration use).
    pub fn reset_metrics(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();
        metrics::reset(&env, &admin)
    }

    // ── Issue #548: KYC/AML Compliance ────────────────────────────────────────

    /// Register the trusted compliance verifier. Admin only.
    pub fn set_compliance_verifier(
        env: Env,
        admin: Address,
        verifier: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        compliance::set_verifier(&env, &admin, &verifier)
    }

    /// Trusted verifier attests compliance for a subject address.
    pub fn attest_compliance(
        env: Env,
        verifier: Address,
        subject: Address,
        status: ComplianceStatus,
        level: ComplianceLevel,
        expires_at: u64,
        jurisdiction: String,
    ) -> Result<(), ContractError> {
        verifier.require_auth();
        compliance::attest(
            &env,
            &verifier,
            &subject,
            status,
            level,
            expires_at,
            jurisdiction,
        )
    }

    /// Revoke a compliance attestation. Verifier or admin only.
    pub fn revoke_compliance(
        env: Env,
        revoker: Address,
        subject: Address,
    ) -> Result<(), ContractError> {
        revoker.require_auth();
        compliance::revoke(&env, &revoker, &subject)
    }

    /// Return the compliance attestation for an address.
    pub fn get_compliance_attestation(env: Env, address: Address) -> Option<ComplianceAttestation> {
        compliance::get_attestation(&env, &address)
    }

    /// Enable compliance requirement for an existing grant. Owner only.
    pub fn set_grant_compliance_level(
        env: Env,
        owner: Address,
        grant_id: u64,
        level: ComplianceLevel,
    ) -> Result<(), ContractError> {
        owner.require_auth();
        let mut grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if grant.owner != owner {
            return Err(ContractError::Unauthorized);
        }
        grant.require_compliance = Some(level as u32);
        Storage::set_grant(&env, grant_id, &grant);
        Ok(())
    }

    // ── Issue #524: Price Oracle Integration ─────────────────────────────────

    /// Configure the on-chain price oracle. Admin only.
    pub fn set_oracle(env: Env, admin: Address, config: OracleConfig) -> Result<(), ContractError> {
        oracle::set_oracle(&env, &admin, config)
    }

    /// Fetch the current oracle price for a token.
    pub fn get_price(env: Env, token: Address) -> Result<PriceQuote, ContractError> {
        oracle::get_price(&env, &token)
    }

    /// Convert an amount between two token denominations using oracle prices.
    pub fn convert_amount(
        env: Env,
        amount: i128,
        from_token: Address,
        to_token: Address,
    ) -> Result<i128, ContractError> {
        oracle::convert_amount(&env, amount, &from_token, &to_token)
    }

    // ── Issue #585: Fee Relayer for Gasless Contributor UX ──────────────────

    /// Configure the relay system. Admin only.
    pub fn relay_set_config(
        env: Env,
        admin: Address,
        config: RelayConfig,
    ) -> Result<(), ContractError> {
        relay::set_relay_config(&env, &admin, config)
    }

    /// Execute a relayed action on behalf of sender.
    pub fn relay_execute(
        env: Env,
        relayer: Address,
        sender: Address,
        action: RelayableAction,
        nonce: u32,
        payload: Bytes,
    ) -> Result<(), ContractError> {
        relay::execute_relayed(&env, &relayer, &sender, action, nonce, payload)
    }

    /// Check if relay is allowed for an address and action.
    pub fn relay_can_relay(env: Env, sender: Address, action: RelayableAction) -> bool {
        relay::can_relay(&env, &sender, &action)
    }

    /// Reimburse the relayer from the treasury.
    pub fn relay_reimburse(env: Env, relayer: Address) -> Result<(), ContractError> {
        relay::reimburse_relayer(&env, &relayer)
    }

    /// Get relay allowance for an address.
    pub fn relay_get_allowance(env: Env, address: Address) -> RelayAllowance {
        relay::get_allowance(&env, &address)
    }

    /// Get current relay config.
    pub fn relay_get_config(env: Env) -> Option<RelayConfig> {
        relay::get_relay_config(&env)
    }

    // ── Issue #567: Decentralized Reviewer Recruitment Marketplace ──────────

    /// Register as a reviewer.
    pub fn reviewer_register(
        env: Env,
        reviewer: Address,
        display_name: String,
        expertise_tags: Vec<String>,
        hourly_rate: Option<i128>,
    ) -> Result<(), ContractError> {
        reviewer_pool::register_reviewer(&env, &reviewer, display_name, expertise_tags, hourly_rate)
    }

    /// Update reviewer availability status.
    pub fn reviewer_set_availability(
        env: Env,
        reviewer: Address,
        availability: ReviewerAvailability,
    ) -> Result<(), ContractError> {
        reviewer_pool::set_availability(&env, &reviewer, availability)
    }

    /// Request a reviewer for a grant.
    pub fn reviewer_request(
        env: Env,
        owner: Address,
        grant_id: u64,
        reviewer: Address,
        message: String,
        ttl_ledgers: u32,
    ) -> Result<(), ContractError> {
        reviewer_pool::request_reviewer(&env, &owner, grant_id, &reviewer, message, ttl_ledgers)
    }

    /// Accept a reviewer request.
    pub fn reviewer_accept_request(
        env: Env,
        reviewer: Address,
        grant_id: u64,
    ) -> Result<(), ContractError> {
        reviewer_pool::accept_request(&env, &reviewer, grant_id)
    }

    /// Decline a reviewer request.
    pub fn reviewer_decline_request(
        env: Env,
        reviewer: Address,
        grant_id: u64,
    ) -> Result<(), ContractError> {
        reviewer_pool::decline_request(&env, &reviewer, grant_id)
    }

    /// Get reviewer profile.
    pub fn reviewer_get_profile(env: Env, reviewer: Address) -> Option<ReviewerProfile> {
        reviewer_pool::get_profile(&env, &reviewer)
    }

    /// Get reviewer request.
    pub fn reviewer_get_request(
        env: Env,
        grant_id: u64,
        reviewer: Address,
    ) -> Option<ReviewerRequest> {
        reviewer_pool::get_request(&env, grant_id, &reviewer)
    }

    // ── Issue #571: Taxonomy, Category, and Tag System for Grants ──────────

    /// Create a new category.
    pub fn tags_create_category(
        env: Env,
        admin: Address,
        name: String,
        subcategories: Vec<String>,
    ) -> Result<u32, ContractError> {
        grant_tags::create_category(&env, &admin, name, subcategories)
    }

    /// Tag a grant.
    pub fn tags_tag_grant(
        env: Env,
        owner: Address,
        grant_id: u64,
        category_id: Option<u32>,
        subcategory: Option<String>,
        freeform_tags: Vec<String>,
    ) -> Result<(), ContractError> {
        grant_tags::tag_grant(
            &env,
            &owner,
            grant_id,
            category_id,
            subcategory,
            freeform_tags,
        )
    }

    /// Update tags on a grant.
    pub fn tags_update_tags(
        env: Env,
        owner: Address,
        grant_id: u64,
        freeform_tags: Vec<String>,
    ) -> Result<(), ContractError> {
        grant_tags::update_tags(&env, &owner, grant_id, freeform_tags)
    }

    /// Get tags for a grant.
    pub fn tags_get_tags(env: Env, grant_id: u64) -> Option<GrantTag> {
        grant_tags::get_tags(&env, grant_id)
    }

    /// Find grants by tag.
    pub fn tags_find_by_tag(env: Env, tag: String, offset: u32, limit: u32) -> Vec<u64> {
        grant_tags::find_by_tag(&env, &tag, offset, limit)
    }

    /// Find grants by category.
    pub fn tags_find_by_category(env: Env, category_id: u32, offset: u32, limit: u32) -> Vec<u64> {
        grant_tags::find_by_category(&env, category_id, offset, limit)
    }

    /// List all categories.
    pub fn tags_list_categories(env: Env) -> Vec<GrantCategory> {
        grant_tags::list_categories(&env)
    }

    /// Remove a tag from a grant.
    pub fn tags_remove_tag(
        env: Env,
        owner: Address,
        grant_id: u64,
        tag: String,
    ) -> Result<(), ContractError> {
        grant_tags::remove_tag(&env, &owner, grant_id, &tag)
    }

    // ── Issue #577: Automatic and Manual Grant Renewal ────────────────────

    /// Propose renewal of a grant.
    pub fn renewal_propose(
        env: Env,
        proposer: Address,
        original_grant_id: u64,
        new_title: String,
        new_description: String,
        new_total_amount: i128,
        new_num_milestones: u32,
        inherit_reviewers: bool,
        inherit_contributor: bool,
        ttl_ledgers: u32,
    ) -> Result<(), ContractError> {
        grant_renewal::propose_renewal(
            &env,
            &proposer,
            original_grant_id,
            new_title,
            new_description,
            new_total_amount,
            new_num_milestones,
            inherit_reviewers,
            inherit_contributor,
            ttl_ledgers,
        )
    }

    /// Approve a renewal proposal.
    pub fn renewal_approve(
        env: Env,
        reviewer: Address,
        original_grant_id: u64,
    ) -> Result<RenewalStatus, ContractError> {
        grant_renewal::approve_renewal(&env, &reviewer, original_grant_id)
    }

    /// Activate an approved renewal.
    pub fn renewal_activate(
        env: Env,
        owner: Address,
        original_grant_id: u64,
    ) -> Result<u64, ContractError> {
        grant_renewal::activate_renewal(&env, &owner, original_grant_id)
    }

    /// Decline a renewal proposal.
    pub fn renewal_decline(
        env: Env,
        caller: Address,
        original_grant_id: u64,
    ) -> Result<(), ContractError> {
        grant_renewal::decline_renewal(&env, &caller, original_grant_id)
    }

    /// Get renewal proposal.
    pub fn renewal_get_proposal(env: Env, original_grant_id: u64) -> Option<RenewalProposal> {
        grant_renewal::get_proposal(&env, original_grant_id)
    }

    /// Get renewal chain.
    pub fn renewal_chain(env: Env, original_grant_id: u64) -> Vec<u64> {
        grant_renewal::renewal_chain(&env, original_grant_id)
    }

    // ── Issue #576: Token Swap Entry Points ────────────────────────────────────

    pub fn set_dex_config(
        env: Env,
        admin: Address,
        config: DexConfig,
    ) -> Result<(), ContractError> {
        token_swap::set_dex_config(&env, &admin, config)
    }

    pub fn get_dex_config(env: Env) -> Result<DexConfig, ContractError> {
        token_swap::get_dex_config(&env)
    }

    pub fn swap_tokens(
        env: Env,
        caller: Address,
        route: SwapRoute,
        amount_in: i128,
    ) -> Result<SwapResult, ContractError> {
        token_swap::swap(&env, &caller, route, amount_in)
    }

    pub fn swap_quote(env: Env, route: SwapRoute, amount_in: i128) -> Result<i128, ContractError> {
        token_swap::quote(&env, &route, amount_in)
    }

    pub fn swap_and_fund(
        env: Env,
        funder: Address,
        grant_id: u64,
        input_token: Address,
        input_amount: i128,
    ) -> Result<SwapResult, ContractError> {
        emergency::require_not_paused(&env)?;
        token_swap::swap_and_fund(&env, &funder, grant_id, &input_token, input_amount)
    }

    pub fn swap_and_pay(
        env: Env,
        grant_id: u64,
        recipient: Address,
        grant_token: Address,
        preferred_token: Address,
        amount: i128,
    ) -> Result<SwapResult, ContractError> {
        emergency::require_not_paused(&env)?;
        token_swap::swap_and_pay(
            &env,
            grant_id,
            &recipient,
            &grant_token,
            &preferred_token,
            amount,
        )
    }

    // ── Issue #581: Milestone Checklist Entry Points ──────────────────────────

    pub fn checklist_define_criteria(
        env: Env,
        owner: Address,
        grant_id: u64,
        milestone_idx: u32,
        criteria: Vec<AcceptanceCriteria>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        checklist::define_criteria(&env, &owner, grant_id, milestone_idx, criteria)
    }

    pub fn checklist_submit(
        env: Env,
        contributor: Address,
        grant_id: u64,
        milestone_idx: u32,
        evidence_urls: Vec<Option<soroban_sdk::String>>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        checklist::submit_checklist(&env, &contributor, grant_id, milestone_idx, evidence_urls)
    }

    pub fn checklist_review_criterion(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        milestone_idx: u32,
        criterion_idx: u32,
        approve: bool,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        checklist::review_criterion(
            &env,
            &reviewer,
            grant_id,
            milestone_idx,
            criterion_idx,
            approve,
        )
    }

    pub fn checklist_all_required_approved(env: Env, grant_id: u64, milestone_idx: u32) -> bool {
        checklist::all_required_approved(&env, grant_id, milestone_idx)
    }

    pub fn checklist_get(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<ChecklistSubmission> {
        checklist::get_checklist(&env, grant_id, milestone_idx)
    }

    pub fn checklist_get_criterion_status(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        criterion_idx: u32,
    ) -> Option<CriterionStatus> {
        checklist::get_criterion_status(&env, grant_id, milestone_idx, criterion_idx)
    }

    // ── Issue #589: Scoring Entry Points ──────────────────────────────────────

    pub fn scoring_define_rubric(
        env: Env,
        admin: Address,
        name: soroban_sdk::String,
        weights: Vec<ScoringWeight>,
    ) -> Result<u32, ContractError> {
        scoring::define_rubric(&env, &admin, name, weights)
    }

    pub fn scoring_score_contributor(
        env: Env,
        contributor: Address,
        rubric_id: u32,
    ) -> Result<ScoreResult, ContractError> {
        scoring::score_contributor(&env, &contributor, rubric_id)
    }

    pub fn scoring_rank_contributors(
        env: Env,
        contributors: Vec<Address>,
        rubric_id: u32,
    ) -> Vec<ScoreResult> {
        scoring::rank_contributors(&env, contributors, rubric_id)
    }

    pub fn scoring_get_rubric(env: Env, rubric_id: u32) -> Result<ScoringRubric, ContractError> {
        scoring::get_rubric(&env, rubric_id)
    }

    pub fn scoring_list_rubrics(env: Env) -> Vec<u32> {
        scoring::list_rubrics(&env)
    }

    // ── Issue #594: Circuit Breaker Entry Points ──────────────────────────────

    pub fn breaker_trip(
        env: Env,
        caller: Address,
        module: ProtocolModule,
        reason: soroban_sdk::String,
        auto_reset_ledger: Option<u32>,
    ) -> Result<(), ContractError> {
        circuit_breaker::trip(&env, &caller, module, reason, auto_reset_ledger)
    }

    pub fn breaker_reset(
        env: Env,
        caller: Address,
        module: ProtocolModule,
    ) -> Result<(), ContractError> {
        circuit_breaker::reset(&env, &caller, module)
    }

    pub fn breaker_is_open(env: Env, module: ProtocolModule) -> bool {
        circuit_breaker::is_open(&env, module)
    }

    pub fn breaker_get_state(env: Env, module: ProtocolModule) -> BreakerState {
        circuit_breaker::get_state(&env, module)
    }

    pub fn breaker_tripped_modules(env: Env) -> Vec<ProtocolModule> {
        circuit_breaker::tripped_modules(&env)
    }

    pub fn breaker_auto_reset_expired(env: Env) -> u32 {
        circuit_breaker::auto_reset_expired(&env)
    }

    // ── Issue #566: Invoice-Style Milestone Billing Entry Points ─────────────

    /// Submit an invoice for a milestone. Contributor only.
    pub fn invoice_submit(
        env: Env,
        contributor: Address,
        grant_id: u64,
        milestone_idx: u32,
        invoice_number: String,
        line_items: Vec<LineItem>,
        tax_bps: u32,
        notes: Option<String>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        invoice::submit_invoice(
            &env,
            &contributor,
            grant_id,
            milestone_idx,
            invoice_number,
            line_items,
            tax_bps,
            notes,
        )
    }

    /// Approve an invoice. Reviewer only. Triggers milestone approval.
    pub fn invoice_approve(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        invoice::approve_invoice(&env, &reviewer, grant_id, milestone_idx)
    }

    /// Reject an invoice with a reason. Reviewer only.
    pub fn invoice_reject(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        milestone_idx: u32,
        reason: String,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        invoice::reject_invoice(&env, &reviewer, grant_id, milestone_idx, reason)
    }

    /// Resubmit a rejected invoice with corrections.
    pub fn invoice_resubmit(
        env: Env,
        contributor: Address,
        grant_id: u64,
        milestone_idx: u32,
        updated_items: Vec<LineItem>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        invoice::resubmit_invoice(&env, &contributor, grant_id, milestone_idx, updated_items)
    }

    /// Return the invoice for a milestone.
    pub fn invoice_get(env: Env, grant_id: u64, milestone_idx: u32) -> Option<Invoice> {
        invoice::get_invoice(&env, grant_id, milestone_idx)
    }

    // ── Issue #582: Advanced Protocol Analytics Entry Points ─────────────────

    /// Record a data point in a rolling window.
    pub fn analytics_record(env: Env, metric: soroban_sdk::Symbol, value: i128) {
        analytics::record(&env, metric, value);
    }

    /// Compute the rolling average for a metric.
    pub fn analytics_rolling_average(env: Env, metric: soroban_sdk::Symbol) -> Option<i128> {
        analytics::rolling_average(&env, metric)
    }

    /// Compute stats for a grant category.
    pub fn analytics_category_stats(env: Env, category_id: u32) -> CategoryStats {
        analytics::category_stats(&env, category_id)
    }

    /// Build and cache the full analytics snapshot.
    pub fn analytics_build_snapshot(env: Env) -> AnalyticsSnapshot {
        analytics::build_snapshot(&env)
    }

    /// Return the latest cached snapshot.
    pub fn analytics_get_snapshot(env: Env) -> Option<AnalyticsSnapshot> {
        analytics::get_snapshot(&env)
    }

    /// Return the raw rolling window for a metric.
    pub fn analytics_get_window(env: Env, metric: soroban_sdk::Symbol) -> Option<RollingWindow> {
        analytics::get_window(&env, metric)
    }

    // ── Issue #596: Dynamic On-Chain Protocol Parameter Store Entry Points ───

    /// Set a parameter directly. Admin only for non-DAO params; DAO vote required for others.
    pub fn param_set(
        env: Env,
        caller: Address,
        key: soroban_sdk::Symbol,
        value: ParamValue,
        description: soroban_sdk::String,
        requires_dao: bool,
    ) -> Result<(), ContractError> {
        params::set_param(&env, &caller, key, value, description, requires_dao)
    }

    /// Get a parameter value by key.
    pub fn param_get(env: Env, key: soroban_sdk::Symbol) -> Option<ParamRecord> {
        params::get_param(&env, &key)
    }

    /// Get a u32 param or return a default value.
    pub fn param_get_u32(env: Env, key: soroban_sdk::Symbol, default: u32) -> u32 {
        params::get_u32(&env, &key, default)
    }

    /// Get an i128 param or return a default value.
    pub fn param_get_i128(env: Env, key: soroban_sdk::Symbol, default: i128) -> i128 {
        params::get_i128(&env, &key, default)
    }

    /// Get a bool param or return a default value.
    pub fn param_get_bool(env: Env, key: soroban_sdk::Symbol, default: bool) -> bool {
        params::get_bool(&env, &key, default)
    }

    /// Return all registered param keys.
    pub fn param_list(env: Env) -> Vec<soroban_sdk::Symbol> {
        params::list_params(&env)
    }

    /// Return the change history for a param (last 20 changes).
    pub fn param_history(env: Env, key: soroban_sdk::Symbol) -> Vec<ParamRecord> {
        params::param_history(&env, &key)
    }

    // ── Issue #593: Role-Based Access Control (RBAC) Entry Points ────────────

    /// Grant a role to an address. SuperAdmin only (or ProtocolAdmin for lesser roles).
    pub fn rbac_grant_role(
        env: Env,
        granter: Address,
        grantee: Address,
        role: Role,
        expires_at: Option<u64>,
    ) -> Result<(), ContractError> {
        access_control::grant_role(&env, &granter, &grantee, role, expires_at)
    }

    /// Revoke a role. SuperAdmin or ProtocolAdmin.
    pub fn rbac_revoke_role(
        env: Env,
        revoker: Address,
        holder: Address,
        role: Role,
    ) -> Result<(), ContractError> {
        access_control::revoke_role(&env, &revoker, &holder, role)
    }

    /// Check if an address holds a specific role (respects expiry).
    pub fn rbac_has_role(env: Env, address: Address, role: Role) -> bool {
        access_control::has_role(&env, &address, role)
    }

    /// Assert that an address holds a role; return Err(Unauthorized) if not.
    pub fn rbac_require_role(env: Env, address: Address, role: Role) -> Result<(), ContractError> {
        access_control::require_role(&env, &address, role)
    }

    /// Assert any of a list of roles (OR logic). Returns Ok if holder has at least one.
    pub fn rbac_require_any_role(
        env: Env,
        address: Address,
        roles: Vec<Role>,
    ) -> Result<(), ContractError> {
        access_control::require_any_role(&env, &address, roles)
    }

    /// Return all addresses holding a specific role.
    pub fn rbac_role_members(env: Env, role: Role) -> Vec<Address> {
        access_control::role_members(&env, role)
    }

    /// Return all roles held by an address.
    pub fn rbac_roles_of(env: Env, address: Address) -> Vec<Role> {
        access_control::roles_of(&env, &address)
    }

    /// Renounce your own role (voluntary self-removal).
    pub fn rbac_renounce_role(env: Env, holder: Address, role: Role) -> Result<(), ContractError> {
        access_control::renounce_role(&env, &holder, role)
    }

    // ── Crowdfund Module ──────────────────────────────────────────────────────

    /// Create a new crowdfunding campaign with a funding target and deadline.
    /// If the target is met by the deadline the owner receives the tokens;
    /// otherwise backers can reclaim their pledges via `crowdfund_refund`.
    #[allow(clippy::too_many_arguments)]
    pub fn crowdfund_create(
        env: Env,
        owner: Address,
        title: String,
        description: String,
        token: Address,
        target_amount: i128,
        deadline_ledgers: u32,
    ) -> Result<u64, ContractError> {
        owner.require_auth();
        crowdfund::create_campaign(
            &env,
            &owner,
            title,
            description,
            &token,
            target_amount,
            deadline_ledgers,
        )
    }

    /// Pledge tokens to an active campaign. Caller must pre-approve the
    /// contract to transfer `amount` tokens.
    pub fn crowdfund_pledge(
        env: Env,
        campaign_id: u64,
        backer: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        backer.require_auth();
        crowdfund::pledge(&env, campaign_id, &backer, amount)
    }

    /// Finalize a campaign once its deadline has passed. Anyone may call this.
    /// Returns the resulting `CrowdfundStatus` (Succeeded or Failed).
    pub fn crowdfund_finalize(
        env: Env,
        campaign_id: u64,
    ) -> Result<CrowdfundStatus, ContractError> {
        crowdfund::finalize(&env, campaign_id)
    }

    /// Claim a pledge refund after a Failed or Cancelled campaign.
    pub fn crowdfund_refund(
        env: Env,
        campaign_id: u64,
        backer: Address,
    ) -> Result<(), ContractError> {
        backer.require_auth();
        crowdfund::claim_refund(&env, campaign_id, &backer)
    }

    /// Cancel an active campaign. Only the campaign owner may call this.
    pub fn crowdfund_cancel(
        env: Env,
        campaign_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        crowdfund::cancel(&env, campaign_id, &caller)
    }

    /// Fetch campaign details by id.
    pub fn crowdfund_get_campaign(env: Env, campaign_id: u64) -> Option<CrowdfundCampaign> {
        crowdfund::get_campaign(&env, campaign_id)
    }

    /// Fetch a specific backer's pledge for a campaign.
    pub fn crowdfund_get_pledge(
        env: Env,
        campaign_id: u64,
        backer: Address,
    ) -> Option<CrowdfundPledge> {
        crowdfund::get_pledge(&env, campaign_id, &backer)
    }

    /// List all backer addresses for a campaign.
    pub fn crowdfund_list_backers(env: Env, campaign_id: u64) -> Vec<Address> {
        crowdfund::list_backers(&env, campaign_id)
    }

    // ── Portfolio (#565) ──────────────────────────────────────────────────────

    pub fn get_portfolio(
        env: Env,
        contributor: Address,
    ) -> Result<ContributorPortfolio, ContractError> {
        portfolio::get_portfolio(&env, &contributor)
    }

    pub fn portfolio_grant_summary(
        env: Env,
        contributor: Address,
        grant_id: u64,
    ) -> Result<GrantSummary, ContractError> {
        portfolio::get_grant_summary(&env, &contributor, grant_id)
    }

    pub fn portfolio_earnings_by_token(env: Env, contributor: Address) -> Vec<(Address, i128)> {
        portfolio::earnings_by_token(&env, &contributor)
    }

    pub fn portfolio_hash(env: Env, contributor: Address) -> Bytes {
        portfolio::portfolio_hash(&env, &contributor)
    }

    // ── Milestone NFT (#570) ──────────────────────────────────────────────────

    pub fn nft_get(env: Env, grant_id: u64, milestone_idx: u32) -> Option<MilestoneNft> {
        milestone_nft::get_nft(&env, grant_id, milestone_idx)
    }

    pub fn nft_get_by_token_id(env: Env, token_id: u32) -> Option<MilestoneNft> {
        milestone_nft::get_by_token_id(&env, token_id)
    }

    pub fn nft_get_by_owner(env: Env, owner: Address) -> Vec<u32> {
        milestone_nft::get_by_owner(&env, &owner)
    }

    pub fn nft_verify(env: Env, token_id: u32) -> bool {
        milestone_nft::verify_nft(&env, token_id)
    }

    pub fn nft_set_transferable(
        env: Env,
        admin: Address,
        token_id: u32,
        transferable: bool,
    ) -> Result<(), ContractError> {
        milestone_nft::set_transferable(&env, &admin, token_id, transferable)
    }

    pub fn nft_transfer(
        env: Env,
        from: Address,
        to: Address,
        token_id: u32,
    ) -> Result<(), ContractError> {
        milestone_nft::transfer(&env, &from, &to, token_id)
    }

    // ── Open Review (#590) ────────────────────────────────────────────────────

    pub fn open_review_submit(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        milestone_idx: u32,
        signal: PublicReviewSignal,
        comment: String,
    ) -> Result<(), ContractError> {
        open_review::submit_review(&env, &reviewer, grant_id, milestone_idx, signal, comment)
    }

    pub fn open_review_mark_helpful(
        env: Env,
        voter: Address,
        grant_id: u64,
        milestone_idx: u32,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        open_review::mark_helpful(&env, &voter, grant_id, milestone_idx, &reviewer)
    }

    pub fn open_review_get_reviews(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Vec<PublicReview> {
        open_review::get_reviews(&env, grant_id, milestone_idx)
    }

    pub fn open_review_aggregate_signals(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> (u32, u32, u32) {
        open_review::aggregate_signals(&env, grant_id, milestone_idx)
    }

    pub fn open_review_get_review(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<PublicReview> {
        open_review::get_review(&env, &reviewer, grant_id, milestone_idx)
    }

    pub fn open_review_has_reviewed(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> bool {
        open_review::has_reviewed(&env, &reviewer, grant_id, milestone_idx)
    }

    // ── Milestone DAG (#595) ──────────────────────────────────────────────────

    pub fn milestone_deps_attach_dag(
        env: Env,
        owner: Address,
        grant_id: u64,
        deps: Vec<MilestoneDependency>,
    ) -> Result<(), ContractError> {
        milestone_deps::attach_dag(&env, &owner, grant_id, deps)
    }

    pub fn milestone_deps_can_submit(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<(), ContractError> {
        milestone_deps::can_submit(&env, grant_id, milestone_idx)
    }

    pub fn deps_unblocked_milestones(env: Env, grant_id: u64) -> Vec<u32> {
        milestone_deps::unblocked_milestones(&env, grant_id)
    }

    pub fn milestone_deps_dependents_of(env: Env, grant_id: u64, idx: u32) -> Vec<u32> {
        milestone_deps::dependents_of(&env, grant_id, idx)
    }

    pub fn milestone_deps_get_dag(env: Env, grant_id: u64) -> Option<MilestoneDag> {
        milestone_deps::get_dag(&env, grant_id)
    }

    pub fn milestone_deps_topological_order(
        env: Env,
        deps: Vec<MilestoneDependency>,
        total: u32,
    ) -> Result<Vec<u32>, ContractError> {
        milestone_deps::topological_order(&env, &deps, total)
    }

    // ── Issue #597: Grant Index Entry Points ──────────────────────────────────

    pub fn index_by_owner(env: Env, owner: Address, offset: u32, limit: u32) -> Vec<u64> {
        grant_index::by_owner(&env, &owner, offset, limit)
    }

    pub fn index_by_status(env: Env, status: GrantStatus, offset: u32, limit: u32) -> Vec<u64> {
        grant_index::by_status(&env, status, offset, limit)
    }

    pub fn index_by_token(env: Env, token: Address, offset: u32, limit: u32) -> Vec<u64> {
        grant_index::by_token(&env, &token, offset, limit)
    }

    pub fn index_by_contributor(
        env: Env,
        contributor: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<u64> {
        grant_index::by_contributor(&env, &contributor, offset, limit)
    }

    pub fn index_recent(env: Env, offset: u32, limit: u32) -> Vec<u64> {
        grant_index::recent(&env, offset, limit)
    }

    pub fn index_counts(env: Env, owner: Option<Address>) -> (u32, u32, u32) {
        grant_index::index_counts(&env, owner.as_ref())
    }

    // ── Issue #587: Grant Forking Entry Points ────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn fork_grant(
        env: Env,
        caller: Address,
        original_grant_id: u64,
        new_title: String,
        new_description: String,
        new_total_amount: i128,
        new_token: Address,
        inherit_reviewers: bool,
        inherit_milestones: bool,
    ) -> Result<u64, ContractError> {
        caller.require_auth();
        fork::fork_grant(
            &env,
            &caller,
            original_grant_id,
            new_title,
            new_description,
            new_total_amount,
            &new_token,
            inherit_reviewers,
            inherit_milestones,
        )
    }

    pub fn get_fork_record(env: Env, grant_id: u64) -> Option<ForkRecord> {
        fork::get_fork_record(&env, grant_id)
    }

    pub fn get_forks(env: Env, original_grant_id: u64) -> Vec<u64> {
        fork::get_forks(&env, original_grant_id)
    }

    pub fn fork_depth(env: Env, grant_id: u64) -> u32 {
        fork::fork_depth(&env, grant_id)
    }

    pub fn is_descendant(env: Env, ancestor_id: u64, descendant_id: u64) -> bool {
        fork::is_descendant(&env, ancestor_id, descendant_id)
    }

    // ── Issue #580: Notification Subscription Entry Points ────────────────────

    pub fn subscribe(
        env: Env,
        subscriber: Address,
        event: NotificationEvent,
        scope: SubscriptionScope,
    ) -> Result<(), ContractError> {
        subscriber.require_auth();
        notification::subscribe(&env, &subscriber, event, scope)
    }

    pub fn unsubscribe(
        env: Env,
        subscriber: Address,
        event: NotificationEvent,
        scope: SubscriptionScope,
    ) -> Result<(), ContractError> {
        subscriber.require_auth();
        notification::unsubscribe(&env, &subscriber, event, &scope)
    }

    pub fn get_subscriptions(env: Env, subscriber: Address) -> Vec<Subscription> {
        notification::get_subscriptions(&env, &subscriber)
    }

    pub fn get_subscribers(
        env: Env,
        event: NotificationEvent,
        scope: SubscriptionScope,
    ) -> Vec<Address> {
        notification::get_subscribers(&env, event, &scope)
    }

    pub fn is_subscribed(
        env: Env,
        subscriber: Address,
        event: NotificationEvent,
        scope: SubscriptionScope,
    ) -> bool {
        notification::is_subscribed(&env, &subscriber, &event, &scope)
    }

    // ── Private Helpers ───────────────────────────────────────────────────────

    fn update_contributor_reputation(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        contributor: &Address,
        payout_amount: i128,
    ) {
        if Storage::has_milestone_reputation_applied(env, grant_id, milestone_idx) {
            return;
        }
        Storage::mark_milestone_reputation_applied(env, grant_id, milestone_idx);
        let mut profile = match Storage::get_contributor(env, contributor.clone()) {
            Some(p) => p,
            None => return,
        };
        let _ = reputation::record_completion(
            env,
            grant_id,
            milestone_idx,
            &mut profile,
            payout_amount,
        );
    }

    // ── Issue #579: IP License Tracking ──────────────────────────────────────

    /// Attach an IP license record to a milestone deliverable.
    /// Only the grant owner may call this after the milestone exists.
    #[allow(clippy::too_many_arguments)]
    pub fn attach_license(
        env: Env,
        caller: Address,
        grant_id: u64,
        milestone_idx: u32,
        spdx_id: String,
        license_type: LicenseType,
        rights: IpRights,
        restrictions: String,
    ) -> Result<LicenseRecord, ContractError> {
        emergency::require_not_paused(&env)?;
        license::attach_license(
            &env,
            &caller,
            grant_id,
            milestone_idx,
            spdx_id,
            license_type,
            rights,
            restrictions,
        )
    }

    /// Return the license record for a milestone deliverable, if any.
    pub fn get_license(env: Env, grant_id: u64, milestone_idx: u32) -> Option<LicenseRecord> {
        license::get_license(&env, grant_id, milestone_idx)
    }

    // ── Issue #592: Multi-Recipient Payment Splitting ─────────────────────────

    /// Register a payment split for a milestone.
    /// `recipients` share_bps values must sum to 10 000.
    pub fn register_payment_split(
        env: Env,
        caller: Address,
        grant_id: u64,
        milestone_idx: u32,
        recipients: Vec<SplitRecipient>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        split_payment::register_split(&env, &caller, grant_id, milestone_idx, recipients)
    }

    /// Return the registered payment split for a milestone, if any.
    pub fn get_payment_split(env: Env, grant_id: u64, milestone_idx: u32) -> Option<PaymentSplit> {
        split_payment::get_split(&env, grant_id, milestone_idx)
    }

    // ── Issue #578: Cross-Protocol Grant Syndication ─────────────────────────

    pub fn form_syndicate(
        env: Env,
        lead: Address,
        grant_id: u64,
        target_total: i128,
        min_commitment: i128,
        max_members: u32,
        deadline_ledgers: u32,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        lead.require_auth();
        syndication::form_syndicate(
            &env,
            &lead,
            grant_id,
            target_total,
            min_commitment,
            max_members,
            deadline_ledgers,
        )
    }

    pub fn join_syndicate(
        env: Env,
        member: Address,
        grant_id: u64,
        amount: i128,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        member.require_auth();
        syndication::join_syndicate(&env, &member, grant_id, amount)
    }

    pub fn close_syndicate(env: Env, lead: Address, grant_id: u64) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        lead.require_auth();
        syndication::close_syndicate(&env, &lead, grant_id)
    }

    pub fn record_payout_allocation(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        payout: i128,
    ) -> Result<(), ContractError> {
        syndication::record_payout_allocation(&env, grant_id, milestone_idx, payout)
    }

    pub fn withdraw_syndicate(
        env: Env,
        member: Address,
        grant_id: u64,
    ) -> Result<i128, ContractError> {
        member.require_auth();
        syndication::withdraw_syndicate(&env, &member, grant_id)
    }

    pub fn get_syndicate_member(
        env: Env,
        grant_id: u64,
        member: Address,
    ) -> Option<SyndicateMember> {
        syndication::get_member(&env, grant_id, &member)
    }

    pub fn get_syndicate_members(env: Env, grant_id: u64) -> Vec<SyndicateMember> {
        syndication::get_members(&env, grant_id)
    }

    pub fn get_syndicate(env: Env, grant_id: u64) -> Option<SyndicateGrant> {
        syndication::get_syndicate(&env, grant_id)
    }

    // ── Issue #591: Grant Specification Versioning ───────────────────────────

    pub fn propose_amendment(
        env: Env,
        owner: Address,
        grant_id: u64,
        changed_fields: Vec<String>,
        new_values: Vec<String>,
        rationale: String,
    ) -> Result<u32, ContractError> {
        emergency::require_not_paused(&env)?;
        owner.require_auth();
        versioning::propose_amendment(
            &env,
            &owner,
            grant_id,
            changed_fields,
            new_values,
            rationale,
        )
    }

    pub fn vote_amendment(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        amendment_version: u32,
        approve: bool,
    ) -> Result<AmendmentStatus, ContractError> {
        reviewer.require_auth();
        versioning::vote_amendment(&env, &reviewer, grant_id, amendment_version, approve)
    }

    pub fn apply_amendment(
        env: Env,
        grant_id: u64,
        amendment_version: u32,
    ) -> Result<GrantVersion, ContractError> {
        versioning::apply_amendment(&env, grant_id, amendment_version)
    }

    pub fn get_version(env: Env, grant_id: u64, version: u32) -> Option<GrantVersion> {
        versioning::get_version(&env, grant_id, version)
    }

    pub fn current_version(env: Env, grant_id: u64) -> u32 {
        versioning::current_version(&env, grant_id)
    }

    pub fn amendment_history(env: Env, grant_id: u64) -> Vec<Amendment> {
        versioning::amendment_history(&env, grant_id)
    }

    // ── Issue #568: Grant Ownership and Role Transfer ─────────────────────────

    /// Propose transferring grant ownership or a reviewer role to a new address.
    /// The new holder must call `accept_grant_transfer` to complete the handoff.
    pub fn propose_grant_transfer(
        env: Env,
        caller: Address,
        grant_id: u64,
        new_holder: Address,
        role: TransferableRole,
        reviewer_to_replace: Option<Address>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        grant_transfer::propose_transfer(
            &env,
            &caller,
            grant_id,
            new_holder,
            role,
            reviewer_to_replace,
        )
    }

    /// Accept a pending transfer proposal. Caller must be the proposed new holder.
    pub fn accept_grant_transfer(
        env: Env,
        new_holder: Address,
        grant_id: u64,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        grant_transfer::accept_transfer(&env, &new_holder, grant_id)
    }

    /// Return the pending transfer proposal for a grant, if any.
    pub fn get_transfer_proposal(env: Env, grant_id: u64) -> Option<TransferProposal> {
        grant_transfer::get_transfer_proposal(&env, grant_id)
    }

    // ── Issue #583: Typed Evidence Schemas ───────────────────────────────────

    /// Define a typed evidence schema for a milestone.
    /// Contributors must submit conforming structured evidence before `milestone_submit`.
    pub fn set_evidence_schema(
        env: Env,
        caller: Address,
        grant_id: u64,
        milestone_idx: u32,
        fields: Vec<EvidenceField>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        evidence_schema::set_schema(&env, &caller, grant_id, milestone_idx, fields)
    }

    /// Submit structured evidence for a milestone, conforming to the registered schema.
    pub fn submit_structured_evidence(
        env: Env,
        caller: Address,
        grant_id: u64,
        milestone_idx: u32,
        values: Map<String, String>,
    ) -> Result<(), ContractError> {
        emergency::require_not_paused(&env)?;
        evidence_schema::submit_evidence(&env, &caller, grant_id, milestone_idx, values)
    }

    /// Return the evidence schema for a milestone, if any.
    pub fn get_evidence_schema(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<EvidenceSchema> {
        evidence_schema::get_schema(&env, grant_id, milestone_idx)
    }

    /// Return the submitted structured evidence for a milestone, if any.
    pub fn get_structured_evidence(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<StructuredEvidence> {
        evidence_schema::get_evidence(&env, grant_id, milestone_idx)
    }

    // ── Issue #569: Referral and Growth Incentive System ─────────────────────

    /// Create a referral code. Any registered contributor or reviewer.
    pub fn referral_create_code(
        env: Env,
        referrer: Address,
        expires_at: Option<u64>,
        max_uses: Option<u32>,
    ) -> Result<Bytes, ContractError> {
        referral::create_code(&env, &referrer, expires_at, max_uses)
    }

    /// Apply a referral code when a new contributor/funder joins.
    pub fn referral_apply_code(
        env: Env,
        referred: Address,
        code_hash: Bytes,
    ) -> Result<(), ContractError> {
        referral::apply_code(&env, &referred, &code_hash)
    }

    /// Referrer claims accumulated referral rewards for a token.
    pub fn referral_claim_rewards(
        env: Env,
        referrer: Address,
        token: Address,
    ) -> Result<i128, ContractError> {
        referral::claim_rewards(&env, &referrer, &token)
    }

    /// Return total unclaimed referral rewards for a referrer and token.
    pub fn referral_pending_rewards(env: Env, referrer: Address, token: Address) -> i128 {
        referral::pending_rewards(&env, &referrer, &token)
    }

    /// Return the referral record for a referred address.
    pub fn referral_get_record(env: Env, referred: Address) -> Option<ReferralRecord> {
        referral::get_record(&env, &referred)
    }

    /// Deactivate a referral code. Creator or admin only.
    pub fn referral_deactivate_code(
        env: Env,
        caller: Address,
        code_hash: Bytes,
    ) -> Result<(), ContractError> {
        referral::deactivate_code(&env, &caller, &code_hash)
    }

    // ── Issue #572: Deadline Extension Request Workflow ──────────────────────

    /// Contributor requests a deadline extension for a milestone.
    pub fn request_extension(
        env: Env,
        contributor: Address,
        grant_id: u64,
        milestone_idx: u32,
        new_deadline: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        milestone_extension::request_extension(
            &env,
            &contributor,
            grant_id,
            milestone_idx,
            new_deadline,
            reason,
        )
    }

    /// Reviewer votes on a pending extension request.
    pub fn vote_extension(
        env: Env,
        reviewer: Address,
        grant_id: u64,
        milestone_idx: u32,
        approve: bool,
    ) -> Result<ExtensionStatus, ContractError> {
        milestone_extension::vote_extension(&env, &reviewer, grant_id, milestone_idx, approve)
    }

    /// Contributor withdraws a pending extension request.
    pub fn withdraw_extension(
        env: Env,
        contributor: Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<(), ContractError> {
        milestone_extension::withdraw_request(&env, &contributor, grant_id, milestone_idx)
    }

    /// Return the current extension request for a milestone.
    pub fn get_extension_request(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<ExtensionRequest> {
        milestone_extension::get_request(&env, grant_id, milestone_idx)
    }

    /// Return all resolved extension requests for a grant.
    pub fn get_extension_history(env: Env, grant_id: u64) -> Vec<ExtensionRequest> {
        milestone_extension::get_extension_history(&env, grant_id)
    }

    // ── Issue #573: Community Arbitration Pool ───────────────────────────────

    /// Join the arbitration pool by staking tokens.
    pub fn arbiter_join_pool(
        env: Env,
        arbiter: Address,
        token: Address,
        stake: i128,
    ) -> Result<(), ContractError> {
        arbitration_pool::join_pool(&env, &arbiter, &token, stake)
    }

    /// Leave the arbitration pool and withdraw stake.
    pub fn arbiter_leave_pool(env: Env, arbiter: Address) -> Result<i128, ContractError> {
        arbitration_pool::leave_pool(&env, &arbiter)
    }

    /// Enable community arbitration for an open dispute by assigning a randomized
    /// panel from the pool. Admin-gated. Returns the arbitration case id.
    pub fn assign_arbitration_panel(
        env: Env,
        admin: Address,
        grant_id: u64,
        milestone_idx: u32,
        dispute_id: u32,
        panel_size: u32,
    ) -> Result<u32, ContractError> {
        admin.require_auth();
        if Storage::get_global_admin(&env) != Some(admin.clone()) {
            return Err(ContractError::Unauthorized);
        }
        let mut d = Storage::get_dispute(&env, grant_id, milestone_idx)
            .ok_or(ContractError::InvalidState)?;
        dispute::assign_pool_panel(&env, &mut d, dispute_id, panel_size)
    }

    /// Arbiter casts a vote on an arbitration case.
    pub fn cast_arbiter_vote(
        env: Env,
        arbiter: Address,
        case_id: u32,
        favor_contributor: bool,
        confidence: u32,
    ) -> Result<(), ContractError> {
        arbitration_pool::cast_arbiter_vote(&env, &arbiter, case_id, favor_contributor, confidence)
    }

    /// Finalize an arbitration case once voting closes.
    pub fn finalize_arbitration_case(env: Env, case_id: u32) -> Result<bool, ContractError> {
        arbitration_pool::finalize_case(&env, case_id)
    }

    /// Distribute rewards and slash minority arbiters for a finalized case.
    pub fn settle_arbitration_rewards(env: Env, case_id: u32) -> Result<(), ContractError> {
        arbitration_pool::settle_rewards(&env, case_id)
    }

    /// Return pool statistics: (active_arbiters, total_staked).
    pub fn arbitration_pool_stats(env: Env) -> (u32, i128) {
        arbitration_pool::pool_stats(&env)
    }

    /// Return an arbiter's profile.
    pub fn get_arbiter(env: Env, address: Address) -> Option<Arbiter> {
        arbitration_pool::get_arbiter(&env, &address)
    }

    // ── Issue #574: Surety Bonds for High-Value Grant Delivery ───────────────

    /// Grant owner requires a performance bond for their grant.
    pub fn require_bond(
        env: Env,
        owner: Address,
        grant_id: u64,
        token: Address,
        amount: i128,
        ttl_ledgers: u32,
    ) -> Result<u32, ContractError> {
        performance_bond::require_bond(&env, &owner, grant_id, &token, amount, ttl_ledgers)
    }

    /// Guarantor posts a required bond by depositing the bond amount.
    pub fn post_bond(env: Env, guarantor: Address, bond_id: u32) -> Result<(), ContractError> {
        performance_bond::post_bond(&env, &guarantor, bond_id)
    }

    /// Funder claims a bond payout after contributor default.
    pub fn claim_bond(
        env: Env,
        funder: Address,
        grant_id: u64,
        reason: String,
    ) -> Result<BondClaim, ContractError> {
        performance_bond::claim_bond(&env, &funder, grant_id, reason)
    }

    /// Return the performance bond for a grant.
    pub fn get_bond(env: Env, grant_id: u64) -> Option<PerformanceBond> {
        performance_bond::get_bond(&env, grant_id)
    }

    /// Check if a grant has an active (posted) bond.
    pub fn has_active_bond(env: Env, grant_id: u64) -> bool {
        performance_bond::has_active_bond(&env, grant_id)
    }

    // ── Issue #564: Collateral Escrow Entry Points ───────────────────────────

    /// Set collateral requirement for a grant. Owner only, before work starts.
    pub fn collateral_set_requirement(
        env: Env,
        owner: Address,
        grant_id: u64,
        req: CollateralRequirement,
    ) -> Result<(), ContractError> {
        collateral::set_requirement(&env, &owner, grant_id, &req)
    }

    /// Contributor deposits required collateral to begin work.
    pub fn collateral_deposit(
        env: Env,
        contributor: Address,
        grant_id: u64,
    ) -> Result<(), ContractError> {
        collateral::deposit(&env, &contributor, grant_id)
    }

    /// Release collateral back to contributor on grant completion.
    pub fn collateral_release(
        env: Env,
        grant_id: u64,
        contributor: Address,
    ) -> Result<i128, ContractError> {
        collateral::release(&env, grant_id, &contributor)
    }

    /// Forfeit a portion of collateral (called by dispute or abandon logic).
    pub fn collateral_forfeit(
        env: Env,
        grant_id: u64,
        contributor: Address,
        forfeit_bps: u32,
        reason: String,
    ) -> Result<i128, ContractError> {
        collateral::forfeit(&env, grant_id, &contributor, forfeit_bps, reason)
    }

    /// Return collateral deposit for a contributor.
    pub fn collateral_get_deposit(
        env: Env,
        grant_id: u64,
        contributor: Address,
    ) -> Option<CollateralDeposit> {
        collateral::get_deposit(&env, grant_id, &contributor)
    }

    /// Return the collateral requirement for a grant.
    pub fn collateral_get_requirement(env: Env, grant_id: u64) -> Option<CollateralRequirement> {
        collateral::get_requirement(&env, grant_id)
    }

    // ── Issue #609: Token Lockup Entry Points ─────────────────────────────────

    /// Attach a lockup to a milestone payout. Owner sets before first submission.
    pub fn lockup_attach(
        env: Env,
        owner: Address,
        grant_id: u64,
        milestone_idx: u32,
        lockup_duration_seconds: u64,
    ) -> Result<(), ContractError> {
        lockup::attach_lockup(
            &env,
            &owner,
            grant_id,
            milestone_idx,
            lockup_duration_seconds,
        )
    }

    /// Lock funds on milestone approval. Called internally by payout path.
    pub fn lockup_lock_payout(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        holder: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        lockup::lock_payout(&env, grant_id, milestone_idx, &holder, &token, amount)
    }

    /// Contributor releases their own lockup after unlock time.
    pub fn lockup_release(
        env: Env,
        holder: Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<i128, ContractError> {
        lockup::release(&env, &holder, grant_id, milestone_idx)
    }

    /// Return whether a lockup has expired and is claimable.
    pub fn lockup_is_unlocked(env: Env, grant_id: u64, milestone_idx: u32) -> bool {
        lockup::is_unlocked(&env, grant_id, milestone_idx)
    }

    /// Return the lockup record for a milestone.
    pub fn lockup_get(env: Env, grant_id: u64, milestone_idx: u32) -> Option<LockupRecord> {
        lockup::get_lockup(&env, grant_id, milestone_idx)
    }

    /// Admin revoke: return funds to escrow (fraud/dispute resolution).
    pub fn lockup_revoke(
        env: Env,
        admin: Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<(), ContractError> {
        lockup::revoke(&env, &admin, grant_id, milestone_idx)
    }

    // ── Issue #619: Structured Data Export Entry Points ──────────────────────

    /// Export a paginated list of grants, optionally filtered by last_updated_after timestamp.
    pub fn export_grants(
        env: Env,
        offset: u32,
        limit: u32,
        last_updated_after: Option<u64>,
    ) -> ExportGrantPage {
        data_export::export_grants(&env, offset, limit, last_updated_after)
    }

    /// Export milestones for a specific grant.
    pub fn export_milestones(env: Env, grant_id: u64) -> soroban_sdk::Vec<ExportMilestone> {
        data_export::export_milestones(&env, grant_id)
    }

    /// Export all milestones updated after a timestamp (cross-grant, paginated).
    pub fn export_milestones_since(
        env: Env,
        since: u64,
        offset: u32,
        limit: u32,
    ) -> ExportMilestonePage {
        data_export::export_milestones_since(&env, since, offset, limit)
    }

    /// Return the last-updated timestamp for the entire dataset (for cache invalidation).
    pub fn last_global_update(env: Env) -> u64 {
        data_export::last_global_update(&env)
    }

    /// Return a compact state hash for the protocol (fingerprint for change detection).
    pub fn state_fingerprint(env: Env) -> soroban_sdk::BytesN<32> {
        data_export::state_fingerprint(&env)
    }

    // ── Issue #598: Funder Report Entry Points ───────────────────────────────

    /// Build a comprehensive financial report for a funder. Read-only.
    pub fn get_funder_report(env: Env, funder: Address) -> Result<FunderReport, ContractError> {
        funder_report::get_report(&env, &funder)
    }

    /// Return per-token financial summary for a funder.
    pub fn funder_token_summary(env: Env, funder: Address, token: Address) -> FunderTokenSummary {
        funder_report::token_summary(&env, &funder, &token)
    }

    /// Return summaries for all grants funded by an address.
    pub fn funder_grant_summaries(
        env: Env,
        funder: Address,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<FunderGrantSummary>, ContractError> {
        funder_report::grant_summaries(&env, &funder, offset, limit)
    }

    /// Return total amount currently in escrow across all grants for a funder (per token).
    pub fn funder_total_in_escrow(env: Env, funder: Address, token: Address) -> i128 {
        funder_report::total_in_escrow(&env, &funder, &token)
    }

    /// Return a lightweight report suitable for a dashboard widget.
    /// Returns: (grants_count, total_committed, total_in_escrow, total_paid_out)
    pub fn funder_dashboard_summary(env: Env, funder: Address) -> (u32, i128, i128, i128) {
        funder_report::dashboard_summary(&env, &funder)
    }

    // ── Issue #512: Whitelist Entry Points ────────────────────────────────────

    /// Add an address to a whitelist scope. Admin or grant owner only.
    pub fn whitelist_add(
        env: Env,
        caller: Address,
        address: Address,
        scope: WhitelistScope,
    ) -> Result<(), ContractError> {
        whitelist::add(&env, &caller, &address, &scope)
    }

    /// Remove an address from a whitelist scope. Admin or grant owner only.
    pub fn whitelist_remove(
        env: Env,
        caller: Address,
        address: Address,
        scope: WhitelistScope,
    ) -> Result<(), ContractError> {
        whitelist::remove(&env, &caller, &address, &scope)
    }

    /// Check if an address is on the whitelist for a scope.
    /// If mode is Open, always returns true.
    pub fn whitelist_is_allowed(env: Env, address: Address, scope: WhitelistScope) -> bool {
        whitelist::is_allowed(&env, &address, &scope)
    }

    /// Set the operating mode for a scope (Open or Restricted). Admin only.
    pub fn whitelist_set_mode(
        env: Env,
        admin: Address,
        scope: WhitelistScope,
        mode: WhitelistMode,
    ) -> Result<(), ContractError> {
        whitelist::set_mode(&env, &admin, &scope, mode)
    }

    /// Return the current mode for a scope.
    pub fn whitelist_get_mode(env: Env, scope: WhitelistScope) -> WhitelistMode {
        whitelist::get_mode(&env, &scope)
    }

    /// Return all entries in a whitelist scope.
    pub fn whitelist_get_entries(env: Env, scope: WhitelistScope) -> Vec<WhitelistEntry> {
        whitelist::get_entries(&env, &scope)
    }

    // ── Issue #622: Batched Multi-Key Storage Reads ──────────────────────

    /// Return all data needed for the grant detail page. Single RPC call.
    pub fn batch_grant_detail(env: Env, grant_id: u64) -> Result<GrantDetailView, ContractError> {
        batch_read::grant_detail(&env, grant_id)
    }

    /// Return all data needed for the protocol dashboard. Single RPC call.
    pub fn batch_dashboard(env: Env) -> DashboardView {
        batch_read::dashboard(&env)
    }

    /// Return all data needed for a reviewer's personal dashboard.
    pub fn batch_reviewer_dashboard(env: Env, reviewer: Address) -> ReviewerView {
        batch_read::reviewer_dashboard(&env, &reviewer)
    }

    /// Return detail views for multiple grants at once (max 10).
    pub fn batch_multi_grant_detail(
        env: Env,
        grant_ids: Vec<u64>,
    ) -> Result<Vec<GrantDetailView>, ContractError> {
        batch_read::multi_grant_detail(&env, grant_ids)
    }

    /// Return minimal grant cards for a list of grant IDs (cheaper than full detail).
    pub fn batch_grant_cards(env: Env, grant_ids: Vec<u64>) -> Vec<ExportGrant> {
        batch_read::grant_cards(&env, grant_ids)
    }

    // ── Issue #613: Condition-Based Milestone Fund Release ───────────────

    /// Attach release conditions to a milestone. Owner only, before submission.
    pub fn conditional_attach_conditions(
        env: Env,
        owner: Address,
        grant_id: u64,
        milestone_idx: u32,
        conditions: Vec<ReleaseCondition>,
    ) -> Result<(), ContractError> {
        conditional_release::attach_conditions(&env, &owner, grant_id, milestone_idx, conditions)
    }

    /// Check all conditions for a milestone. Returns detailed results per condition.
    pub fn conditional_check_conditions(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Vec<ConditionResult> {
        conditional_release::check_conditions(&env, grant_id, milestone_idx)
    }

    /// Return true only if every condition is met.
    pub fn conditional_all_met(env: Env, grant_id: u64, milestone_idx: u32) -> bool {
        conditional_release::all_conditions_met(&env, grant_id, milestone_idx)
    }

    /// Return the conditions attached to a milestone.
    pub fn conditional_get_conditions(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Vec<ReleaseCondition> {
        conditional_release::get_conditions(&env, grant_id, milestone_idx)
    }

    // ── Issue #612: Automatic Milestone Approval After Reviewer Timeout ──

    /// Configure auto-approve for a grant. Owner only.
    pub fn auto_approve_set_config(
        env: Env,
        owner: Address,
        grant_id: u64,
        config: AutoApproveConfig,
    ) -> Result<(), ContractError> {
        auto_approve::set_config(&env, &owner, grant_id, config)
    }

    /// Attempt auto-approve for a milestone. Anyone may call; enforces all conditions.
    pub fn auto_approve_try(
        env: Env,
        caller: Address,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Result<bool, ContractError> {
        auto_approve::try_auto_approve(&env, &caller, grant_id, milestone_idx)
    }

    /// Return whether auto-approve conditions are currently met for a milestone.
    pub fn auto_approve_can(env: Env, grant_id: u64, milestone_idx: u32) -> bool {
        auto_approve::can_auto_approve(&env, grant_id, milestone_idx)
    }

    /// Return the auto-approve config for a grant.
    pub fn auto_approve_get_config(env: Env, grant_id: u64) -> Option<AutoApproveConfig> {
        auto_approve::get_config(&env, grant_id)
    }

    /// Return the auto-approve record if it was triggered.
    pub fn auto_approve_get_record(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<AutoApproveRecord> {
        auto_approve::get_record(&env, grant_id, milestone_idx)
    }

    // ── Issue #618: Automatic Lifecycle Transitions Based on Ledger Time ─

    /// Register a new timer for a grant. Owner or protocol (for defaults).
    pub fn timer_register(
        env: Env,
        caller: Address,
        grant_id: u64,
        trigger_type: TimerTriggerType,
        fires_at: u64,
    ) -> Result<(), ContractError> {
        grant_timer::register_timer(&env, &caller, grant_id, trigger_type, fires_at)
    }

    /// Attempt to fire all eligible timers for a grant. Anyone can call.
    pub fn timer_trigger(env: Env, caller: Address, grant_id: u64) -> u32 {
        grant_timer::trigger_timers(&env, &caller, grant_id)
    }

    /// Return all timers for a grant.
    pub fn timer_get_timers(env: Env, grant_id: u64) -> Vec<TimerRecord> {
        grant_timer::get_timers(&env, grant_id)
    }

    /// Return only unfired, eligible (past fires_at) timers.
    pub fn timer_pending(env: Env, grant_id: u64) -> Vec<TimerRecord> {
        grant_timer::pending_timers(&env, grant_id)
    }

    /// Cancel a timer (owner or admin only).
    pub fn timer_cancel(
        env: Env,
        caller: Address,
        grant_id: u64,
        trigger_type: TimerTriggerType,
    ) -> Result<(), ContractError> {
        grant_timer::cancel_timer(&env, &caller, grant_id, trigger_type)
    }
}

/// Issue #574: if a grant requires a performance bond, block milestone work until
/// the guarantor has posted it. No-op for grants without a bond requirement.
fn require_bond_posted(env: &Env, grant_id: u64) -> Result<(), ContractError> {
    if let Some(bond) = performance_bond::get_bond(env, grant_id) {
        if bond.status == BondStatus::Pending {
            return Err(ContractError::BondNotPosted);
        }
    }
    Ok(())
}

fn apply_milestone_submission(
    env: &Env,
    grant_id: u64,
    grant: &Grant,
    milestone_idx: u32,
    description: String,
    proof_url: String,
    actor: &Address,
) -> Result<(), ContractError> {
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }

    if let Some(existing) = Storage::get_milestone(env, grant_id, milestone_idx) {
        if existing.state == MilestoneState::Submitted || existing.state == MilestoneState::Approved
        {
            return Err(ContractError::MilestoneAlreadySubmitted);
        }
    }

    // Validate structured evidence against the schema when one has been registered.
    evidence_schema::validate_evidence(env, grant_id, milestone_idx)?;

    let is_cross_chain = crate::grant_bridge::has_valid_proof(env, grant_id, milestone_idx);
    let final_proof_url = if is_cross_chain {
        if let Some(proof) = crate::grant_bridge::get_proof(env, grant_id, milestone_idx) {
            proof.tx_hash
        } else {
            proof_url
        }
    } else {
        proof_url
    };
    
    let milestone = Milestone {
        idx: milestone_idx,
        description: description.clone(),
        amount: grant.milestone_amount,
        state: MilestoneState::Submitted,
        votes: soroban_sdk::Map::new(env),
        approvals: 0,
        rejections: 0,
        reasons: soroban_sdk::Map::new(env),
        status_updated_at: 0,
        proof_url: Some(final_proof_url),
        submission_timestamp: env.ledger().timestamp(),
        deadline: None,
        reviewer_count_snapshot: grant.reviewers.len(),
    };

    Storage::set_milestone(env, grant_id, milestone_idx, &milestone);
    Events::emit_milestone_submitted(env, grant_id, milestone_idx, description);

    audit::log(
        env,
        grant_id,
        AuditAction::MilestoneSubmitted,
        actor,
        Some(milestone_idx),
        Some(grant.milestone_amount),
    );

    provenance::record(
        env,
        ContributionType::MilestoneDelivered,
        actor,
        grant_id,
        Some(milestone_idx),
        Some(grant.milestone_amount),
        Some(grant.token.clone()),
        soroban_sdk::Vec::new(env),
    );

    Ok(())
}

/// Shared grant creation logic used by `grant_create` and template factories.
pub(crate) fn internal_grant_create(
    env: &Env,
    owner: &Address,
    title: String,
    description: String,
    token: &Address,
    total_amount: i128,
    milestone_amount: i128,
    num_milestones: u32,
    reviewers: soroban_sdk::Vec<Address>,
) -> Result<u64, ContractError> {
    if total_amount <= 0 || milestone_amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let protocol_cfg = config::get_config(env);

    if num_milestones == 0 || num_milestones > protocol_cfg.max_milestones_per_grant {
        return Err(ContractError::InvalidInput);
    }

    // Issue #512: check whitelist for reviewers
    for r in reviewers.iter() {
        if !whitelist::is_allowed(env, &r, &WhitelistScope::GlobalReviewer) {
            return Err(ContractError::AddressNotWhitelisted);
        }
    }

    if reviewers.len() > protocol_cfg.max_reviewers {
        return Err(ContractError::ReviewerLimitExceeded);
    }

    let total_required = milestone_amount
        .checked_mul(num_milestones as i128)
        .ok_or(ContractError::InvalidInput)?;

    if total_amount < total_required {
        return Err(ContractError::InvalidInput);
    }

    let grant_id = Storage::increment_grant_counter(env);

    let grant = Grant {
        id: grant_id,
        owner: owner.clone(),
        title: title.clone(),
        description,
        token: token.clone(),
        status: GrantStatus::Active,
        total_amount,
        milestone_amount,
        reviewers,
        total_milestones: num_milestones,
        milestones_paid_out: 0,
        escrow_balance: 0,
        funders: soroban_sdk::Vec::new(env),
        reason: None,
        timestamp: env.ledger().timestamp(),
        require_compliance: None,
    };

    Storage::set_grant(env, grant_id, &grant);

    // Maintain owner grant index for portfolio queries
    Storage::push_owner_grant_id(env, owner, grant_id);

    versioning::create_initial_version(env, &grant);
    Storage::set_escrow_state(
        env,
        grant_id,
        &EscrowState {
            mode: EscrowMode::Standard,
            lifecycle: EscrowLifecycleState::Funding,
            quorum_ready: false,
            approvals_count: 0,
        },
    );
    Storage::set_multisig_signers(env, grant_id, &soroban_sdk::Vec::new(env));

    escrow::open(env, grant_id, owner, token)?;

    grant_index::on_grant_created(env, grant_id, owner, token, GrantStatus::Active);

    Events::emit_grant_created(env, grant_id, owner.clone(), title, total_amount);

    audit::log(
        env,
        grant_id,
        AuditAction::GrantCreated,
        owner,
        None,
        Some(total_amount),
    );

    metrics::increment(env, MetricField::GrantsCreated, 1);
    metrics::increment(env, MetricField::GrantsActive, 1);

    if hooks::has_hooks(env, HookEvent::GrantCreated) {
        hooks::trigger(env, HookEvent::GrantCreated, Bytes::new(env));
    }

    provenance::record(
        env,
        ContributionType::GrantCreated,
        owner,
        grant_id,
        None,
        Some(total_amount),
        Some(token.clone()),
        soroban_sdk::Vec::new(env),
    );

    Ok(grant_id)
}
