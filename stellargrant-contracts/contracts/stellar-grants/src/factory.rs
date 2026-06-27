use soroban_sdk::{Address, Env, String, Vec};

use crate::constants::LEDGERS_PER_DAY;
use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{GrantArchetype, GrantTemplate, VotingMechanism};

/// Return the default GrantTemplate for a given archetype.
pub fn template_for(archetype: GrantArchetype) -> GrantTemplate {
    match archetype {
        GrantArchetype::ResearchGrant => GrantTemplate {
            archetype: GrantArchetype::ResearchGrant,
            num_milestones: 4,
            review_window_ledgers: LEDGERS_PER_DAY * 30,
            min_reviewers: 3,
            quorum_threshold_bps: 6_667,
            voting_mechanism: VotingMechanism::SimpleMajority,
            requires_staking: false,
            multisig_required: false,
            sequential_milestones: true,
            insurance_opt_in: false,
        },
        GrantArchetype::DevelopmentBounty => GrantTemplate {
            archetype: GrantArchetype::DevelopmentBounty,
            num_milestones: 6,
            review_window_ledgers: LEDGERS_PER_DAY * 14,
            min_reviewers: 2,
            quorum_threshold_bps: 5_001,
            voting_mechanism: VotingMechanism::Weighted,
            requires_staking: true,
            multisig_required: false,
            sequential_milestones: true,
            insurance_opt_in: false,
        },
        GrantArchetype::CommunityProject => GrantTemplate {
            archetype: GrantArchetype::CommunityProject,
            num_milestones: 3,
            review_window_ledgers: LEDGERS_PER_DAY * 7,
            min_reviewers: 1,
            quorum_threshold_bps: 5_001,
            voting_mechanism: VotingMechanism::SimpleMajority,
            requires_staking: false,
            multisig_required: false,
            sequential_milestones: false,
            insurance_opt_in: false,
        },
        GrantArchetype::ProtocolIntegration => GrantTemplate {
            archetype: GrantArchetype::ProtocolIntegration,
            num_milestones: 5,
            review_window_ledgers: LEDGERS_PER_DAY * 21,
            min_reviewers: 4,
            quorum_threshold_bps: 7_500,
            voting_mechanism: VotingMechanism::Weighted,
            requires_staking: false,
            multisig_required: true,
            sequential_milestones: true,
            insurance_opt_in: true,
        },
        GrantArchetype::CustomTemplate => GrantTemplate {
            archetype: GrantArchetype::CustomTemplate,
            num_milestones: 1,
            review_window_ledgers: LEDGERS_PER_DAY,
            min_reviewers: 1,
            quorum_threshold_bps: 5_001,
            voting_mechanism: VotingMechanism::SimpleMajority,
            requires_staking: false,
            multisig_required: false,
            sequential_milestones: true,
            insurance_opt_in: false,
        },
    }
}

/// Create a grant using a predefined archetype template.
pub fn create_from_template(
    env: &Env,
    owner: &Address,
    archetype: GrantArchetype,
    title: String,
    description: String,
    token: &Address,
    total_amount: i128,
    reviewers: Vec<Address>,
) -> Result<u64, ContractError> {
    let template = template_for(archetype);
    create_from_custom_template(
        env,
        owner,
        template,
        title,
        description,
        token,
        total_amount,
        reviewers,
    )
}

/// Create a grant using a custom GrantTemplate.
pub fn create_from_custom_template(
    env: &Env,
    owner: &Address,
    template: GrantTemplate,
    title: String,
    description: String,
    token: &Address,
    total_amount: i128,
    reviewers: Vec<Address>,
) -> Result<u64, ContractError> {
    validate_template(&template)?;

    if reviewers.len() < template.min_reviewers {
        return Err(ContractError::InvalidInput);
    }

    if total_amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let milestone_amount = total_amount
        .checked_div(template.num_milestones as i128)
        .ok_or(ContractError::InvalidInput)?;
    if milestone_amount <= 0 {
        return Err(ContractError::InvalidInput);
    }

    let grant_id = crate::internal_grant_create(
        env,
        owner,
        title,
        description,
        token,
        total_amount,
        milestone_amount,
        template.num_milestones,
        reviewers,
    )?;

    Storage::set_voting_mechanism(env, grant_id, &template.voting_mechanism);
    Ok(grant_id)
}

/// Validate that a GrantTemplate has self-consistent values.
pub fn validate_template(template: &GrantTemplate) -> Result<(), ContractError> {
    if template.num_milestones < 1 {
        return Err(ContractError::InvalidInput);
    }
    if template.quorum_threshold_bps <= 5_000 || template.quorum_threshold_bps > 10_000 {
        return Err(ContractError::InvalidInput);
    }
    if template.min_reviewers < 1 {
        return Err(ContractError::InvalidInput);
    }
    if template.review_window_ledgers == 0 {
        return Err(ContractError::InvalidInput);
    }
    Ok(())
}

/// Return a list of all available archetypes with their template defaults.
pub fn list_archetypes(env: &Env) -> Vec<GrantTemplate> {
    let mut templates = Vec::new(env);
    templates.push_back(template_for(GrantArchetype::ResearchGrant));
    templates.push_back(template_for(GrantArchetype::DevelopmentBounty));
    templates.push_back(template_for(GrantArchetype::CommunityProject));
    templates.push_back(template_for(GrantArchetype::ProtocolIntegration));
    templates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, String, Vec};

    fn with_contract<F, R>(env: &Env, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let contract_id = env.register(StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            Storage::set_protocol_config(env, &config::default_config());
            f()
        })
    }

    #[test]
    fn test_archetypes_have_distinct_defaults() {
        let research = template_for(GrantArchetype::ResearchGrant);
        let bounty = template_for(GrantArchetype::DevelopmentBounty);
        let community = template_for(GrantArchetype::CommunityProject);
        let protocol = template_for(GrantArchetype::ProtocolIntegration);

        assert_eq!(research.num_milestones, 4);
        assert_eq!(bounty.num_milestones, 6);
        assert_eq!(community.num_milestones, 3);
        assert_eq!(protocol.num_milestones, 5);
        assert_ne!(research.min_reviewers, protocol.min_reviewers);
        assert!(bounty.requires_staking);
        assert!(protocol.multisig_required);
    }

    #[test]
    fn test_invalid_template_rejected() {
        let mut template = template_for(GrantArchetype::CommunityProject);
        template.quorum_threshold_bps = 4_000;
        assert_eq!(
            validate_template(&template),
            Err(ContractError::InvalidInput)
        );
    }

    #[test]
    fn test_insufficient_reviewers_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);
            let reviewers = Vec::new(&env);

            let err = create_from_template(
                &env,
                &owner,
                GrantArchetype::ResearchGrant,
                String::from_str(&env, "Research"),
                String::from_str(&env, "Desc"),
                &token,
                400,
                reviewers,
            )
            .unwrap_err();
            assert_eq!(err, ContractError::InvalidInput);
        });
    }

    #[test]
    fn test_research_template_creates_grant() {
        let env = Env::default();
        env.mock_all_auths();
        with_contract(&env, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);
            let mut reviewers = Vec::new(&env);
            reviewers.push_back(Address::generate(&env));
            reviewers.push_back(Address::generate(&env));
            reviewers.push_back(Address::generate(&env));

            let grant_id = create_from_template(
                &env,
                &owner,
                GrantArchetype::ResearchGrant,
                String::from_str(&env, "Research"),
                String::from_str(&env, "Desc"),
                &token,
                400,
                reviewers,
            )
            .unwrap();
            assert_eq!(grant_id, 1);

            let grant = Storage::get_grant(&env, grant_id).unwrap();
            assert_eq!(grant.total_milestones, 4);
        });
    }
}
