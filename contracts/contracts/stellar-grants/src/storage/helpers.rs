use crate::storage::keys::StorageKey;
use crate::types::{BountyGrant, BountySubmission};
use soroban_sdk::{Address, Env, Vec};

pub struct Storage;

impl Storage {
    pub fn next_bounty_id(env: &Env) -> u64 {
        let key = StorageKey::BountyCounter;
        let current: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        let next = current + 1;
        env.storage().persistent().set(&key, &next);
        next
    }

    pub fn get_bounty(env: &Env, id: u64) -> Option<BountyGrant> {
        let key = StorageKey::Bounty(id);
        env.storage().persistent().get(&key)
    }

    pub fn set_bounty(env: &Env, bounty: &BountyGrant) {
        let key = StorageKey::Bounty(bounty.id);
        env.storage().persistent().set(&key, bounty);
    }

    pub fn get_bounty_submission(env: &Env, bounty_id: u64, submitter: &Address) -> Option<BountySubmission> {
        let key = StorageKey::BountySubmission {
            bounty_id,
            submitter: submitter.clone(),
        };
        env.storage().persistent().get(&key)
    }

    pub fn set_bounty_submission(env: &Env, submission: &BountySubmission) {
        let key = StorageKey::BountySubmission {
            bounty_id: submission.bounty_id,
            submitter: submission.submitter.clone(),
        };
        env.storage().persistent().set(&key, submission);
    }

    pub fn get_bounty_submitters(env: &Env, bounty_id: u64) -> Vec<Address> {
        let key = StorageKey::BountySubmitters(bounty_id);
        env.storage().persistent().get(&key).unwrap_or(Vec::new(env))
    }

    pub fn add_bounty_submitter(env: &Env, bounty_id: u64, submitter: &Address) {
        let key = StorageKey::BountySubmitters(bounty_id);
        let mut submitters = Self::get_bounty_submitters(env, bounty_id);
        submitters.push_back(submitter.clone());
        env.storage().persistent().set(&key, &submitters);
    }
}
