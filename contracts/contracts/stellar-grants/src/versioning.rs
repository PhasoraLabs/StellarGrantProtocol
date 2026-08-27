use soroban_sdk::{Address, Env, Map, String, Symbol, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{Amendment, AmendmentStatus, Grant, GrantVersion};

fn field(env: &Env, name: &str) -> String {
    String::from_str(env, name)
}

/// Render an integer as a decimal `String`.
///
/// `soroban_sdk` is `no_std` and ships no numeric-to-string conversion, so the
/// digits are emitted by hand into a fixed buffer (itoa-style). Digits are
/// accumulated in the negative domain so that `i128::MIN` stays representable.
fn int_to_string(env: &Env, value: i128) -> String {
    // 39 digits for i128::MIN plus the sign.
    let mut buf = [0u8; 40];
    let mut idx = buf.len();
    let negative = value < 0;
    let mut remaining = if negative { value } else { -value };

    loop {
        idx -= 1;
        buf[idx] = b'0' + (-(remaining % 10)) as u8;
        remaining /= 10;
        if remaining == 0 {
            break;
        }
    }
    if negative {
        idx -= 1;
        buf[idx] = b'-';
    }

    String::from_bytes(env, &buf[idx..])
}

fn grant_to_version(
    env: &Env,
    grant: &Grant,
    version: u32,
    amendment_id: Option<u32>,
) -> GrantVersion {
    GrantVersion {
        grant_id: grant.id,
        version,
        title: grant.title.clone(),
        description: grant.description.clone(),
        total_amount: grant.total_amount,
        total_milestones: grant.total_milestones,
        created_at: env.ledger().timestamp(),
        amendment_id,
    }
}

fn is_material(env: &Env, changed_fields: &Vec<String>) -> bool {
    let title = field(env, "title");
    let total_amount = field(env, "total_amount");
    let total_milestones = field(env, "total_milestones");
    for changed in changed_fields.iter() {
        if changed == title || changed == total_amount || changed == total_milestones {
            return true;
        }
    }
    false
}

fn current_snapshot(env: &Env, grant_id: u64) -> Result<GrantVersion, ContractError> {
    let current = current_version(env, grant_id);
    if current == 0 {
        let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
        let snapshot = grant_to_version(env, &grant, 1, None);
        Storage::set_grant_version(env, grant_id, 1, &snapshot);
        Storage::set_current_version(env, grant_id, 1);
        return Ok(snapshot);
    }
    Storage::get_grant_version(env, grant_id, current).ok_or(ContractError::GrantNotFound)
}

pub fn create_initial_version(env: &Env, grant: &Grant) {
    let snapshot = grant_to_version(env, grant, 1, None);
    Storage::set_grant_version(env, grant.id, 1, &snapshot);
    Storage::set_current_version(env, grant.id, 1);
}

/// Propose an amendment to a grant. Owner only.
pub fn propose_amendment(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    changed_fields: Vec<String>,
    new_values: Vec<String>,
    rationale: String,
) -> Result<u32, ContractError> {
    if changed_fields.is_empty() || changed_fields.len() != new_values.len() {
        return Err(ContractError::InvalidInput);
    }

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    let current = current_snapshot(env, grant_id)?;
    let amendment_version = current.version.saturating_add(1);
    let mut previous_values = Vec::new(env);
    let title = field(env, "title");
    let description = field(env, "description");
    let total_amount = field(env, "total_amount");
    let total_milestones = field(env, "total_milestones");

    for changed in changed_fields.iter() {
        if changed == title {
            previous_values.push_back(current.title.clone());
        } else if changed == description {
            previous_values.push_back(current.description.clone());
        } else if changed == total_amount {
            previous_values.push_back(int_to_string(env, current.total_amount));
        } else if changed == total_milestones {
            previous_values.push_back(int_to_string(env, current.total_milestones as i128));
        } else {
            return Err(ContractError::InvalidInput);
        }
    }

    let status = if is_material(env, &changed_fields) {
        AmendmentStatus::Proposed
    } else {
        AmendmentStatus::Approved
    };
    let resolved_at = if status == AmendmentStatus::Approved {
        Some(env.ledger().timestamp())
    } else {
        None
    };
    let amendment = Amendment {
        grant_id,
        version: amendment_version,
        proposed_by: owner.clone(),
        changed_fields,
        previous_values,
        new_values,
        rationale,
        status: status.clone(),
        reviewer_votes: Map::new(env),
        proposed_at: env.ledger().timestamp(),
        resolved_at,
    };

    Storage::set_amendment(env, grant_id, amendment_version, &amendment);
    let mut history = Storage::get_amendment_history(env, grant_id);
    history.push_back(amendment_version);
    Storage::set_amendment_history(env, grant_id, &history);
    env.events().publish(
        (Symbol::new(env, "amendment_proposed"), grant_id),
        (owner.clone(), amendment_version),
    );
    if status == AmendmentStatus::Approved {
        env.events().publish(
            (Symbol::new(env, "amendment_approved"), grant_id),
            amendment_version,
        );
    }
    Ok(amendment_version)
}

/// Reviewer votes on an amendment.
pub fn vote_amendment(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    amendment_version: u32,
    approve: bool,
) -> Result<AmendmentStatus, ContractError> {
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if !grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let mut amendment = Storage::get_amendment(env, grant_id, amendment_version)
        .ok_or(ContractError::InvalidInput)?;
    if amendment.status != AmendmentStatus::Proposed {
        return Err(ContractError::InvalidState);
    }
    if amendment.reviewer_votes.contains_key(reviewer.clone()) {
        return Err(ContractError::AlreadyVoted);
    }

    amendment.reviewer_votes.set(reviewer.clone(), approve);

    // Tally votes to check quorum (>50% of reviewers)
    let mut approvals = 0u32;
    let mut rejections = 0u32;
    for (_, voted_approve) in amendment.reviewer_votes.iter() {
        if voted_approve {
            approvals = approvals.saturating_add(1);
        } else {
            rejections = rejections.saturating_add(1);
        }
    }

    let total_reviewers = grant.reviewers.len() as u32;
    let approval_quorum = total_reviewers > 0 && approvals * 2 > total_reviewers;
    let rejection_quorum = total_reviewers > 0 && rejections * 2 > total_reviewers;

    if approval_quorum {
        amendment.status = AmendmentStatus::Approved;
        amendment.resolved_at = Some(env.ledger().timestamp());
        env.events().publish(
            (Symbol::new(env, "amendment_approved"), grant_id),
            amendment_version,
        );
    } else if rejection_quorum {
        amendment.status = AmendmentStatus::Rejected;
        amendment.resolved_at = Some(env.ledger().timestamp());
    }

    Storage::set_amendment(env, grant_id, amendment_version, &amendment);
    Ok(amendment.status)
}

/// Apply an approved amendment, creating a new version snapshot.
pub fn apply_amendment(
    env: &Env,
    grant_id: u64,
    amendment_version: u32,
) -> Result<GrantVersion, ContractError> {
    let amendment = Storage::get_amendment(env, grant_id, amendment_version)
        .ok_or(ContractError::InvalidInput)?;
    if amendment.status != AmendmentStatus::Approved {
        return Err(ContractError::InvalidState);
    }

    let mut grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    let mut snapshot = current_snapshot(env, grant_id)?;
    let title = field(env, "title");
    let description = field(env, "description");
    let total_amount = field(env, "total_amount");
    let total_milestones = field(env, "total_milestones");
    for i in 0..amendment.changed_fields.len() {
        let changed = amendment.changed_fields.get(i).unwrap();
        let value = amendment.new_values.get(i).unwrap();
        if changed == title {
            snapshot.title = value.clone();
            grant.title = value;
        } else if changed == description {
            snapshot.description = value.clone();
            grant.description = value;
        } else if changed == total_amount {
            // Try to parse string value as i128
            if let Some(new_total) = parse_i128_from_string(&value) {
                snapshot.total_amount = new_total;
                grant.total_amount = new_total;
            }
        } else if changed == total_milestones {
            // Try to parse string value as u32
            if let Some(new_total) = parse_u32_from_string(&value) {
                snapshot.total_milestones = new_total;
                grant.total_milestones = new_total;
            }
        }
    }

    snapshot.version = amendment_version;
    snapshot.created_at = env.ledger().timestamp();
    snapshot.amendment_id = Some(amendment_version);
    Storage::set_grant(env, grant_id, &grant);
    Storage::set_grant_version(env, grant_id, amendment_version, &snapshot);
    Storage::set_current_version(env, grant_id, amendment_version);
    env.events().publish(
        (Symbol::new(env, "amendment_applied"), grant_id),
        amendment_version,
    );
    Ok(snapshot)
}

fn parse_i128_from_string(s: &String) -> Option<i128> {
    let bytes = s.to_bytes();
    let negative = bytes.get(0)? == b'-';
    let start = if negative { 1 } else { 0 };
    if bytes.len() == start {
        return None;
    }

    let mut value = 0i128;
    for i in start..bytes.len() {
        let digit = bytes.get(i)?.checked_sub(b'0')?;
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?;
        value = if negative {
            value.checked_sub(digit as i128)?
        } else {
            value.checked_add(digit as i128)?
        };
    }
    Some(value)
}

fn parse_u32_from_string(s: &String) -> Option<u32> {
    u32::try_from(parse_i128_from_string(s)?).ok()
}

/// Return a specific version snapshot.
pub fn get_version(env: &Env, grant_id: u64, version: u32) -> Option<GrantVersion> {
    Storage::get_grant_version(env, grant_id, version)
}

/// Return the current version number for a grant.
pub fn current_version(env: &Env, grant_id: u64) -> u32 {
    Storage::get_current_version(env, grant_id)
}

/// Return the full amendment history for a grant.
pub fn amendment_history(env: &Env, grant_id: u64) -> Vec<Amendment> {
    let mut amendments = Vec::new(env);
    for version in Storage::get_amendment_history(env, grant_id).iter() {
        if let Some(amendment) = Storage::get_amendment(env, grant_id, version) {
            amendments.push_back(amendment);
        }
    }
    amendments
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::types::GrantStatus;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::vec;

    fn seed_grant(
        env: &Env,
        id: u64,
        owner: &Address,
        total_amount: i128,
        total_milestones: u32,
    ) -> u64 {
        let grant = Grant {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "Original title"),
            description: String::from_str(env, "Original description"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount,
            milestone_amount: total_amount / (total_milestones.max(1) as i128),
            reviewers: Vec::new(env),
            total_milestones,
            milestones_paid_out: 0,
            escrow_balance: total_amount,
            funders: Vec::new(env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        };
        Storage::set_grant(env, grant.id, &grant);
        create_initial_version(env, &grant);
        grant.id
    }

    fn set_reviewers(env: &Env, grant_id: u64, reviewers: Vec<Address>) {
        let mut grant = Storage::get_grant(env, grant_id).unwrap();
        grant.reviewers = reviewers;
        Storage::set_grant(env, grant_id, &grant);
    }

    fn propose_field(env: &Env, owner: &Address, grant_id: u64, name: &str, value: &str) -> u32 {
        propose_amendment(
            env,
            owner,
            grant_id,
            vec![env, field(env, name)],
            vec![env, String::from_str(env, value)],
            String::from_str(env, "test"),
        )
        .unwrap()
    }

    #[test]
    fn test_int_to_string_covers_sign_and_bounds() {
        let env = Env::default();
        assert_eq!(int_to_string(&env, 0), String::from_str(&env, "0"));
        assert_eq!(int_to_string(&env, 7), String::from_str(&env, "7"));
        assert_eq!(
            int_to_string(&env, 5_000_000),
            String::from_str(&env, "5000000")
        );
        assert_eq!(int_to_string(&env, -42), String::from_str(&env, "-42"));
        assert_eq!(
            int_to_string(&env, i128::MIN),
            String::from_str(&env, "-170141183460469231731687303715884105728")
        );
        assert_eq!(
            int_to_string(&env, i128::MAX),
            String::from_str(&env, "170141183460469231731687303715884105727")
        );
    }

    #[test]
    fn test_propose_amendment_records_real_previous_total_amount() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let grant_id = seed_grant(&env, 1, &owner, 5_000_000, 4);

            let version = propose_amendment(
                &env,
                &owner,
                grant_id,
                vec![&env, String::from_str(&env, "total_amount")],
                vec![&env, String::from_str(&env, "7000000")],
                String::from_str(&env, "scope grew"),
            )
            .expect("amendment should be proposed");

            let amendment =
                Storage::get_amendment(&env, grant_id, version).expect("amendment should exist");
            assert_eq!(
                amendment.previous_values.get(0).unwrap(),
                String::from_str(&env, "5000000")
            );
        });
    }

    #[test]
    fn test_propose_amendment_records_real_previous_total_milestones() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let grant_id = seed_grant(&env, 1, &owner, 5_000_000, 4);

            let version = propose_amendment(
                &env,
                &owner,
                grant_id,
                vec![&env, String::from_str(&env, "total_milestones")],
                vec![&env, String::from_str(&env, "6")],
                String::from_str(&env, "more checkpoints"),
            )
            .expect("amendment should be proposed");

            let amendment =
                Storage::get_amendment(&env, grant_id, version).expect("amendment should exist");
            assert_eq!(
                amendment.previous_values.get(0).unwrap(),
                String::from_str(&env, "4")
            );
        });
    }

    #[test]
    fn test_materiality_and_rejection_quorum() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let non_material = seed_grant(&env, 1, &owner, 1_000, 2);
            let version = propose_field(&env, &owner, non_material, "description", "Clarified");
            assert_eq!(
                Storage::get_amendment(&env, non_material, version)
                    .unwrap()
                    .status,
                AmendmentStatus::Approved
            );

            let material = seed_grant(&env, 2, &owner, 1_000, 2);
            let version = propose_field(&env, &owner, material, "title", "New title");
            assert_eq!(
                Storage::get_amendment(&env, material, version)
                    .unwrap()
                    .status,
                AmendmentStatus::Proposed
            );

            let rejected = seed_grant(&env, 3, &owner, 1_000, 2);
            let reviewer1 = Address::generate(&env);
            let reviewer2 = Address::generate(&env);
            set_reviewers(
                &env,
                rejected,
                vec![&env, reviewer1.clone(), reviewer2.clone()],
            );
            let version = propose_field(&env, &owner, rejected, "title", "New title");
            assert_eq!(
                vote_amendment(&env, &reviewer1, rejected, version, false),
                Ok(AmendmentStatus::Proposed)
            );
            assert_eq!(
                vote_amendment(&env, &reviewer2, rejected, version, false),
                Ok(AmendmentStatus::Rejected)
            );
        });
    }

    #[test]
    fn test_propose_amendment_records_previous_values_for_all_fields() {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let grant_id = seed_grant(&env, 1, &owner, 1_234, 2);
            set_reviewers(&env, grant_id, vec![&env, reviewer.clone()]);

            let version = propose_amendment(
                &env,
                &owner,
                grant_id,
                vec![
                    &env,
                    String::from_str(&env, "title"),
                    String::from_str(&env, "description"),
                    String::from_str(&env, "total_amount"),
                    String::from_str(&env, "total_milestones"),
                ],
                vec![
                    &env,
                    String::from_str(&env, "New title"),
                    String::from_str(&env, "New description"),
                    String::from_str(&env, "9999"),
                    String::from_str(&env, "3"),
                ],
                String::from_str(&env, "full rewrite"),
            )
            .expect("amendment should be proposed");

            let amendment =
                Storage::get_amendment(&env, grant_id, version).expect("amendment should exist");
            assert_eq!(
                amendment.previous_values,
                vec![
                    &env,
                    String::from_str(&env, "Original title"),
                    String::from_str(&env, "Original description"),
                    String::from_str(&env, "1234"),
                    String::from_str(&env, "2"),
                ]
            );

            assert_eq!(
                vote_amendment(&env, &reviewer, grant_id, version, true),
                Ok(AmendmentStatus::Approved)
            );
            let snapshot = apply_amendment(&env, grant_id, version).unwrap();
            let grant = Storage::get_grant(&env, grant_id).unwrap();
            assert_eq!(
                (snapshot.total_amount, snapshot.total_milestones),
                (9_999, 3)
            );
            assert_eq!((grant.total_amount, grant.total_milestones), (9_999, 3));
        });
    }
}
