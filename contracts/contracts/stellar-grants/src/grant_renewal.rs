use crate::storage::Storage;
use crate::types::{ContractError, GrantStatus, RenewalProposal, RenewalStatus};
use soroban_sdk::{Address, Env, String, Vec};

pub fn propose_renewal(
    env: &Env,
    proposer: &Address,
    original_grant_id: u64,
    new_title: String,
    new_description: String,
    new_total_amount: i128,
    new_num_milestones: u32,
    inherit_reviewers: bool,
    inherit_contributor: bool,
    ttl_ledgers: u32,
) -> Result<(), ContractError> {
    proposer.require_auth();

    let grant = Storage::get_grant_v(env, original_grant_id);

    if grant.owner != *proposer && !grant.reviewers.contains(proposer.clone()) {
        return Err(ContractError::Unauthorized);
    }

    if grant.status != GrantStatus::Completed
        && grant.milestones_paid_out < grant.total_milestones - 1
    {
        return Err(ContractError::InvalidState);
    }

    let proposal = RenewalProposal {
        original_grant_id,
        proposed_by: proposer.clone(),
        new_title,
        new_description,
        new_total_amount,
        new_num_milestones,
        inherit_reviewers,
        inherit_contributor,
        status: RenewalStatus::Proposed,
        reviewer_votes: 0,
        proposed_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (ttl_ledgers as u64 * 5),
        new_grant_id: None,
    };

    Storage::set_renewal_proposal(env, &proposal);
    Ok(())
}

pub fn approve_renewal(
    env: &Env,
    reviewer: &Address,
    original_grant_id: u64,
) -> Result<RenewalStatus, ContractError> {
    reviewer.require_auth();

    let grant = Storage::get_grant_v(env, original_grant_id);
    if !grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let mut proposal =
        Storage::get_renewal_proposal(env, original_grant_id).ok_or(ContractError::InvalidState)?;

    if proposal.status != RenewalStatus::Proposed {
        return Err(ContractError::InvalidState);
    }

    if env.ledger().timestamp() > proposal.expires_at {
        return Err(ContractError::InvalidState);
    }

    proposal.reviewer_votes = proposal
        .reviewer_votes
        .checked_add(1)
        .ok_or(ContractError::InvalidInput)?;
    proposal.status = RenewalStatus::ReviewerApproved;
    Storage::set_renewal_proposal(env, &proposal);
    Ok(proposal.status)
}

pub fn activate_renewal(
    env: &Env,
    owner: &Address,
    original_grant_id: u64,
) -> Result<u64, ContractError> {
    owner.require_auth();

    let mut proposal =
        Storage::get_renewal_proposal(env, original_grant_id).ok_or(ContractError::InvalidState)?;

    if proposal.status != RenewalStatus::ReviewerApproved {
        return Err(ContractError::InvalidState);
    }

    let original_grant = Storage::get_grant_v(env, original_grant_id);
    if original_grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    let reviewers = if proposal.inherit_reviewers {
        original_grant.reviewers.clone()
    } else {
        Vec::new(env)
    };

    let new_grant_id = crate::internal_grant_create(
        env,
        owner,
        proposal.new_title.clone(),
        proposal.new_description.clone(),
        &original_grant.token,
        proposal.new_total_amount,
        proposal
            .new_total_amount
            .checked_div(proposal.new_num_milestones as i128)
            .ok_or(ContractError::InvalidInput)?,
        proposal.new_num_milestones,
        reviewers,
    )?;

    proposal.status = RenewalStatus::Active;
    proposal.new_grant_id = Some(new_grant_id);
    Storage::set_renewal_proposal(env, &proposal);
    Storage::set_renewal_history(env, new_grant_id, original_grant_id);

    Ok(new_grant_id)
}

pub fn decline_renewal(
    env: &Env,
    caller: &Address,
    original_grant_id: u64,
) -> Result<(), ContractError> {
    caller.require_auth();

    let mut proposal =
        Storage::get_renewal_proposal(env, original_grant_id).ok_or(ContractError::InvalidState)?;

    if proposal.status == RenewalStatus::Declined || proposal.status == RenewalStatus::Expired {
        return Err(ContractError::InvalidState);
    }

    let grant = Storage::get_grant_v(env, original_grant_id);
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }

    proposal.status = RenewalStatus::Declined;
    Storage::set_renewal_proposal(env, &proposal);
    Ok(())
}

pub fn get_proposal(env: &Env, original_grant_id: u64) -> Option<RenewalProposal> {
    Storage::get_renewal_proposal(env, original_grant_id)
}

pub fn renewal_chain(env: &Env, original_grant_id: u64) -> soroban_sdk::Vec<u64> {
    let mut chain = soroban_sdk::Vec::new(env);
    chain.push_back(original_grant_id);

    let mut current_id = original_grant_id;
    while let Some(proposal) = Storage::get_renewal_proposal(env, current_id) {
        if let Some(new_id) = proposal.new_grant_id {
            chain.push_back(new_id);
            current_id = new_id;
        } else {
            break;
        }
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantStatus, WhitelistMode, WhitelistScope};
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env};

    struct Ctx {
        env: Env,
        cid: Address,
        owner: Address,
        reviewer: Address,
    }

    fn make_grant(env: &Env, owner: &Address, reviewer: &Address, completed: bool) -> Grant {
        let mut reviewers = Vec::new(env);
        reviewers.push_back(reviewer.clone());
        Grant {
            id: 1,
            owner: owner.clone(),
            title: String::from_str(env, "Orig"),
            description: String::from_str(env, "Orig desc"),
            token: Address::generate(env),
            status: if completed {
                GrantStatus::Completed
            } else {
                GrantStatus::Active
            },
            total_amount: 1_000,
            milestone_amount: 500,
            reviewers,
            total_milestones: 2,
            milestones_paid_out: if completed { 2 } else { 0 },
            escrow_balance: 0,
            funders: Vec::new(env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        }
    }

    fn setup(completed: bool) -> Ctx {
        let env = Env::default();
        env.mock_all_auths();
        let cid = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        env.as_contract(&cid, || {
            // Claim id 1 from the counter so the seeded original grant matches
            // what internal_grant_create would have produced.
            assert_eq!(Storage::increment_grant_counter(&env), 1);
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, &reviewer, completed));
        });
        Ctx {
            env,
            cid,
            owner,
            reviewer,
        }
    }

    fn propose(
        ctx: &Ctx,
        proposer: &Address,
        inherit_reviewers: bool,
    ) -> Result<(), ContractError> {
        let env = &ctx.env;
        let proposer = proposer.clone();
        ctx.env.as_contract(&ctx.cid, || {
            propose_renewal(
                env,
                &proposer,
                1,
                String::from_str(env, "Renewed"),
                String::from_str(env, "Renewed desc"),
                1_000,
                2,
                inherit_reviewers,
                false,
                100,
            )
        })
    }

    fn approve(ctx: &Ctx, reviewer: &Address) -> Result<RenewalStatus, ContractError> {
        let env = &ctx.env;
        let reviewer = reviewer.clone();
        ctx.env
            .as_contract(&ctx.cid, || approve_renewal(env, &reviewer, 1))
    }

    fn activate(ctx: &Ctx, owner: &Address) -> Result<u64, ContractError> {
        let env = &ctx.env;
        let owner = owner.clone();
        ctx.env
            .as_contract(&ctx.cid, || activate_renewal(env, &owner, 1))
    }

    // ── happy path ─────────────────────────────────────────────────────────

    #[test]
    fn propose_approve_activate_creates_a_real_new_grant() {
        let ctx = setup(true);
        propose(&ctx, &ctx.owner, false).unwrap();
        assert_eq!(
            approve(&ctx, &ctx.reviewer).unwrap(),
            RenewalStatus::ReviewerApproved
        );

        let new_id = activate(&ctx, &ctx.owner).unwrap();
        assert_ne!(new_id, 1);

        ctx.env.as_contract(&ctx.cid, || {
            let new_grant = Storage::get_grant(&ctx.env, new_id).unwrap();
            assert_eq!(new_grant.owner, ctx.owner);
            assert_eq!(new_grant.total_amount, 1_000);
            assert_eq!(new_grant.status, GrantStatus::Active);

            let proposal = get_proposal(&ctx.env, 1).unwrap();
            assert_eq!(proposal.status, RenewalStatus::Active);
            assert_eq!(proposal.new_grant_id, Some(new_id));

            // renewal_chain links original -> new grant.
            let chain = renewal_chain(&ctx.env, 1);
            assert_eq!(chain.len(), 2);
            assert_eq!(chain.get(0).unwrap(), 1);
            assert_eq!(chain.get(1).unwrap(), new_id);
        });
    }

    // ── authorization guards ───────────────────────────────────────────────

    #[test]
    fn propose_renewal_rejects_unrelated_caller() {
        let ctx = setup(true);
        let stranger = Address::generate(&ctx.env);
        assert_eq!(
            propose(&ctx, &stranger, false),
            Err(ContractError::Unauthorized)
        );
    }

    #[test]
    fn propose_renewal_rejects_grant_not_near_completion() {
        // Active grant, 0 of 2 milestones paid out -> not eligible.
        let ctx = setup(false);
        assert_eq!(
            propose(&ctx, &ctx.owner, false),
            Err(ContractError::InvalidState)
        );
    }

    #[test]
    fn approve_renewal_rejects_non_reviewer() {
        let ctx = setup(true);
        propose(&ctx, &ctx.owner, false).unwrap();
        let stranger = Address::generate(&ctx.env);
        assert_eq!(approve(&ctx, &stranger), Err(ContractError::Unauthorized));
    }

    #[test]
    fn approve_renewal_rejects_expired_proposal() {
        let ctx = setup(true);
        propose(&ctx, &ctx.owner, false).unwrap();
        // ttl_ledgers = 100 -> expires_at = now + 500.
        ctx.env.ledger().set_timestamp(10_000);
        assert_eq!(
            approve(&ctx, &ctx.reviewer),
            Err(ContractError::InvalidState)
        );
    }

    #[test]
    fn decline_renewal_rejects_non_owner() {
        let ctx = setup(true);
        propose(&ctx, &ctx.owner, false).unwrap();
        let stranger = Address::generate(&ctx.env);
        let env = &ctx.env;
        let s = stranger.clone();
        let res = ctx
            .env
            .as_contract(&ctx.cid, || decline_renewal(env, &s, 1));
        assert_eq!(res, Err(ContractError::Unauthorized));

        let o = ctx.owner.clone();
        ctx.env
            .as_contract(&ctx.cid, || decline_renewal(env, &o, 1))
            .unwrap();
        ctx.env.as_contract(&ctx.cid, || {
            assert_eq!(
                get_proposal(&ctx.env, 1).unwrap().status,
                RenewalStatus::Declined
            );
        });
    }

    // ── inherit_reviewers vs. whitelist (issue #971) ───────────────────────

    #[test]
    fn activate_with_inherit_reviewers_rechecks_whitelist_and_rejects_removed_reviewer() {
        let ctx = setup(true);
        propose(&ctx, &ctx.owner, true).unwrap();
        approve(&ctx, &ctx.reviewer).unwrap();

        // The original reviewer is no longer allowed under a now-restricted
        // GlobalReviewer scope; internal_grant_create must re-validate the
        // inherited reviewer list and refuse.
        ctx.env.as_contract(&ctx.cid, || {
            Storage::set_whitelist_mode(
                &ctx.env,
                &WhitelistScope::GlobalReviewer,
                WhitelistMode::Restricted,
            );
        });

        assert_eq!(
            activate(&ctx, &ctx.owner),
            Err(ContractError::AddressNotWhitelisted)
        );
    }

    #[test]
    fn activate_with_inherit_reviewers_succeeds_when_reviewer_still_whitelisted() {
        // Default GlobalReviewer mode is Open -> inherited reviewer passes.
        let ctx = setup(true);
        propose(&ctx, &ctx.owner, true).unwrap();
        approve(&ctx, &ctx.reviewer).unwrap();

        let new_id = activate(&ctx, &ctx.owner).unwrap();
        ctx.env.as_contract(&ctx.cid, || {
            let new_grant = Storage::get_grant(&ctx.env, new_id).unwrap();
            assert_eq!(new_grant.reviewers.len(), 1);
            assert_eq!(new_grant.reviewers.get(0).unwrap(), ctx.reviewer);
        });
    }
}
