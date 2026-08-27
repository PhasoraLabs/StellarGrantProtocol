use crate::events::LicenseAttached;
use crate::storage::Storage;
use crate::types::{ContractError, IpRights, LicenseRecord, LicenseType, MilestoneState};
use soroban_sdk::{Address, Env, String};

/// Attach a license record to an approved milestone deliverable.
/// Only the grant owner may call this. The milestone must already exist.
pub fn attach_license(
    env: &Env,
    caller: &Address,
    grant_id: u64,
    milestone_idx: u32,
    spdx_id: String,
    license_type: LicenseType,
    rights: IpRights,
    restrictions: String,
) -> Result<LicenseRecord, ContractError> {
    caller.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *caller {
        return Err(ContractError::Unauthorized);
    }
    if milestone_idx >= grant.total_milestones {
        return Err(ContractError::MilestoneIndexOutOfBounds);
    }
    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;
    if milestone.state != MilestoneState::Approved {
        return Err(ContractError::InvalidState);
    }

    let record = LicenseRecord {
        grant_id,
        milestone_idx,
        spdx_id,
        license_type,
        rights,
        restrictions,
        attached_by: caller.clone(),
        attached_at: env.ledger().timestamp(),
    };
    Storage::set_license_record(env, grant_id, milestone_idx, &record);
    LicenseAttached {
        grant_id,
        milestone_idx,
        spdx_id: record.spdx_id.clone(),
        attached_by: caller.clone(),
    }
    .publish(env);
    Ok(record)
}

pub fn get_license(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<LicenseRecord> {
    Storage::get_license_record(env, grant_id, milestone_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{
        Grant, GrantStatus, Milestone, MilestoneState,
    };
    use soroban_sdk::{testutils::Address as _, Address, Env, Map, String, Vec};

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let admin = Address::generate(&env);
        (env, contract_id, owner, admin)
    }

    fn create_grant_with_milestone(env: &Env, owner: &Address, grant_id: u64) {
        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Desc"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 2,
            milestones_paid_out: 0,
            escrow_balance: 1000,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);

        let milestone = Milestone {
            idx: 0,
            description: String::from_str(env, "MS1"),
            amount: 500,
            state: MilestoneState::Approved,
            votes: Map::new(env),
            approvals: 1,
            rejections: 0,
            reasons: Map::new(env),
            status_updated_at: 0,
            proof_url: Some(String::from_str(env, "https://proof")),
            submission_timestamp: env.ledger().timestamp(),
            deadline: None,
            reviewer_count_snapshot: 1,
        };
        Storage::set_milestone(env, grant_id, 0, &milestone);
    }

    fn create_grant_with_milestone_in_state(
        env: &Env,
        owner: &Address,
        grant_id: u64,
        state: MilestoneState,
    ) {
        create_grant_with_milestone(env, owner, grant_id);
        let mut milestone = Storage::get_milestone(env, grant_id, 0).unwrap();
        milestone.state = state;
        Storage::set_milestone(env, grant_id, 0, &milestone);
    }

    #[test]
    fn test_attach_and_get_license() {
        let (env, contract_id, owner, _) = setup();
        env.as_contract(&contract_id, || {
            create_grant_with_milestone(&env, &owner, 1);

            let rights = IpRights {
                commercial_use: true,
                modification: false,
                distribution: true,
                sublicense: false,
            };

            let record = attach_license(
                &env,
                &owner,
                1,
                0,
                String::from_str(&env, "MIT"),
                LicenseType::OpenSource,
                rights,
                String::from_str(&env, "No restrictions"),
            )
            .unwrap();

            assert_eq!(record.grant_id, 1);
            assert_eq!(record.milestone_idx, 0);
            assert_eq!(record.license_type, LicenseType::OpenSource);
            assert!(record.rights.commercial_use);
            assert!(!record.rights.modification);

            let fetched = get_license(&env, 1, 0).unwrap();
            assert_eq!(fetched, record);
        });
    }

    #[test]
    fn test_get_license_returns_none_when_not_set() {
        let (env, contract_id, _, _) = setup();
        env.as_contract(&contract_id, || {
            assert!(get_license(&env, 999, 0).is_none());
        });
    }

    #[test]
    fn test_attach_license_unauthorized() {
        let (env, contract_id, owner, _) = setup();
        env.as_contract(&contract_id, || {
            create_grant_with_milestone(&env, &owner, 1);

            let other = Address::generate(&env);
            let rights = IpRights {
                commercial_use: false,
                modification: false,
                distribution: false,
                sublicense: false,
            };

            let res = attach_license(
                &env,
                &other,
                1,
                0,
                String::from_str(&env, "MIT"),
                LicenseType::OpenSource,
                rights,
                String::from_str(&env, ""),
            );
            assert_eq!(res, Err(ContractError::Unauthorized));
        });
    }

    #[test]
    fn test_attach_license_milestone_out_of_bounds() {
        let (env, contract_id, owner, _) = setup();
        env.as_contract(&contract_id, || {
            create_grant_with_milestone(&env, &owner, 1);

            let rights = IpRights {
                commercial_use: false,
                modification: false,
                distribution: false,
                sublicense: false,
            };

            let res = attach_license(
                &env,
                &owner,
                1,
                99,
                String::from_str(&env, "MIT"),
                LicenseType::OpenSource,
                rights,
                String::from_str(&env, ""),
            );
            assert_eq!(res, Err(ContractError::MilestoneIndexOutOfBounds));
        });
    }

    #[test]
    fn test_attach_license_rejects_pending_milestone() {
        let (env, contract_id, owner, _) = setup();
        env.as_contract(&contract_id, || {
            create_grant_with_milestone_in_state(&env, &owner, 1, MilestoneState::Pending);

            let rights = IpRights {
                commercial_use: false,
                modification: false,
                distribution: false,
                sublicense: false,
            };

            let result = attach_license(
                &env,
                &owner,
                1,
                0,
                String::from_str(&env, "MIT"),
                LicenseType::OpenSource,
                rights,
                String::from_str(&env, ""),
            );

            assert_eq!(result, Err(ContractError::InvalidState));
        });
    }

    #[test]
    fn test_attach_license_rejects_rejected_milestone() {
        let (env, contract_id, owner, _) = setup();
        env.as_contract(&contract_id, || {
            create_grant_with_milestone_in_state(&env, &owner, 1, MilestoneState::Rejected);

            let rights = IpRights {
                commercial_use: false,
                modification: false,
                distribution: false,
                sublicense: false,
            };

            let result = attach_license(
                &env,
                &owner,
                1,
                0,
                String::from_str(&env, "MIT"),
                LicenseType::OpenSource,
                rights,
                String::from_str(&env, ""),
            );

            assert_eq!(result, Err(ContractError::InvalidState));
        });
    }

    #[test]
    fn test_attach_license_all_types() {
        let (env, contract_id, owner, _) = setup();
        let rights = IpRights {
            commercial_use: true,
            modification: true,
            distribution: true,
            sublicense: true,
        };

        // Proprietary
        env.as_contract(&contract_id, || {
            create_grant_with_milestone(&env, &owner, 2);
            let r = attach_license(
                &env,
                &owner,
                2,
                0,
                String::from_str(&env, "PROPRIETARY"),
                LicenseType::Proprietary,
                rights.clone(),
                String::from_str(&env, ""),
            )
            .unwrap();
            assert_eq!(r.license_type, LicenseType::Proprietary);
        });

        // CreativeCommons
        env.as_contract(&contract_id, || {
            create_grant_with_milestone(&env, &owner, 3);
            let r = attach_license(
                &env,
                &owner,
                3,
                0,
                String::from_str(&env, "CC-BY-4.0"),
                LicenseType::CreativeCommons,
                rights.clone(),
                String::from_str(&env, ""),
            )
            .unwrap();
            assert_eq!(r.license_type, LicenseType::CreativeCommons);
        });

        // Custom
        env.as_contract(&contract_id, || {
            create_grant_with_milestone(&env, &owner, 4);
            let r = attach_license(
                &env,
                &owner,
                4,
                0,
                String::from_str(&env, "CUSTOM"),
                LicenseType::Custom,
                rights,
                String::from_str(&env, "Special terms"),
            )
            .unwrap();
            assert_eq!(r.license_type, LicenseType::Custom);
        });
    }
}
