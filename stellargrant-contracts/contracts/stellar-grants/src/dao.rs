use soroban_sdk::{Address, Env, String};

use crate::config;
use crate::constants::{DEFAULT_DAO_VOTING_PERIOD_LEDGERS, MAX_DAO_DESCRIPTION_LEN, MAX_DAO_TITLE_LEN};
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{ContractError, DaoProposal, DaoProposalStatus, DaoProposalType};

/// Enable or disable DAO governance mode. While enabled, protocol-level
/// config/admin changes must be routed through a passed-and-executed
/// proposal rather than applied directly by the global admin.
pub fn set_dao_mode(env: &Env, admin: &Address, enabled: bool) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;
    Storage::set_dao_mode_enabled(env, enabled);
    Ok(())
}

pub fn is_dao_mode_enabled(env: &Env) -> bool {
    Storage::is_dao_mode_enabled(env)
}

/// Configure the voting period (in ledgers) applied to newly created proposals.
pub fn set_voting_period(env: &Env, admin: &Address, ledgers: u32) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;
    if ledgers == 0 {
        return Err(ContractError::InvalidInput);
    }
    Storage::set_dao_voting_period_ledgers(env, ledgers);
    Ok(())
}

/// Configure the minimum total vote weight (for + against) required before a
/// proposal can be finalized.
pub fn set_quorum_votes(env: &Env, admin: &Address, quorum: u64) -> Result<(), ContractError> {
    require_global_admin(env, admin)?;
    if quorum == 0 {
        return Err(ContractError::InvalidInput);
    }
    Storage::set_dao_quorum_votes(env, quorum);
    Ok(())
}

/// Submit a new governance proposal. Any registered contributor/reviewer
/// (anyone holding non-zero reputation) may propose.
pub fn create_proposal(
    env: &Env,
    proposer: &Address,
    title: String,
    description: String,
    proposal_type: DaoProposalType,
) -> Result<u64, ContractError> {
    if title.is_empty() || title.len() > MAX_DAO_TITLE_LEN {
        return Err(ContractError::InvalidInput);
    }
    if description.len() > MAX_DAO_DESCRIPTION_LEN {
        return Err(ContractError::InvalidInput);
    }

    let id = Storage::next_dao_proposal_id(env);
    let now = env.ledger().timestamp();
    let voting_period = Storage::get_dao_voting_period_ledgers(env);
    let voting_deadline = now.saturating_add(voting_period as u64);

    let proposal = DaoProposal {
        id,
        proposer: proposer.clone(),
        title: title.clone(),
        description,
        proposal_type,
        status: DaoProposalStatus::Active,
        votes_for: 0,
        votes_against: 0,
        created_at: now,
        voting_deadline,
    };

    Storage::set_dao_proposal(env, &proposal);
    Events::emit_dao_proposal_created(env, id, proposer.clone(), title, voting_deadline);

    Ok(id)
}

/// Cast a reputation-weighted vote on an active proposal. One vote per address.
pub fn vote(
    env: &Env,
    voter: &Address,
    proposal_id: u64,
    support: bool,
) -> Result<DaoProposal, ContractError> {
    let mut proposal =
        Storage::get_dao_proposal(env, proposal_id).ok_or(ContractError::DaoProposalNotFound)?;

    if proposal.status != DaoProposalStatus::Active {
        return Err(ContractError::DaoProposalNotActive);
    }
    if env.ledger().timestamp() > proposal.voting_deadline {
        return Err(ContractError::DaoProposalVotingClosed);
    }
    if Storage::has_dao_voted(env, proposal_id, voter) {
        return Err(ContractError::AlreadyVoted);
    }

    let weight = Storage::get_reviewer_reputation(env, voter.clone()) as u64;

    if support {
        proposal.votes_for = proposal.votes_for.saturating_add(weight);
    } else {
        proposal.votes_against = proposal.votes_against.saturating_add(weight);
    }

    Storage::record_dao_vote(env, proposal_id, voter);
    Storage::set_dao_proposal(env, &proposal);

    Events::emit_dao_vote_cast(env, proposal_id, voter.clone(), support, weight);

    Ok(proposal)
}

/// Finalize a proposal once its voting deadline has passed: marks it Passed
/// or Rejected based on simple majority, provided quorum was met.
pub fn finalize(env: &Env, proposal_id: u64) -> Result<DaoProposalStatus, ContractError> {
    let mut proposal =
        Storage::get_dao_proposal(env, proposal_id).ok_or(ContractError::DaoProposalNotFound)?;

    if proposal.status != DaoProposalStatus::Active {
        return Err(ContractError::DaoProposalNotActive);
    }
    if env.ledger().timestamp() <= proposal.voting_deadline {
        return Err(ContractError::DaoProposalVotingClosed);
    }

    let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
    if total_votes < Storage::get_dao_quorum_votes(env) {
        return Err(ContractError::DaoProposalQuorumNotReached);
    }

    let passed = proposal.votes_for > proposal.votes_against;
    proposal.status = if passed {
        DaoProposalStatus::Passed
    } else {
        DaoProposalStatus::Rejected
    };
    Storage::set_dao_proposal(env, &proposal);

    Events::emit_dao_proposal_finalized(
        env,
        proposal_id,
        passed,
        proposal.votes_for,
        proposal.votes_against,
    );

    Ok(proposal.status.clone())
}

/// Execute a passed proposal's on-chain effect (config update, admin change,
/// or treasury withdrawal). Anyone may trigger execution once a proposal has
/// passed — the payload itself is the authorization.
pub fn execute(env: &Env, executor: &Address, proposal_id: u64) -> Result<(), ContractError> {
    let mut proposal =
        Storage::get_dao_proposal(env, proposal_id).ok_or(ContractError::DaoProposalNotFound)?;

    if proposal.status != DaoProposalStatus::Passed {
        return Err(ContractError::DaoProposalRejected);
    }

    match &proposal.proposal_type {
        DaoProposalType::UpdateConfig(new_config) => {
            config::validate_config(new_config)?;
            Storage::set_protocol_config(env, new_config);
        }
        DaoProposalType::ChangeAdmin(new_admin) => {
            Storage::set_global_admin(env, new_admin);
        }
        DaoProposalType::TreasuryWithdrawal(token, to, amount) => {
            crate::treasury::withdraw(
                env,
                &Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?,
                token,
                to,
                *amount,
            )?;
        }
        DaoProposalType::Generic => {}
    }

    proposal.status = DaoProposalStatus::Executed;
    proposal.executed_at = Some(env.ledger().timestamp());
    Storage::set_dao_proposal(env, &proposal);

    Events::emit_dao_proposal_executed(env, proposal_id, executor.clone());

    Ok(())
}

/// Cancel a proposal before it is finalized. Only the original proposer or
/// the global admin may cancel.
pub fn cancel(env: &Env, caller: &Address, proposal_id: u64) -> Result<(), ContractError> {
    let mut proposal =
        Storage::get_dao_proposal(env, proposal_id).ok_or(ContractError::DaoProposalNotFound)?;

    if proposal.status != DaoProposalStatus::Active {
        return Err(ContractError::DaoProposalNotActive);
    }
    if proposal.proposer != *caller && Storage::get_global_admin(env) != Some(caller.clone()) {
        return Err(ContractError::Unauthorized);
    }

    proposal.status = DaoProposalStatus::Cancelled;
    Storage::set_dao_proposal(env, &proposal);

    Events::emit_dao_proposal_cancelled(env, proposal_id, caller.clone());

    Ok(())
}

pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<DaoProposal> {
    Storage::get_dao_proposal(env, proposal_id)
}

/// Require that DAO mode is disabled — used to gate the legacy admin-direct
/// config/admin-change entry points once governance takes over.
pub fn require_dao_mode_disabled(env: &Env) -> Result<(), ContractError> {
    if Storage::is_dao_mode_enabled(env) {
        return Err(ContractError::DaoModeDisabled);
    }
    Ok(())
}

fn require_global_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    let admin = Storage::get_global_admin(env).ok_or(ContractError::Unauthorized)?;
    if admin != *caller {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup(env: &Env) -> Address {
        let admin = Address::generate(env);
        Storage::set_global_admin(env, &admin);
        admin
    }

    #[test]
    fn test_create_proposal_empty_title_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let proposer = Address::generate(&env);
        let result = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, ""),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        );
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_create_proposal_success_starts_active() {
        let env = Env::default();
        env.mock_all_auths();
        let proposer = Address::generate(&env);
        let id = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, "Add new feature"),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        )
        .unwrap();
        let proposal = get_proposal(&env, id).unwrap();
        assert_eq!(proposal.status, DaoProposalStatus::Active);
        assert_eq!(proposal.votes_for, 0);
    }

    #[test]
    fn test_vote_twice_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let proposer = Address::generate(&env);
        let voter = Address::generate(&env);
        let id = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, "Title"),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        )
        .unwrap();

        vote(&env, &voter, id, true).unwrap();
        let result = vote(&env, &voter, id, true);
        assert_eq!(result, Err(ContractError::AlreadyVoted));
    }

    #[test]
    fn test_vote_on_unknown_proposal_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let voter = Address::generate(&env);
        let result = vote(&env, &voter, 999, true);
        assert_eq!(result, Err(ContractError::DaoProposalNotFound));
    }

    #[test]
    fn test_finalize_before_deadline_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let proposer = Address::generate(&env);
        let id = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, "Title"),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        )
        .unwrap();
        let result = finalize(&env, id);
        assert_eq!(result, Err(ContractError::DaoProposalVotingClosed));
    }

    #[test]
    fn test_finalize_quorum_not_reached_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let proposer = Address::generate(&env);
        let id = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, "Title"),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        )
        .unwrap();

        env.ledger().with_mut(|l| {
            l.timestamp += DEFAULT_DAO_VOTING_PERIOD_LEDGERS as u64 + 1;
        });

        let result = finalize(&env, id);
        assert_eq!(result, Err(ContractError::DaoProposalQuorumNotReached));
    }

    #[test]
    fn test_execute_non_passed_proposal_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let proposer = Address::generate(&env);
        let id = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, "Title"),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        )
        .unwrap();
        let result = execute(&env, &proposer, id);
        assert_eq!(result, Err(ContractError::DaoProposalRejected));
    }

    #[test]
    fn test_cancel_by_non_proposer_non_admin_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        setup(&env);
        let proposer = Address::generate(&env);
        let stranger = Address::generate(&env);
        let id = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, "Title"),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        )
        .unwrap();
        let result = cancel(&env, &stranger, id);
        assert_eq!(result, Err(ContractError::Unauthorized));
    }

    #[test]
    fn test_cancel_by_proposer_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let proposer = Address::generate(&env);
        let id = create_proposal(
            &env,
            &proposer,
            String::from_str(&env, "Title"),
            String::from_str(&env, "desc"),
            DaoProposalType::Generic,
        )
        .unwrap();
        cancel(&env, &proposer, id).unwrap();
        let proposal = get_proposal(&env, id).unwrap();
        assert_eq!(proposal.status, DaoProposalStatus::Cancelled);
    }

    #[test]
    fn test_set_dao_mode_requires_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = setup(&env);
        let stranger = Address::generate(&env);
        assert_eq!(
            set_dao_mode(&env, &stranger, true),
            Err(ContractError::Unauthorized)
        );
        set_dao_mode(&env, &admin, true).unwrap();
        assert!(is_dao_mode_enabled(&env));
    }

    #[test]
    fn test_require_dao_mode_disabled_passes_when_off() {
        let env = Env::default();
        assert!(require_dao_mode_disabled(&env).is_ok());
    }

    #[test]
    fn test_require_dao_mode_disabled_fails_when_on() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = setup(&env);
        set_dao_mode(&env, &admin, true).unwrap();
        assert_eq!(
            require_dao_mode_disabled(&env),
            Err(ContractError::DaoModeDisabled)
        );
    }
}
