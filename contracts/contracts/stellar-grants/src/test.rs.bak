use crate::types::{DisputeInfo, EscrowLifecycleState, EscrowMode, EscrowState, Grant, Milestone};
use soroban_sdk::{contracttype, Env};

#[contracttype]
pub enum DataKey {
    Grant(u64),
    Milestone(u64, u32),
    GrantCounter,
    Contributor(soroban_sdk::Address),
    /// Reviewer stake amount for a grant: (grant_id, reviewer) -> i128
    ReviewerStake(u64, soroban_sdk::Address),
    /// Minimum stake required to review a grant
    MinReviewerStake,
    /// Treasury address for slashed stakes
    Treasury,
    /// Identity oracle contract address for KYC verification
    IdentityOracle,
    ReviewerReputation(soroban_sdk::Address),
    GlobalAdmin,
    Council,
    EscrowState(u64),
    MultisigSigners(u64),
    ReleaseSignerApproval(u64, soroban_sdk::Address),
    GrantMinReputation(u64),
    /// Tracks whether a voter has already upvoted a specific milestone.
    MilestoneUpvoter(u64, u32, soroban_sdk::Address),
    Blacklist(soroban_sdk::Address),
    /// Per-status index: maps GrantStatus discriminant → Vec<u64> of grant IDs.
    GrantStatusIndex(u32),
    /// Monotonic schema / upgrade generation for migrations (see `UPGRADE_GUIDE.md`).
    StorageVersion,
    /// Global contract pause flag stored in instance storage.
    IsPaused,
    /// Tracks whether reputation was already credited for a milestone payout (issue #151).
    MilestoneReputationApplied(u64, u32),
    /// Global dispute fee amount in the primary token (issue #152).
    DisputeFeeAmount,
    /// Dispute fee info stored per milestone when a dispute is raised (issue #152).
    MilestoneDisputeInfo(u64, u32),
    /// Tracks whether a funder has already voted on a milestone.
    FunderVote(u64, u32, soroban_sdk::Address),
}

pub struct Storage;

// Soroban TTL values are expressed in ledgers. At roughly 5 seconds per ledger,
// this refreshes entries when they have less than about 6 days left and extends
// them out to roughly 58 days, which keeps long-lived grants active without
// needing constant writes.
const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_000_000;

impl Storage {
    fn bump_persistent_ttl(env: &Env, key: &DataKey) {
        env.storage().persistent().extend_ttl(
            key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }

    // --- Staking helpers ---

    pub fn get_reviewer_stake(env: &Env, grant_id: u64, reviewer: &soroban_sdk::Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::ReviewerStake(grant_id, reviewer.clone()))
            .unwrap_or(0)
    }

    pub fn set_reviewer_stake(
        env: &Env,
        grant_id: u64,
        reviewer: &soroban_sdk::Address,
        amount: i128,
    ) {
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

    pub fn get_treasury(env: &Env) -> Option<soroban_sdk::Address> {
        env.storage().persistent().get(&DataKey::Treasury)
    }

    pub fn get_identity_oracle(env: &Env) -> Option<soroban_sdk::Address> {
        env.storage().persistent().get(&DataKey::IdentityOracle)
    }

    pub fn get_global_admin(env: &Env) -> Option<soroban_sdk::Address> {
        env.storage().persistent().get(&DataKey::GlobalAdmin)
    }

    pub fn set_global_admin(env: &Env, admin: &soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::GlobalAdmin, admin);
    }

    pub fn get_council(env: &Env) -> Option<soroban_sdk::Address> {
        env.storage().persistent().get(&DataKey::Council)
    }

    pub fn set_council(env: &Env, council: &soroban_sdk::Address) {
        env.storage().persistent().set(&DataKey::Council, council);
    }

    pub fn get_storage_version(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::StorageVersion)
            .unwrap_or(1)
    }

    pub fn set_storage_version(env: &Env, version: u32) {
        env.storage()
            .persistent()
            .set(&DataKey::StorageVersion, &version);
    }

    pub fn get_grant(env: &Env, grant_id: u64) -> Option<Grant> {
        let key = DataKey::Grant(grant_id);
        let grant = env.storage().persistent().get(&key);
        if grant.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        grant
    }

    pub fn set_grant(env: &Env, grant_id: u64, grant: &Grant) {
        let key = DataKey::Grant(grant_id);
        env.storage().persistent().set(&key, grant);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn has_grant(env: &Env, grant_id: u64) -> bool {
        env.storage().persistent().has(&DataKey::Grant(grant_id))
    }

    pub fn get_milestone(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Milestone> {
        let key = DataKey::Milestone(grant_id, milestone_idx);
        let milestone = env.storage().persistent().get(&key);
        if milestone.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        milestone
    }

    pub fn set_milestone(env: &Env, grant_id: u64, milestone_idx: u32, milestone: &Milestone) {
        let key = DataKey::Milestone(grant_id, milestone_idx);
        env.storage().persistent().set(&key, milestone);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn increment_grant_counter(env: &Env) -> u64 {
        let key = DataKey::GrantCounter;
        let mut counter: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        counter += 1;
        env.storage().persistent().set(&key, &counter);
        Self::bump_persistent_ttl(env, &key);
        counter
    }

    pub fn get_contributor(
        env: &Env,
        contributor: soroban_sdk::Address,
    ) -> Option<crate::types::ContributorProfile> {
        let key = DataKey::Contributor(contributor);
        let profile = env.storage().persistent().get(&key);
        if profile.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        profile
    }

    pub fn set_contributor(
        env: &Env,
        contributor: soroban_sdk::Address,
        profile: &crate::types::ContributorProfile,
    ) {
        let key = DataKey::Contributor(contributor);
        env.storage().persistent().set(&key, profile);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_reviewer_reputation(env: &Env, reviewer: soroban_sdk::Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ReviewerReputation(reviewer))
            .unwrap_or(1) // Default basic reputation
    }

    pub fn set_reviewer_reputation(env: &Env, reviewer: soroban_sdk::Address, score: u32) {
        env.storage()
            .persistent()
            .set(&DataKey::ReviewerReputation(reviewer), &score);
    }

    pub fn get_escrow_state(env: &Env, grant_id: u64) -> EscrowState {
        env.storage()
            .persistent()
            .get(&DataKey::EscrowState(grant_id))
            .unwrap_or(EscrowState::new(
                EscrowMode::Standard,
                EscrowLifecycleState::Funding,
                false,
                0,
            ))
    }

    pub fn set_escrow_state(env: &Env, grant_id: u64, state: &EscrowState) {
        env.storage()
            .persistent()
            .set(&DataKey::EscrowState(grant_id), state);
    }

    pub fn get_multisig_signers(
        env: &Env,
        grant_id: u64,
    ) -> soroban_sdk::Vec<soroban_sdk::Address> {
        env.storage()
            .persistent()
            .get(&DataKey::MultisigSigners(grant_id))
            .unwrap_or(soroban_sdk::Vec::new(env))
    }

    pub fn set_multisig_signers(
        env: &Env,
        grant_id: u64,
        signers: &soroban_sdk::Vec<soroban_sdk::Address>,
    ) {
        env.storage()
            .persistent()
            .set(&DataKey::MultisigSigners(grant_id), signers);
    }

    pub fn has_release_approval(env: &Env, grant_id: u64, signer: &soroban_sdk::Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseSignerApproval(grant_id, signer.clone()))
            .unwrap_or(false)
    }

    pub fn set_release_approval(
        env: &Env,
        grant_id: u64,
        signer: &soroban_sdk::Address,
        approved: bool,
    ) {
        env.storage().persistent().set(
            &DataKey::ReleaseSignerApproval(grant_id, signer.clone()),
            &approved,
        );
    }

    pub fn get_grant_min_reputation(env: &Env, grant_id: u64) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::GrantMinReputation(grant_id))
            .unwrap_or(0)
    }

    pub fn set_grant_min_reputation(env: &Env, grant_id: u64, min_reputation: u64) {
        env.storage()
            .persistent()
            .set(&DataKey::GrantMinReputation(grant_id), &min_reputation);
    }

    pub fn has_milestone_upvote(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        voter: &soroban_sdk::Address,
    ) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::MilestoneUpvoter(
                grant_id,
                milestone_idx,
                voter.clone(),
            ))
            .unwrap_or(false)
    }

    pub fn set_milestone_upvote(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        voter: &soroban_sdk::Address,
    ) {
        env.storage().persistent().set(
            &DataKey::MilestoneUpvoter(grant_id, milestone_idx, voter.clone()),
            &true,
        );
    }

    pub fn is_blacklisted(env: &Env, address: &soroban_sdk::Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Blacklist(address.clone()))
    }

    pub fn set_blacklisted(env: &Env, address: &soroban_sdk::Address) {
        env.storage()
            .persistent()
            .set(&DataKey::Blacklist(address.clone()), &true);
    }

    pub fn remove_blacklisted(env: &Env, address: &soroban_sdk::Address) {
        env.storage()
            .persistent()
            .remove(&DataKey::Blacklist(address.clone()));
    }

    // --- Grant status index helpers ---

    /// Maximum number of grant IDs stored per status bucket to bound gas costs.
    pub const STATUS_INDEX_MAX: u32 = 500;

    pub fn get_status_index(env: &Env, status: u32) -> soroban_sdk::Vec<u64> {
        let key = DataKey::GrantStatusIndex(status);
        let vec = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Vec::new(env));
        if !vec.is_empty() {
            Self::bump_persistent_ttl(env, &key);
        }
        vec
    }

    pub fn set_status_index(env: &Env, status: u32, ids: &soroban_sdk::Vec<u64>) {
        let key = DataKey::GrantStatusIndex(status);
        env.storage().persistent().set(&key, ids);
        Self::bump_persistent_ttl(env, &key);
    }

    /// Add `grant_id` to the index for `status`, respecting the cap.
    pub fn index_add(env: &Env, status: u32, grant_id: u64) {
        let mut ids = Self::get_status_index(env, status);
        if ids.len() >= Self::STATUS_INDEX_MAX {
            return; // silently drop when cap is reached
        }
        if !ids.contains(grant_id) {
            ids.push_back(grant_id);
            Self::set_status_index(env, status, &ids);
        }
    }

    /// Remove `grant_id` from the index for `status`.
    pub fn index_remove(env: &Env, status: u32, grant_id: u64) {
        let ids = Self::get_status_index(env, status);
        let mut new_ids = soroban_sdk::Vec::new(env);
        for id in ids.iter() {
            if id != grant_id {
                new_ids.push_back(id);
            }
        }
        Self::set_status_index(env, status, &new_ids);
    }

    /// Move `grant_id` from one status bucket to another atomically.
    pub fn index_transition(env: &Env, from: u32, to: u32, grant_id: u64) {
        Self::index_remove(env, from, grant_id);
        Self::index_add(env, to, grant_id);
    }

    // --- Global pause flag (instance storage) ---

    pub fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    pub fn set_paused(env: &Env, paused: bool) {
        env.storage().instance().set(&DataKey::IsPaused, &paused);
    }

    // --- Issue #151: milestone reputation tracking ---

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

    // --- Issue #152: dispute fee ---

    pub fn get_dispute_fee_amount(env: &Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::DisputeFeeAmount)
            .unwrap_or(0)
    }

    pub fn set_dispute_fee_amount(env: &Env, amount: i128) {
        let key = DataKey::DisputeFeeAmount;
        env.storage().persistent().set(&key, &amount);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn get_milestone_dispute_info(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
    ) -> Option<DisputeInfo> {
        let key = DataKey::MilestoneDisputeInfo(grant_id, milestone_idx);
        let info = env.storage().persistent().get(&key);
        if info.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        info
    }

    pub fn set_milestone_dispute_info(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        info: &DisputeInfo,
    ) {
        let key = DataKey::MilestoneDisputeInfo(grant_id, milestone_idx);
        env.storage().persistent().set(&key, info);
        Self::bump_persistent_ttl(env, &key);
    }

    pub fn remove_milestone_dispute_info(env: &Env, grant_id: u64, milestone_idx: u32) {
        env.storage()
            .persistent()
            .remove(&DataKey::MilestoneDisputeInfo(grant_id, milestone_idx));
    }

    pub fn get_funder_vote(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        funder: &soroban_sdk::Address,
    ) -> Option<bool> {
        let key = DataKey::FunderVote(grant_id, milestone_idx, funder.clone());
        let vote = env.storage().persistent().get(&key);
        if vote.is_some() {
            Self::bump_persistent_ttl(env, &key);
        }
        vote
    }

    pub fn set_funder_vote(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        funder: &soroban_sdk::Address,
        approve: bool,
    ) {
        let key = DataKey::FunderVote(grant_id, milestone_idx, funder.clone());
        env.storage().persistent().set(&key, &approve);
        Self::bump_persistent_ttl(env, &key);
    }
}
