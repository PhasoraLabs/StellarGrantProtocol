/// Reviewer SLA enforcement module (issue #611).
///
/// Tracks the deadline by which a reviewer must cast their vote.
/// If the deadline passes without a vote the reviewer is marked as
/// SLA-breached and loses reward accrual for that milestone.
use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_000_000;

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReviewerSlaRecord {
    /// The reviewer whose SLA this tracks.
    pub reviewer: Address,
    /// Milestone ID this SLA applies to.
    pub milestone_id: u64,
    /// Ledger timestamp by which the reviewer must vote.
    pub deadline: u64,
    /// True once the reviewer has voted within the window.
    pub fulfilled: bool,
    /// True if the deadline passed without a vote.
    pub breached: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum SlaKey {
    /// Per-reviewer, per-milestone SLA record.
    ReviewerSla(Address, u64),
}

fn extend_ttl(env: &Env, key: &SlaKey) {
    if env.storage().persistent().has(key) {
        env.storage().persistent().extend_ttl(
            key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }
}

/// Compose the SLA subject id for a (grant, milestone) pair.
///
/// SLAs are keyed by a flat `u64`, so the grant id occupies the high 32 bits
/// and the milestone index the low 32 bits.
pub fn milestone_sla_id(grant_id: u64, milestone_idx: u32) -> u64 {
    ((grant_id & 0xFFFF_FFFF) << 32) | milestone_idx as u64
}

/// Register a reviewer SLA for `milestone_id` with the given `deadline`.
pub fn register_sla(env: &Env, reviewer: &Address, milestone_id: u64, deadline: u64) {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    let record = ReviewerSlaRecord {
        reviewer: reviewer.clone(),
        milestone_id,
        deadline,
        fulfilled: false,
        breached: false,
    };
    env.storage().persistent().set(&key, &record);
    extend_ttl(env, &key);

    env.events().publish(
        (symbol_short!("sla"), symbol_short!("reg"), milestone_id),
        (reviewer.clone(), deadline),
    );
}

/// Mark a reviewer's SLA as fulfilled (called when they cast their vote).
/// No-op if the SLA doesn't exist or is already resolved.
pub fn fulfill_sla(env: &Env, reviewer: &Address, milestone_id: u64) {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    extend_ttl(env, &key);
    let mut record: ReviewerSlaRecord = match env.storage().persistent().get(&key) {
        Some(r) => r,
        None => return,
    };
    if record.fulfilled || record.breached {
        return;
    }
    record.fulfilled = true;
    env.storage().persistent().set(&key, &record);
    extend_ttl(env, &key);
}

/// Check whether the SLA is breached (deadline passed, not fulfilled).
/// Persists the breached flag and emits an event on first detection.
pub fn check_and_mark_breach(env: &Env, reviewer: &Address, milestone_id: u64) -> bool {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    extend_ttl(env, &key);
    let mut record: ReviewerSlaRecord = match env.storage().persistent().get(&key) {
        Some(r) => r,
        None => return false,
    };
    if record.fulfilled || record.breached {
        return record.breached;
    }
    let now = env.ledger().timestamp();
    if now > record.deadline {
        record.breached = true;
        env.storage().persistent().set(&key, &record);
        extend_ttl(env, &key);
        env.events().publish(
            (symbol_short!("sla"), symbol_short!("breach"), milestone_id),
            reviewer.clone(),
        );
        return true;
    }
    false
}

/// Returns the SLA record for a reviewer / milestone pair, if it exists.
pub fn get_sla(env: &Env, reviewer: &Address, milestone_id: u64) -> Option<ReviewerSlaRecord> {
    let key = SlaKey::ReviewerSla(reviewer.clone(), milestone_id);
    extend_ttl(env, &key);
    env.storage().persistent().get(&key)
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::config;
    use crate::constants::DEFAULT_REVIEWER_SLA_SECONDS;
    use crate::reviewer_reward;
    use crate::types::AcceptanceCriteria;
    use crate::{StellarGrantsContract, StellarGrantsContractClient};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token, vec, String, Vec};

    struct Fixture<'a> {
        env: Env,
        contract_id: Address,
        client: StellarGrantsContractClient<'a>,
        owner: Address,
        reviewer: Address,
        grant_id: u64,
    }

    /// Register a reviewer, assign them to a funded single-milestone grant via
    /// the request/accept flow, and submit the milestone for review.
    fn setup_assigned_reviewer<'a>() -> Fixture<'a> {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        client.set_global_admin(&admin, &admin);
        client.update_config(&admin, &config::default_config());

        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);
        let funder = Address::generate(&env);
        token::StellarAssetClient::new(&env, &token).mint(&funder, &10_000_000);

        client.reviewer_register(
            &reviewer,
            &String::from_str(&env, "Reviewer"),
            &vec![&env, String::from_str(&env, "rust")],
            &None,
        );

        let grant_id = client.grant_create(
            &owner,
            &String::from_str(&env, "Grant"),
            &String::from_str(&env, "Description"),
            &token,
            &1_000_000,
            &1_000_000,
            &1,
            &Vec::new(&env),
        );
        client.grant_fund(&grant_id, &funder, &1_000_000);

        client.reviewer_request(
            &owner,
            &grant_id,
            &reviewer,
            &String::from_str(&env, "Please review"),
            &100,
        );
        client.reviewer_accept_request(&reviewer, &grant_id);

        client.milestone_submit(
            &grant_id,
            &0,
            &owner,
            &String::from_str(&env, "Done"),
            &String::from_str(&env, "https://proof.url"),
        );
        client.checklist_define_criteria(
            &owner,
            &grant_id,
            &0,
            &vec![
                &env,
                AcceptanceCriteria {
                    idx: 0,
                    description: String::from_str(&env, "Ships"),
                    is_required: true,
                },
            ],
        );
        client.checklist_submit(&owner, &grant_id, &0, &vec![&env, None]);
        client.checklist_review_criterion(&reviewer, &grant_id, &0, &0, &true);

        Fixture {
            client,
            contract_id,
            owner,
            reviewer,
            grant_id,
            env,
        }
    }

    fn advance(env: &Env, seconds: u64) {
        let now = env.ledger().timestamp();
        env.ledger().with_mut(|l| l.timestamp = now + seconds);
    }

    fn sla_of(
        f: &Fixture<'_>,
        reviewer: &Address,
        milestone_idx: u32,
    ) -> Option<ReviewerSlaRecord> {
        let sla_id = milestone_sla_id(f.grant_id, milestone_idx);
        f.env
            .as_contract(&f.contract_id, || get_sla(&f.env, reviewer, sla_id))
    }

    fn check_sla(f: &Fixture<'_>, reviewer: &Address, milestone_idx: u32) -> bool {
        let sla_id = milestone_sla_id(f.grant_id, milestone_idx);
        f.env.as_contract(&f.contract_id, || {
            check_and_mark_breach(&f.env, reviewer, sla_id)
        })
    }

    #[test]
    fn test_milestone_sla_id_is_unique_per_grant_and_milestone() {
        assert_eq!(milestone_sla_id(0, 0), 0);
        assert_eq!(milestone_sla_id(0, 7), 7);
        assert_eq!(milestone_sla_id(1, 0), 1 << 32);
        assert_ne!(milestone_sla_id(1, 0), milestone_sla_id(0, 1));
        assert_ne!(milestone_sla_id(2, 3), milestone_sla_id(3, 2));
    }

    #[test]
    fn test_accepting_an_assignment_registers_an_sla_per_milestone() {
        let f = setup_assigned_reviewer();

        let sla =
            sla_of(&f, &f.reviewer, 0).expect("accepting an assignment should register an SLA");
        assert_eq!(sla.reviewer, f.reviewer);
        assert_eq!(sla.deadline, DEFAULT_REVIEWER_SLA_SECONDS);
        assert!(!sla.fulfilled);
        assert!(!sla.breached);
    }

    #[test]
    fn test_voting_within_the_deadline_fulfills_the_sla_and_accrues_reward() {
        let f = setup_assigned_reviewer();

        advance(&f.env, DEFAULT_REVIEWER_SLA_SECONDS - 1);
        f.client
            .milestone_vote(&f.grant_id, &0, &f.reviewer, &true, &None);

        let sla = sla_of(&f, &f.reviewer, 0).unwrap();
        assert!(sla.fulfilled);
        assert!(!sla.breached);
        assert!(!check_sla(&f, &f.reviewer, 0));

        f.env.as_contract(&f.contract_id, || {
            let participation = reviewer_reward::get_participation(&f.env, &f.reviewer, f.grant_id)
                .expect("an on-time vote should accrue participation");
            assert_eq!(participation.votes_cast, 1);
        });
    }

    #[test]
    fn test_missing_the_deadline_records_a_breach_and_forfeits_reward() {
        let f = setup_assigned_reviewer();

        // The reviewer sits on the assignment past their deadline.
        advance(&f.env, DEFAULT_REVIEWER_SLA_SECONDS + 1);

        assert!(check_sla(&f, &f.reviewer, 0));

        f.client
            .milestone_vote(&f.grant_id, &0, &f.reviewer, &true, &None);

        let sla = sla_of(&f, &f.reviewer, 0).unwrap();
        assert!(sla.breached, "deadline passed without a vote");
        assert!(!sla.fulfilled, "a late vote must not fulfill the SLA");

        f.env.as_contract(&f.contract_id, || {
            assert!(
                reviewer_reward::get_participation(&f.env, &f.reviewer, f.grant_id).is_none(),
                "a breached reviewer accrues no reward for the milestone"
            );
        });

        // The vote itself still counts toward the milestone outcome.
        assert_eq!(
            f.client.get_milestone(&f.grant_id, &0).state,
            crate::types::MilestoneState::Approved
        );
    }

    #[test]
    fn test_late_vote_alone_marks_the_breach_without_a_prior_check() {
        let f = setup_assigned_reviewer();

        advance(&f.env, DEFAULT_REVIEWER_SLA_SECONDS + 1);

        // No explicit check_reviewer_sla call: casting the vote must detect it.
        f.client
            .milestone_vote(&f.grant_id, &0, &f.reviewer, &true, &None);

        let sla = sla_of(&f, &f.reviewer, 0).unwrap();
        assert!(sla.breached);
        assert!(!sla.fulfilled);

        f.env.as_contract(&f.contract_id, || {
            assert!(reviewer_reward::get_participation(&f.env, &f.reviewer, f.grant_id).is_none());
        });
    }

    #[test]
    fn test_reviewer_without_an_sla_record_is_never_breached() {
        let f = setup_assigned_reviewer();
        let stranger = Address::generate(&f.env);

        assert!(sla_of(&f, &stranger, 0).is_none());
        assert!(!check_sla(&f, &stranger, 0));

        // Unassigned reviewers keep accruing as before.
        f.env.as_contract(&f.contract_id, || {
            reviewer_reward::record_participation(&f.env, &stranger, f.grant_id, 0, false);
            assert!(reviewer_reward::get_participation(&f.env, &stranger, f.grant_id).is_some());
        });
        let _ = &f.owner;
    }
}
