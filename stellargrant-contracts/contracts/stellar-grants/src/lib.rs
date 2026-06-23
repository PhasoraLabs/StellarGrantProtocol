#![no_std]
#![allow(clippy::too_many_arguments)]
mod batch;
mod constants;
mod errors;
mod events;
mod governance;
mod migration;
mod reentrancy;
mod registry;
mod storage;
mod types;

pub use errors::ContractError;
pub use events::Events;
pub use storage::Storage;
pub use types::{
    BatchItemResult, BatchMilestoneVote, BatchResult, ContractError, ContractVersion,
    EscrowLifecycleState, EscrowMode, EscrowState, Grant, GrantFund, GrantStatus, MigrationRecord,
    Milestone, MilestoneState, MilestoneSubmission, RegistryEntry, RegistryEntryType,
};

use soroban_sdk::{contract, contractimpl, token, Address, Env, String, Vec};

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
        owner.require_auth();

        if total_amount <= 0 || milestone_amount <= 0 {
            return Err(ContractError::ZeroAmount);
        }

        if num_milestones == 0 || num_milestones > constants::MAX_MILESTONES_PER_GRANT {
            return Err(ContractError::InvalidInput);
        }

        if reviewers.len() > constants::MAX_REVIEWERS_PER_GRANT {
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
            token,
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

        Events::emit_grant_created(&env, grant_id, owner, title, total_amount);

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
            grants_count: 0,
            total_earned: 0,
        };

        Storage::set_contributor(&env, contributor.clone(), &profile);

        // Register in global index and emit contributor_registered event
        registry::register_contributor(&env, &contributor, &name)?;

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
            let mut grant =
                Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

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
                let mut total_contributions: i128 = 0;
                for fund_entry in grant.funders.iter() {
                    total_contributions += fund_entry.amount;
                }

                if total_contributions <= 0 {
                    return Err(ContractError::InvalidInput);
                }

                let token_client = token::Client::new(&env, &grant.token);
                let funders_len = grant.funders.len();
                let mut distributed = 0i128;

                for i in 0..funders_len {
                    let fund_entry = grant.funders.get(i).unwrap();
                    let is_last = i + 1 == funders_len;
                    let refund_amount = if is_last {
                        total_refundable - distributed
                    } else {
                        let amount = fund_entry
                            .amount
                            .checked_mul(total_refundable)
                            .ok_or(ContractError::InvalidInput)?
                            .checked_div(total_contributions)
                            .ok_or(ContractError::InvalidInput)?;
                        distributed += amount;
                        amount
                    };

                    if refund_amount > 0 {
                        token_client.transfer(
                            &env.current_contract_address(),
                            &fund_entry.funder,
                            &refund_amount,
                        );
                        Events::emit_refund_issued(
                            &env,
                            grant_id,
                            fund_entry.funder.clone(),
                            refund_amount,
                        );
                    }
                }
            }

            grant.status = GrantStatus::Cancelled;
            grant.escrow_balance = 0;
            grant.reason = Some(reason.clone());
            grant.timestamp = env.ledger().timestamp();

            Storage::set_grant(&env, grant_id, &grant);

            Events::emit_grant_cancelled(&env, grant_id, caller, reason, total_refundable);

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
        let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if grant.status != GrantStatus::Active {
            return Err(ContractError::InvalidState);
        }

        let total_paid =
            Self::compute_total_paid_if_quorum_ready(env, grant_id, grant.total_milestones)?;
        if grant.escrow_balance < total_paid {
            return Err(ContractError::InvalidInput);
        }
        let remaining_balance = grant.escrow_balance - total_paid;
        let token_client = token::Client::new(env, &grant.token);

        if total_paid > 0 {
            token_client.transfer(&env.current_contract_address(), &grant.owner, &total_paid);
        }

        if remaining_balance > 0 {
            let mut total_contributions: i128 = 0;
            for fund_entry in grant.funders.iter() {
                total_contributions += fund_entry.amount;
            }

            if total_contributions > 0 {
                let funders_len = grant.funders.len();
                let mut distributed = 0i128;
                for i in 0..funders_len {
                    let fund_entry = grant.funders.get(i).unwrap();
                    let is_last = i + 1 == funders_len;
                    let refund_amount = if is_last {
                        remaining_balance - distributed
                    } else {
                        let amount = fund_entry
                            .amount
                            .checked_mul(remaining_balance)
                            .ok_or(ContractError::InvalidInput)?
                            .checked_div(total_contributions)
                            .ok_or(ContractError::InvalidInput)?;
                        distributed += amount;
                        amount
                    };

                    if refund_amount > 0 {
                        token_client.transfer(
                            &env.current_contract_address(),
                            &fund_entry.funder,
                            &refund_amount,
                        );
                        Events::emit_final_refund(
                            env,
                            grant_id,
                            fund_entry.funder.clone(),
                            refund_amount,
                        );
                    }
                }
            }
        }

        grant.status = GrantStatus::Completed;
        grant.escrow_balance = 0;
        grant.milestones_paid_out = grant.total_milestones;
        grant.timestamp = env.ledger().timestamp();
        Storage::set_grant(env, grant_id, &grant);

        let mut escrow_state = Storage::get_escrow_state(env, grant_id);
        escrow_state.lifecycle = EscrowLifecycleState::Released;
        escrow_state.quorum_ready = true;
        Storage::set_escrow_state(env, grant_id, &escrow_state);

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
        )
    }

    /// Submits multiple milestones in one transaction.
    pub fn milestone_submit_batch(
        env: Env,
        grant_id: u64,
        recipient: Address,
        submissions: Vec<MilestoneSubmission>,
    ) -> Result<(), ContractError> {
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
        funder.require_auth();
        reentrancy::with_non_reentrant(&env, || {
            if amount <= 0 {
                return Err(ContractError::ZeroAmount);
            }

            let mut grant =
                Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            if grant.status != GrantStatus::Active {
                return Err(ContractError::InvalidState);
            }

            let token_client = token::Client::new(&env, &grant.token);
            let contract_address = env.current_contract_address();
            token_client.transfer(&funder, &contract_address, &amount);

            grant.escrow_balance = grant
                .escrow_balance
                .checked_add(amount)
                .ok_or(ContractError::InvalidInput)?;

            let mut funder_found = false;
            for i in 0..grant.funders.len() {
                let mut fund_entry = grant.funders.get(i).unwrap();
                if fund_entry.funder == funder {
                    fund_entry.amount = fund_entry
                        .amount
                        .checked_add(amount)
                        .ok_or(ContractError::InvalidInput)?;
                    grant.funders.set(i, fund_entry);
                    funder_found = true;
                    break;
                }
            }

            if !funder_found {
                grant.funders.push_back(GrantFund {
                    funder: funder.clone(),
                    amount,
                });
            }

            Storage::set_grant(&env, grant_id, &grant);

            Events::emit_grant_funded(&env, grant_id, funder, amount, grant.escrow_balance);

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

        let contract_addr = env.current_contract_address();
        let client = token::Client::new(&env, &grant.token);
        client.transfer(&reviewer, &contract_addr, &amount);

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
        let client = token::Client::new(&env, &grant.token);
        client.transfer(&env.current_contract_address(), &treasury, &stake);

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

        let client = token::Client::new(&env, &grant.token);
        client.transfer(&env.current_contract_address(), &reviewer, &stake);

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

    // ── Batch Operations (#526) ─────────────────────────────────────

    /// Vote on multiple milestones in one call. Partial failures are recorded per item.
    pub fn batch_vote_milestones(
        env: Env,
        reviewer: Address,
        votes: Vec<BatchMilestoneVote>,
    ) -> Result<BatchResult, ContractError> {
        batch::batch_vote_milestones(&env, &reviewer, votes)
    }

    /// Fund multiple grants with the same token in one call. Partial failures are recorded per item.
    pub fn batch_fund_grants(
        env: Env,
        funder: Address,
        token: Address,
        items: Vec<(u64, i128)>,
    ) -> Result<BatchResult, ContractError> {
        batch::batch_fund_grants(&env, &funder, &token, items)
    }

    /// Cancel multiple grants. Callable by grant owner or global admin per grant.
    pub fn batch_cancel_grants(
        env: Env,
        caller: Address,
        grant_ids: Vec<u64>,
        reason: String,
    ) -> Result<BatchResult, ContractError> {
        batch::batch_cancel_grants(&env, &caller, grant_ids, reason)
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

            let mut grant =
                Storage::get_grant(&env, grant_id).ok_or(ContractError::GrantNotFound)?;

            if grant.status != GrantStatus::Active {
                return Err(ContractError::InvalidState);
            }

            let contract_addr = env.current_contract_address();
            let client = token::Client::new(&env, &grant.token);
            client.transfer(&funder, &contract_addr, &amount);

            grant.escrow_balance = grant
                .escrow_balance
                .checked_add(amount)
                .ok_or(ContractError::InvalidInput)?;

            let mut found = false;
            let mut new_funders = soroban_sdk::Vec::new(&env);
            for f in grant.funders.iter() {
                if f.funder == funder {
                    new_funders.push_back(GrantFund {
                        funder: f.funder,
                        amount: f.amount + amount,
                    });
                    found = true;
                } else {
                    new_funders.push_back(f);
                }
            }
            if !found {
                new_funders.push_back(GrantFund {
                    funder: funder.clone(),
                    amount,
                });
            }
            grant.funders = new_funders;

            Storage::set_grant(&env, grant_id, &grant);

            Events::emit_grant_funded(&env, grant_id, funder.clone(), amount, grant.escrow_balance);
        }

        Ok(())
    }
}

fn apply_milestone_submission(
    env: &Env,
    grant_id: u64,
    grant: &Grant,
    milestone_idx: u32,
    description: String,
    proof_url: String,
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

    Ok(())
}

#[cfg(test)]
mod test;
