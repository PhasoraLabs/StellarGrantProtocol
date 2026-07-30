#![no_std]

mod bounty;
mod constants;
mod errors;
mod events;
mod metrics;
mod storage;
mod types;

use crate::bounty::BountyModule;
use crate::errors::Error;
use crate::metrics::MetricsModule;
use crate::types::{BountyGrant, BountySubmission};
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

#[contract]
pub struct StellarGrantsContract;

#[contractimpl]
impl StellarGrantsContract {
    pub fn create_bounty(
        env: Env,
        owner: Address,
        title: String,
        description: String,
        token: Address,
        prize_amount: i128,
        submission_deadline: u64,
    ) -> Result<u64, Error> {
        let bounty_id = BountyModule::create_bounty(
            &env,
            owner,
            title,
            description,
            token,
            prize_amount,
            submission_deadline,
        )?;
        MetricsModule::increment_bounties_created(&env)?;
        Ok(bounty_id)
    }

    pub fn submit_bounty_solution(
        env: Env,
        bounty_id: u64,
        submitter: Address,
        proof_url: String,
    ) -> Result<(), Error> {
        BountyModule::submit_solution(&env, bounty_id, submitter, proof_url)
    }

    pub fn start_bounty_review(env: Env, bounty_id: u64, owner: Address) -> Result<(), Error> {
        BountyModule::start_review(&env, bounty_id, owner)
    }

    pub fn select_bounty_winner(
        env: Env,
        bounty_id: u64,
        owner: Address,
        winner: Address,
    ) -> Result<(), Error> {
        BountyModule::select_winner(&env, bounty_id, owner, winner)?;
        MetricsModule::increment_bounties_awarded(&env)?;
        Ok(())
    }

    pub fn cancel_bounty(env: Env, bounty_id: u64, owner: Address) -> Result<(), Error> {
        BountyModule::cancel_bounty(&env, bounty_id, owner)
    }

    pub fn get_bounty(env: Env, bounty_id: u64) -> Result<BountyGrant, Error> {
        BountyModule::get_bounty(&env, bounty_id)
    }

    pub fn get_bounty_submission(
        env: Env,
        bounty_id: u64,
        submitter: Address,
    ) -> Result<BountySubmission, Error> {
        BountyModule::get_submission(&env, bounty_id, submitter)
    }

    pub fn list_bounty_submitters(env: Env, bounty_id: u64) -> Result<Vec<Address>, Error> {
        BountyModule::list_submitters(&env, bounty_id)
    }
}
