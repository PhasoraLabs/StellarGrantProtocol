use soroban_sdk::{token, Address, Env, String, Vec};

use crate::circuit_breaker;
use crate::emergency;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    ContractError, CrowdfundCampaign, CrowdfundPledge, CrowdfundStatus, ProtocolModule,
};

/// Create a new crowdfunding campaign. Funds are held in the contract until
/// the deadline passes. If the target is met, the owner may call `finalize`
/// to receive the collected tokens. If the target is missed, backers may
/// call `claim_refund` to reclaim their pledges.
pub fn create_campaign(
    env: &Env,
    owner: &Address,
    title: String,
    description: String,
    token: &Address,
    target_amount: i128,
    deadline_ledgers: u32,
) -> Result<u64, ContractError> {
    emergency::require_not_paused(env)?;
    circuit_breaker::require_open(env, ProtocolModule::Crowdfund)?;

    if target_amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }
    if title.is_empty() {
        return Err(ContractError::InvalidInput);
    }
    if deadline_ledgers == 0 {
        return Err(ContractError::InvalidInput);
    }

    let now = env.ledger().timestamp();
    let deadline = now.saturating_add(deadline_ledgers as u64);
    let id = Storage::next_crowdfund_id(env);

    let campaign = CrowdfundCampaign {
        id,
        owner: owner.clone(),
        title: title.clone(),
        description,
        token: token.clone(),
        target_amount,
        total_pledged: 0,
        deadline,
        status: CrowdfundStatus::Active,
        created_at: now,
    };

    Storage::set_crowdfund_campaign(env, &campaign);
    Events::emit_crowdfund_created(env, id, owner.clone(), title, target_amount, deadline);

    Ok(id)
}

/// Pledge tokens to an active campaign. The caller must approve the contract
/// to transfer `amount` tokens before calling this function. Each address may
/// pledge multiple times; subsequent calls increase the existing pledge.
pub fn pledge(
    env: &Env,
    campaign_id: u64,
    backer: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    emergency::require_not_paused(env)?;
    circuit_breaker::require_open(env, ProtocolModule::Crowdfund)?;

    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let mut campaign = Storage::get_crowdfund_campaign(env, campaign_id)
        .ok_or(ContractError::CrowdfundNotFound)?;

    if campaign.status != CrowdfundStatus::Active {
        return Err(ContractError::CrowdfundNotActive);
    }
    if env.ledger().timestamp() > campaign.deadline {
        return Err(ContractError::DeadlinePassed);
    }

    token::Client::new(env, &campaign.token).transfer(
        backer,
        &env.current_contract_address(),
        &amount,
    );

    let existing = Storage::get_crowdfund_pledge(env, campaign_id, backer);
    let (new_amount, is_new_backer) = match existing {
        Some(mut p) => {
            p.amount = p.amount.saturating_add(amount);
            p.pledged_at = env.ledger().timestamp();
            Storage::set_crowdfund_pledge(env, &p);
            (p.amount, false)
        }
        None => {
            let pledge = CrowdfundPledge {
                campaign_id,
                backer: backer.clone(),
                amount,
                pledged_at: env.ledger().timestamp(),
                refunded: false,
            };
            Storage::set_crowdfund_pledge(env, &pledge);
            (amount, true)
        }
    };

    if is_new_backer {
        let mut backers = Storage::get_crowdfund_backers(env, campaign_id);
        backers.push_back(backer.clone());
        Storage::set_crowdfund_backers(env, campaign_id, &backers);
    }

    campaign.total_pledged = campaign.total_pledged.saturating_add(amount);
    Storage::set_crowdfund_campaign(env, &campaign);

    Events::emit_crowdfund_pledged(env, campaign_id, backer.clone(), amount, campaign.total_pledged);

    let _ = new_amount;
    Ok(())
}

/// Finalize a campaign once its deadline has passed. Anyone may call this.
///
/// - If `total_pledged >= target_amount`: marks the campaign as Succeeded and
///   transfers all pledged tokens to the campaign owner.
/// - If `total_pledged < target_amount`: marks the campaign as Failed so that
///   backers can call `claim_refund`.
pub fn finalize(env: &Env, campaign_id: u64) -> Result<CrowdfundStatus, ContractError> {
    let mut campaign = Storage::get_crowdfund_campaign(env, campaign_id)
        .ok_or(ContractError::CrowdfundNotFound)?;

    if campaign.status != CrowdfundStatus::Active {
        return Err(ContractError::CrowdfundAlreadyFinalized);
    }
    if env.ledger().timestamp() <= campaign.deadline {
        return Err(ContractError::CrowdfundDeadlineNotReached);
    }

    let new_status = if campaign.total_pledged >= campaign.target_amount {
        token::Client::new(env, &campaign.token).transfer(
            &env.current_contract_address(),
            &campaign.owner,
            &campaign.total_pledged,
        );
        Events::emit_crowdfund_succeeded(env, campaign_id, campaign.total_pledged);
        CrowdfundStatus::Succeeded
    } else {
        Events::emit_crowdfund_failed(env, campaign_id, campaign.total_pledged);
        CrowdfundStatus::Failed
    };

    campaign.status = new_status.clone();
    Storage::set_crowdfund_campaign(env, &campaign);

    Ok(new_status)
}

/// Claim a refund after a campaign has Failed or been Cancelled.
/// Each backer may only claim once.
pub fn claim_refund(
    env: &Env,
    campaign_id: u64,
    backer: &Address,
) -> Result<(), ContractError> {
    let campaign = Storage::get_crowdfund_campaign(env, campaign_id)
        .ok_or(ContractError::CrowdfundNotFound)?;

    if campaign.status != CrowdfundStatus::Failed
        && campaign.status != CrowdfundStatus::Cancelled
    {
        return Err(ContractError::CrowdfundNotActive);
    }

    let mut pledge = Storage::get_crowdfund_pledge(env, campaign_id, backer)
        .ok_or(ContractError::NoRefundableAmount)?;

    if pledge.refunded {
        return Err(ContractError::NoRefundableAmount);
    }
    if pledge.amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let refund_amount = pledge.amount;
    token::Client::new(env, &campaign.token).transfer(
        &env.current_contract_address(),
        backer,
        &refund_amount,
    );

    pledge.refunded = true;
    Storage::set_crowdfund_pledge(env, &pledge);

    Events::emit_crowdfund_refunded(env, campaign_id, backer.clone(), refund_amount);

    Ok(())
}

/// Cancel an Active campaign. Only the campaign owner may call this.
/// After cancellation, all backers may call `claim_refund` individually.
pub fn cancel(env: &Env, campaign_id: u64, caller: &Address) -> Result<(), ContractError> {
    let mut campaign = Storage::get_crowdfund_campaign(env, campaign_id)
        .ok_or(ContractError::CrowdfundNotFound)?;

    if campaign.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    if campaign.status != CrowdfundStatus::Active {
        return Err(ContractError::CrowdfundAlreadyFinalized);
    }

    campaign.status = CrowdfundStatus::Cancelled;
    Storage::set_crowdfund_campaign(env, &campaign);

    Events::emit_crowdfund_cancelled(env, campaign_id, caller.clone(), campaign.total_pledged);

    Ok(())
}

pub fn get_campaign(env: &Env, campaign_id: u64) -> Option<CrowdfundCampaign> {
    Storage::get_crowdfund_campaign(env, campaign_id)
}

pub fn get_pledge(env: &Env, campaign_id: u64, backer: &Address) -> Option<CrowdfundPledge> {
    Storage::get_crowdfund_pledge(env, campaign_id, backer)
}

pub fn list_backers(env: &Env, campaign_id: u64) -> Vec<Address> {
    Storage::get_crowdfund_backers(env, campaign_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn make_campaign(env: &Env, owner: &Address, token: &Address) -> u64 {
        let id = Storage::next_crowdfund_id(env);
        let campaign = CrowdfundCampaign {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Campaign"),
            description: String::from_str(env, "desc"),
            token: token.clone(),
            target_amount: 1_000,
            total_pledged: 0,
            deadline: env.ledger().timestamp() + 1_000,
            status: CrowdfundStatus::Active,
            created_at: env.ledger().timestamp(),
        };
        Storage::set_crowdfund_campaign(env, &campaign);
        id
    }

    #[test]
    fn test_pledge_to_unknown_campaign_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let backer = Address::generate(&env);
        let result = pledge(&env, 999, &backer, 100);
        assert_eq!(result, Err(ContractError::CrowdfundNotFound));
    }

    #[test]
    fn test_pledge_after_deadline_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let backer = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        env.ledger().with_mut(|l| l.timestamp += 2_000);
        let result = pledge(&env, id, &backer, 100);
        assert_eq!(result, Err(ContractError::DeadlinePassed));
    }

    #[test]
    fn test_finalize_before_deadline_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        let result = finalize(&env, id);
        assert_eq!(result, Err(ContractError::CrowdfundDeadlineNotReached));
    }

    #[test]
    fn test_finalize_underfunded_marks_failed() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        env.ledger().with_mut(|l| l.timestamp += 2_000);
        let status = finalize(&env, id).unwrap();
        assert_eq!(status, CrowdfundStatus::Failed);
        let campaign = get_campaign(&env, id).unwrap();
        assert_eq!(campaign.status, CrowdfundStatus::Failed);
    }

    #[test]
    fn test_double_finalize_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        env.ledger().with_mut(|l| l.timestamp += 2_000);
        finalize(&env, id).unwrap();
        let result = finalize(&env, id);
        assert_eq!(result, Err(ContractError::CrowdfundAlreadyFinalized));
    }

    #[test]
    fn test_claim_refund_on_active_campaign_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let backer = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        let result = claim_refund(&env, id, &backer);
        assert_eq!(result, Err(ContractError::CrowdfundNotActive));
    }

    #[test]
    fn test_claim_refund_with_no_pledge_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let backer = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        env.ledger().with_mut(|l| l.timestamp += 2_000);
        finalize(&env, id).unwrap();

        let result = claim_refund(&env, id, &backer);
        assert_eq!(result, Err(ContractError::NoRefundableAmount));
    }

    #[test]
    fn test_cancel_by_non_owner_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let stranger = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        let result = cancel(&env, id, &stranger);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_cancel_marks_status() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        cancel(&env, id, &owner).unwrap();
        let campaign = get_campaign(&env, id).unwrap();
        assert_eq!(campaign.status, CrowdfundStatus::Cancelled);
    }

    #[test]
    fn test_list_backers_empty_initially() {
        let env = Env::default();
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let token = Address::generate(&env);
        let id = make_campaign(&env, &owner, &token);

        let backers = list_backers(&env, id);
        assert_eq!(backers.len(), 0);
    }
}
