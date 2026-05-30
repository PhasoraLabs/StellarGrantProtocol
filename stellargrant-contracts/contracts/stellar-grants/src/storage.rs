use crate::types::{ContractError, ContributorProfile, EscrowState, Grant, Milestone};
use soroban_sdk::{contracttype, Address, Env, Vec};

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
}

pub struct Storage;

impl Storage {
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
            .unwrap_or(1) // Default reputation is 1
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

    pub fn get_identity_oracle(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::IdentityOracle)
    }
}
