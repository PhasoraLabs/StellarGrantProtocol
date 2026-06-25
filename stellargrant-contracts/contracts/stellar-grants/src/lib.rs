#![no_std]
#![allow(clippy::too_many_arguments)]
mod audit;
mod bounty;
mod compliance;
mod config;
mod constants;
mod dao;
mod dispute;
mod emergency;
mod errors;
mod escrow;
mod events;
mod fees;
mod governance;
mod hooks;
mod insurance;
mod metrics;
mod migration;
mod multisig;
mod oracle;
mod pagination;
mod quadratic;
mod reentrancy;
mod registry;
mod reputation;
mod storage;
mod streaming;
mod treasury;
mod types;

pub use errors::ContractError;
pub use events::Events;
pub use storage::Storage;
pub use types::{
    AuditAction, AuditEntry, BountyGrant, BountyStatus, BountySubmission, ComplianceAttestation,
    ComplianceLevel, ComplianceStatus, ContractVersion, DaoProposal, DaoProposalStatus,
    DaoProposalType, DaoVote, Dispute, DisputeStatus, EscrowAccount, EscrowLifecycleState,
    EscrowMode, EscrowState, FeeRecord, FunderLedger, Grant, GrantFund, GrantStatus,
    HookCallResult, HookEvent, HookRegistration, InsuranceClaim, InsurancePolicy, MigrationRecord,
    Milestone, MilestoneState, MilestoneSubmission, MultisigProposal, MultisigSigner, OracleConfig,
    PauseRecord, PaymentStream, PriceQuote, ProtocolConfig, ProtocolMetrics, QuadraticVoteRecord,
    RegistryEntry, RegistryEntryType, ReputationTier, SignatureStatus, TokenMetric,
    TreasurySnapshot, VoiceCredits, VotingMechanism,
};

use metrics::MetricField;
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, String, Vec};

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
    /// Once DAO mode is enabled, admin rotation must go through a passed
    /// `DaoProposalType::ChangeAdmin` proposal instead of this direct call.
    pub fn set_global_admin(
        env: Env,
        caller: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        dao::require_dao_mode_disabled(&env)?;
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
        owner.require_auth();

        if total_amount <= 0 || milestone_amount <= 0 {
            return Err(ContractError::ZeroAmount);
        }

        let protocol_cfg = config::get_config(&env);

        if num_milestones == 0 || num_milestones > protocol_cfg.max_milestones_per_grant {
            return Err(ContractError::InvalidInput);
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

        let grant_id = Storage::increment_grant_counter(&env);

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
            funders: soroban_sdk::Vec::new(&env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };

        Storage::set_grant(&env, grant_id, &grant);
        Storage::set_escrow_state(
            &env,
            grant_id,
            &EscrowState {
                mode: EscrowMode::Standard,
                lifecycle: EscrowLifecycleState::Funding,
                quorum_ready: false,
                approvals_count: 0,
            },
        );
        Storage::set_multisig_signers(&env, grant_id, &soroban_sdk::Vec::new(&env));

        escrow::open(&env, grant_id, &owner, &token)?;

        Events::emit_grant_created(&env, grant_id, owner.clone(), title, total_amount);

        audit::log(
            &env,
            grant_id,
            AuditAction::GrantCreated,
            &owner,
            None,
            Some(total_amount),
        );

        metrics::increment(&env, MetricField::GrantsCreated, 1);
        metrics::increment(&env, MetricField::GrantsActive, 1);

        if hooks::has_hooks(&env, HookEvent::GrantCreated) {
            hooks::trigger(&env, HookEvent::GrantCreated, Bytes::new(&env));
        }

        Ok(grant_id)
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

            let total_refundable = grant.escrow_balance;
            if total_refundable > 0 {
                escrow::refund_all(&env, grant_id)?;
            }

            let mut grant =
                Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;
            grant.status = GrantStatus::Cancelled;
            grant.escrow_balance = 0;
            grant.reason = Some(reason.clone());
            grant.timestamp = env.ledger().timestamp();

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

        // Compliance gate: if the grant requires KYC, check the owner/contributor.
        if let Some(ref required_level) = grant.require_compliance {
            compliance::require_compliant(env, &grant.owner, required_level.clone())?;
        }

        let total_paid =
            Self::compute_total_paid_if_quorum_ready(env, grant_id, grant.total_milestones)?;
        if grant.escrow_balance < total_paid {
            return Err(ContractError::InvalidInput);
        }
        let remaining_balance = grant.escrow_balance - total_paid;

        if total_paid > 0 {
            escrow::release(env, grant_id, &grant.owner, total_paid)?;
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
        reviewer.require_auth();

        let mut grant = Storage::get_grant_v(&env, grant_id);
        let mut milestone = Storage::get_milestone_v(&env, grant_id, milestone_idx);

        let result = governance::cast_vote(
            &env,
            &mut grant,
            &mut milestone,
            &reviewer,
            approve,
            feedback,
        )?;

        Storage::set_milestone(&env, grant_id, milestone_idx, &milestone);

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

        let mut total_weight: u32 = 0;
        for r in grant.reviewers.iter() {
            total_weight += Storage::get_reviewer_reputation(&env, r);
        }

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
        recipient.require_auth();

        let grant = Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

        if grant.status != GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        if grant.owner != recipient {
            return Err(ContractError::Unauthorized);
        }

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

            metrics::update_token_locked(&env, &grant.token, amount);

            Ok(())
        })
    }

    /// Retrieve a grant by its ID
    pub fn get_grant(env: Env, grant_id: u64) -> Result<Grant, ContractError> {
        Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)
    }

    /// Paginated list of all grants, ordered by ascending grant ID.
    pub fn get_grants_page(env: Env, offset: u32, limit: u32) -> Vec<Grant> {
        let total = Storage::get_grant_count(&env);
        let mut all: Vec<Grant> = Vec::new(&env);
        let mut id: u64 = 1;
        while id <= total {
            if let Some(grant) = Storage::get_grant(&env, id) {
                all.push_back(grant);
            }
            id += 1;
        }
        pagination::paginate(&env, &all, offset, limit)
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

    // ── Issue #514: Dispute Resolution Entry Points ───────────────────────────

    pub fn dispute_raise(
        env: Env,
        grant_id: u64,
        milestone_idx: u32,
        caller: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();
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

    /// Update the runtime protocol configuration directly. Admin only, and
    /// only while DAO mode is disabled — once enabled, config changes must
    /// go through a passed `DaoProposalType::UpdateConfig` proposal.
    pub fn update_config(
        env: Env,
        admin: Address,
        new_config: ProtocolConfig,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        dao::require_dao_mode_disabled(&env)?;
        config::set_config(&env, &admin, new_config)
    }

    pub fn get_protocol_config(env: Env) -> ProtocolConfig {
        config::get_config(&env)
    }

    // ── Issue #517: Protocol Fee Management Entry Points ─────────────────────

    pub fn get_fees_collected(env: Env, token: Address) -> i128 {
        fees::total_fees_collected(&env, &token)
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
        grant.require_compliance = Some(level);
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

    // ── Issue #519: Protocol Treasury Management Entry Points ────────────────

    /// Configure the treasury management address. Admin only.
    pub fn set_treasury_address(
        env: Env,
        admin: Address,
        treasury: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        treasury::set_treasury_address(&env, &admin, &treasury)
    }

    /// Record protocol fees / unclaimed funds into the treasury for `token`.
    pub fn treasury_deposit(
        env: Env,
        caller: Address,
        token: Address,
        amount: i128,
    ) -> Result<i128, ContractError> {
        caller.require_auth();
        treasury::deposit(&env, &token, &caller, amount)
    }

    /// Withdraw `amount` of `token` from the treasury to `to`. Admin only.
    pub fn treasury_withdraw(
        env: Env,
        admin: Address,
        token: Address,
        to: Address,
        amount: i128,
    ) -> Result<i128, ContractError> {
        admin.require_auth();
        treasury::withdraw(&env, &admin, &token, &to, amount)
    }

    /// Reallocate treasury accounting from one token balance to another. Admin only.
    pub fn treasury_reallocate(
        env: Env,
        admin: Address,
        from_token: Address,
        to_token: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        treasury::reallocate(&env, &admin, &from_token, &to_token, amount)
    }

    /// Current treasury balance for `token`.
    pub fn treasury_balance(env: Env, token: Address) -> i128 {
        treasury::balance(&env, &token)
    }

    /// Point-in-time treasury health snapshot for `token`, for frontend display.
    pub fn treasury_snapshot(env: Env, token: Address) -> TreasurySnapshot {
        treasury::snapshot(&env, &token)
    }

    // ── Issue #532: Protocol-Wide DAO Governance Entry Points ────────────────

    /// Enable or disable DAO governance mode. Admin only. Once enabled,
    /// `update_config` and `set_global_admin` are locked out in favor of
    /// passed-and-executed DAO proposals.
    pub fn set_dao_mode(env: Env, admin: Address, enabled: bool) -> Result<(), ContractError> {
        admin.require_auth();
        dao::set_dao_mode(&env, &admin, enabled)
    }

    pub fn is_dao_mode_enabled(env: Env) -> bool {
        dao::is_dao_mode_enabled(&env)
    }

    /// Configure the voting period (in ledgers) for newly created proposals. Admin only.
    pub fn dao_set_voting_period(
        env: Env,
        admin: Address,
        ledgers: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        dao::set_voting_period(&env, &admin, ledgers)
    }

    /// Configure the minimum total vote weight required to finalize a proposal. Admin only.
    pub fn dao_set_quorum_votes(env: Env, admin: Address, quorum: u64) -> Result<(), ContractError> {
        admin.require_auth();
        dao::set_quorum_votes(&env, &admin, quorum)
    }

    /// Submit a new governance proposal.
    pub fn dao_propose(
        env: Env,
        proposer: Address,
        title: String,
        description: String,
        proposal_type: DaoProposalType,
    ) -> Result<u64, ContractError> {
        proposer.require_auth();
        dao::create_proposal(&env, &proposer, title, description, proposal_type)
    }

    /// Cast a reputation-weighted vote on an active proposal.
    pub fn dao_vote(
        env: Env,
        voter: Address,
        proposal_id: u64,
        support: bool,
    ) -> Result<DaoProposal, ContractError> {
        voter.require_auth();
        dao::vote(&env, &voter, proposal_id, support)
    }

    /// Finalize a proposal once its voting deadline has passed.
    pub fn dao_finalize(env: Env, proposal_id: u64) -> Result<DaoProposalStatus, ContractError> {
        dao::finalize(&env, proposal_id)
    }

    /// Execute a passed proposal's on-chain effect.
    pub fn dao_execute(
        env: Env,
        executor: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        executor.require_auth();
        dao::execute(&env, &executor, proposal_id)
    }

    /// Cancel an active proposal. Proposer or admin only.
    pub fn dao_cancel(env: Env, caller: Address, proposal_id: u64) -> Result<(), ContractError> {
        caller.require_auth();
        dao::cancel(&env, &caller, proposal_id)
    }

    /// Fetch a DAO proposal by ID.
    pub fn get_dao_proposal(env: Env, proposal_id: u64) -> Option<DaoProposal> {
        dao::get_proposal(&env, proposal_id)
    }

    // ── Issue #533: Competitive Bounty-Mode Grants Entry Points ──────────────

    /// Publish a new bounty-mode grant. The owner deposits the full prize
    /// up front; it is escrowed until a winner is selected or it is cancelled.
    #[allow(clippy::too_many_arguments)]
    pub fn bounty_create(
        env: Env,
        owner: Address,
        title: String,
        description: String,
        token: Address,
        prize_amount: i128,
        submission_window_ledgers: u32,
    ) -> Result<u64, ContractError> {
        owner.require_auth();
        emergency::require_not_paused(&env)?;
        let id = bounty::create_bounty(
            &env,
            &owner,
            title,
            description,
            &token,
            prize_amount,
            submission_window_ledgers,
        )?;
        metrics::increment(&env, MetricField::BountiesCreated, 1);
        Ok(id)
    }

    /// Submit a solution to an open bounty. One submission per contributor.
    pub fn bounty_submit(
        env: Env,
        submitter: Address,
        bounty_id: u64,
        proof_url: String,
    ) -> Result<(), ContractError> {
        submitter.require_auth();
        bounty::submit_solution(&env, bounty_id, &submitter, proof_url)
    }

    /// Close a bounty to new submissions while the owner reviews entries. Owner only.
    pub fn bounty_start_review(env: Env, caller: Address, bounty_id: u64) -> Result<(), ContractError> {
        caller.require_auth();
        bounty::start_review(&env, &caller, bounty_id)
    }

    /// Select the winning submission and pay out the full prize. Owner only.
    pub fn bounty_select_winner(
        env: Env,
        caller: Address,
        bounty_id: u64,
        winner: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        bounty::select_winner(&env, &caller, bounty_id, &winner)?;
        metrics::increment(&env, MetricField::BountiesAwarded, 1);
        if hooks::has_hooks(&env, HookEvent::BountyAwarded) {
            hooks::trigger(&env, HookEvent::BountyAwarded, Bytes::new(&env));
        }
        Ok(())
    }

    /// Cancel an unresolved bounty and refund the prize to the owner.
    pub fn bounty_cancel(env: Env, caller: Address, bounty_id: u64) -> Result<(), ContractError> {
        caller.require_auth();
        bounty::cancel_bounty(&env, &caller, bounty_id)
    }

    /// Fetch a bounty-mode grant by ID.
    pub fn get_bounty(env: Env, bounty_id: u64) -> Option<BountyGrant> {
        bounty::get_bounty(&env, bounty_id)
    }

    /// Fetch a specific contributor's submission for a bounty.
    pub fn get_bounty_submission(
        env: Env,
        bounty_id: u64,
        submitter: Address,
    ) -> Option<BountySubmission> {
        bounty::get_submission(&env, bounty_id, &submitter)
    }

    /// List all addresses that submitted a solution to a bounty.
    pub fn get_bounty_submitters(env: Env, bounty_id: u64) -> Vec<Address> {
        bounty::list_submitters(&env, bounty_id)
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
        proof_url: Some(proof_url),
        submission_timestamp: env.ledger().timestamp(),
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

    Ok(())
}

#[cfg(test)]
mod test;
