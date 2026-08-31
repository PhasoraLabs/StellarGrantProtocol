/// Fuzz / property-based entry point for stellar-grants contract.
///
/// Run via: `cargo test --test fuzz_amounts` from the contract directory.
///
/// Issue #979: the five `prop_*` tests below used to assert properties about
/// arithmetic *reimplemented inline in the test itself* and never called any
/// `stellar_grants::*` function, so a real regression in the crate's overflow
/// check, refund split, escrow release accounting, or quorum rule could never
/// fail them. They now drive the actual contract entry points via
/// `StellarGrantsContractClient` with fuzzed inputs — following the pattern in
/// `fees_fuzz.rs` — with the case count dialled down because each case spins up
/// a fresh contract instance. The pure-math property tests further down
/// (`prop_basis_points_*`, `prop_proportional_share_*`) already call real crate
/// functions and are unchanged.
mod fees_fuzz;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, Address, Env, String, Vec as SorobanVec};
use stellar_grants::{
    AcceptanceCriteria, MilestoneState, StellarGrantsContract, StellarGrantsContractClient,
};

/// `ProtocolConfig::max_milestones_per_grant` default — grant creation rejects
/// anything above this *before* reaching the `checked_mul` overflow guard, so
/// the overflow fuzz test stays at or below it.
const MAX_MILESTONES_PER_GRANT: u32 = 20;
const COMMUNITY_REVIEW_PERIOD: u64 = 3 * 24 * 60 * 60;

/// A freshly-registered contract plus a Stellar-asset token to fund grants with.
struct Fixture {
    env: Env,
    client: StellarGrantsContractClient<'static>,
    token: Address,
    token_admin: token::StellarAssetClient<'static>,
    token_client: token::Client<'static>,
}

fn fixture() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let tok_admin_addr = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(tok_admin_addr)
        .address();

    // Leak `env` to `'static` so the clients can live in the returned struct;
    // the struct (and therefore `env`) is dropped at the end of each proptest
    // case. Same pattern as tests/test_reputation_and_dispute_fee.rs.
    let env_ref: &'static Env = unsafe { &*(&env as *const Env) };
    let contract_id = env.register_contract(None, StellarGrantsContract);
    let client = StellarGrantsContractClient::new(env_ref, &contract_id);
    let token_admin = token::StellarAssetClient::new(env_ref, &token);
    let token_client = token::Client::new(env_ref, &token);
    client.initialize(&admin);

    Fixture {
        env,
        client,
        token,
        token_admin,
        token_client,
    }
}

impl Fixture {
    fn title(&self) -> String {
        String::from_str(&self.env, "G")
    }
    fn desc(&self) -> String {
        String::from_str(&self.env, "D")
    }
    fn no_reviewers(&self) -> SorobanVec<Address> {
        SorobanVec::new(&self.env)
    }

    /// Satisfy the required-criteria checklist gate that `milestone_vote` now
    /// enforces (see tests/integration_lifecycle.rs::setup_checklist).
    fn satisfy_checklist(&self, owner: &Address, reviewer: &Address, grant_id: u64, ms: u32) {
        let criteria = SorobanVec::from_array(
            &self.env,
            [AcceptanceCriteria {
                idx: 0,
                description: String::from_str(&self.env, "c"),
                is_required: true,
            }],
        );
        self.client
            .checklist_define_criteria(owner, &grant_id, &ms, &criteria);
        let evidence = SorobanVec::from_array(
            &self.env,
            [Some(String::from_str(&self.env, "https://evidence.com"))],
        );
        self.client
            .checklist_submit(owner, &grant_id, &ms, &evidence);
        self.client
            .checklist_review_criterion(reviewer, &grant_id, &ms, &0u32, &true);
    }
}

#[test]
fn prop_grant_create_rejects_overflowing_milestone_math() {
    // `internal_grant_create` guards `milestone_amount * num_milestones` with
    // `checked_mul(...).ok_or(ContractError::InvalidInput)`. Feed it inputs that
    // always overflow i128 and assert it returns a *clean* contract error
    // (`Err(Ok(_))`), never a host trap (`Err(Err(_))`) from an unchecked
    // multiply panicking. Delete the `checked_mul` and this fails.
    let mut runner = TestRunner::new(Config::with_cases(48));
    runner
        .run(
            &(
                (i128::MAX / 2 + 1)..=i128::MAX,
                2u32..=MAX_MILESTONES_PER_GRANT,
            ),
            |(milestone_amount, num_milestones)| {
                prop_assert!(milestone_amount
                    .checked_mul(num_milestones as i128)
                    .is_none());

                let f = fixture();
                let owner = Address::generate(&f.env);
                let res = f.client.try_grant_create(
                    &owner,
                    &f.title(),
                    &f.desc(),
                    &f.token,
                    &i128::MAX,
                    &milestone_amount,
                    &num_milestones,
                    &f.no_reviewers(),
                );
                prop_assert!(
                    matches!(res, Err(Ok(_))),
                    "expected a clean ContractError, got {:?}",
                    res
                );
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_grant_create_enforces_total_covers_milestones() {
    // Drives the real `if total_amount < total_required` check in
    // `internal_grant_create`: an under-funded total is rejected cleanly, and an
    // exactly-covering total is accepted with the stored grant echoing the
    // inputs.
    let mut runner = TestRunner::new(Config::with_cases(48));
    runner
        .run(
            &(1i128..=1_000_000i128, 1u32..=20u32, 1i128..=1_000_000i128),
            |(milestone_amount, num_milestones, deficit)| {
                let required = milestone_amount * num_milestones as i128;
                let f = fixture();
                let owner = Address::generate(&f.env);

                let under = (required - deficit).max(1);
                if under < required {
                    let res = f.client.try_grant_create(
                        &owner,
                        &f.title(),
                        &f.desc(),
                        &f.token,
                        &under,
                        &milestone_amount,
                        &num_milestones,
                        &f.no_reviewers(),
                    );
                    prop_assert!(
                        matches!(res, Err(Ok(_))),
                        "under-funded total {} < required {} must be rejected, got {:?}",
                        under,
                        required,
                        res
                    );
                }

                match f.client.try_grant_create(
                    &owner,
                    &f.title(),
                    &f.desc(),
                    &f.token,
                    &required,
                    &milestone_amount,
                    &num_milestones,
                    &f.no_reviewers(),
                ) {
                    Ok(Ok(gid)) => {
                        let g = f.client.get_grant(&gid);
                        prop_assert_eq!(g.total_amount, required);
                        prop_assert_eq!(g.total_milestones, num_milestones);
                        prop_assert_eq!(g.escrow_balance, 0);
                    }
                    other => prop_assert!(false, "expected Ok(Ok(_)), got {:?}", other),
                }
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_cancel_refund_sum_equals_escrow() {
    // Drives the real `escrow::refund_all` split through `grant_cancel`: every
    // funder is paid back, the payouts sum to exactly the gross escrow, none
    // exceeds what that funder contributed, and escrow is zeroed. Break the
    // "last funder gets the remainder" line in `refund_all` and the sum breaks.
    let mut runner = TestRunner::new(Config::with_cases(40));
    runner
        .run(
            &prop::collection::vec(1i128..=1_000_000i128, 1..=5),
            |contributions| {
                let total: i128 = contributions.iter().sum();
                let f = fixture();
                let owner = Address::generate(&f.env);

                let gid = f.client.grant_create(
                    &owner,
                    &f.title(),
                    &f.desc(),
                    &f.token,
                    &total,
                    &total,
                    &1,
                    &f.no_reviewers(),
                );

                let mut funders: std::vec::Vec<Address> = std::vec::Vec::new();
                for &amount in &contributions {
                    let funder = Address::generate(&f.env);
                    f.token_admin.mint(&funder, &amount);
                    f.client.grant_fund(&gid, &funder, &amount);
                    funders.push(funder);
                }
                prop_assert_eq!(f.client.get_grant(&gid).escrow_balance, total);

                f.client
                    .grant_cancel(&gid, &owner, &String::from_str(&f.env, "cancel"));

                let mut refunded = 0i128;
                for (funder, &amount) in funders.iter().zip(contributions.iter()) {
                    let bal = f.token_client.balance(funder);
                    prop_assert!(bal >= 0 && bal <= amount);
                    refunded += bal;
                }
                prop_assert_eq!(refunded, total);
                prop_assert_eq!(f.client.get_grant(&gid).escrow_balance, 0);
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_release_balance_conservation() {
    // Drives real `escrow::release` + `refund_all` through `grant_complete`.
    // The owner is paid the milestone net of the protocol fee (`fees::compute_fee`);
    // `deduct_and_split_fee` only *books* the fee (it doesn't transfer it out),
    // so `refund_all` sweeps the whole remaining escrow — the un-earmarked
    // funding plus the booked fee — back to the funder. Net effect:
    //   owner_payout + funder_refund == gross escrow, escrow fully drained.
    let mut runner = TestRunner::new(Config::with_cases(32));
    runner
        .run(
            &(1i128..=100_000i128, 0i128..=100_000i128),
            |(milestone_amount, extra)| {
                let total = milestone_amount + extra;
                let f = fixture();
                let owner = Address::generate(&f.env);
                let reviewer = Address::generate(&f.env);
                let mut reviewers = SorobanVec::new(&f.env);
                reviewers.push_back(reviewer.clone());

                let gid = f.client.grant_create(
                    &owner,
                    &f.title(),
                    &f.desc(),
                    &f.token,
                    &total,
                    &milestone_amount,
                    &1,
                    &reviewers,
                );
                let funder = Address::generate(&f.env);
                f.token_admin.mint(&funder, &total);
                f.client.grant_fund(&gid, &funder, &total);

                f.client.milestone_submit(
                    &gid,
                    &0,
                    &owner,
                    &String::from_str(&f.env, "m"),
                    &String::from_str(&f.env, "p"),
                );
                f.satisfy_checklist(&owner, &reviewer, gid, 0);
                let now = f.env.ledger().timestamp();
                f.env
                    .ledger()
                    .set_timestamp(now + COMMUNITY_REVIEW_PERIOD + 1);
                f.client.milestone_vote(&gid, &0, &reviewer, &true, &None);
                prop_assert_eq!(
                    f.client.get_milestone(&gid, &0).state,
                    MilestoneState::Approved
                );

                f.client.grant_complete(&gid);

                let fee_bps = f.client.get_protocol_config().protocol_fee_bps;
                let fee = stellar_grants::fees::compute_fee(milestone_amount, fee_bps).unwrap();
                let owner_bal = f.token_client.balance(&owner);
                let funder_bal = f.token_client.balance(&funder);
                prop_assert_eq!(owner_bal, milestone_amount - fee);
                prop_assert_eq!(funder_bal, extra + fee);
                prop_assert_eq!(owner_bal + funder_bal, total);
                prop_assert_eq!(f.client.get_grant(&gid).escrow_balance, 0);
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_quorum_bounds() {
    // Drives the real `governance::quorum_reached` rule (strictly more than half
    // the snapshotted reviewers must approve; reviewer reputation defaults to 1
    // so weighted approvals == vote count) through actual milestone voting.
    // Flip the `>` to `>=` in the crate and the even-reviewer boundary fails.
    let mut runner = TestRunner::new(Config::with_cases(24));
    runner
        .run(
            &(1u32..=6u32).prop_flat_map(|n| (Just(n), 0u32..=n)),
            |(num_reviewers, num_approvals)| {
                let f = fixture();
                let owner = Address::generate(&f.env);
                let mut reviewers = SorobanVec::new(&f.env);
                let mut rlist: std::vec::Vec<Address> = std::vec::Vec::new();
                for _ in 0..num_reviewers {
                    let r = Address::generate(&f.env);
                    reviewers.push_back(r.clone());
                    rlist.push(r);
                }

                let gid = f.client.grant_create(
                    &owner,
                    &f.title(),
                    &f.desc(),
                    &f.token,
                    &1000,
                    &1000,
                    &1,
                    &reviewers,
                );
                let funder = Address::generate(&f.env);
                f.token_admin.mint(&funder, &1000);
                f.client.grant_fund(&gid, &funder, &1000);
                f.client.milestone_submit(
                    &gid,
                    &0,
                    &owner,
                    &String::from_str(&f.env, "m"),
                    &String::from_str(&f.env, "p"),
                );
                f.satisfy_checklist(&owner, &rlist[0], gid, 0);
                let now = f.env.ledger().timestamp();
                f.env
                    .ledger()
                    .set_timestamp(now + COMMUNITY_REVIEW_PERIOD + 1);

                for r in rlist.iter().take(num_approvals as usize) {
                    // Stop once the milestone finalizes — voting on a
                    // non-`Submitted` milestone would panic.
                    if f.client.get_milestone(&gid, &0).state == MilestoneState::Submitted {
                        f.client.milestone_vote(&gid, &0, r, &true, &None);
                    }
                }

                let approved = f.client.get_milestone(&gid, &0).state == MilestoneState::Approved;
                let expected = num_approvals * 2 > num_reviewers;
                prop_assert_eq!(
                    approved,
                    expected,
                    "num_reviewers={} num_approvals={}",
                    num_reviewers,
                    num_approvals
                );
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_basis_points_100pct() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(0i128..=i128::MAX), |amount| {
            prop_assert_eq!(
                stellar_grants::math::basis_points_of(amount, 10_000).unwrap(),
                amount,
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_basis_points_zero_bps() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(0i128..=i128::MAX), |amount| {
            prop_assert_eq!(stellar_grants::math::basis_points_of(amount, 0).unwrap(), 0,);
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_basis_points_imax_1bp() {
    let result = stellar_grants::math::basis_points_of(i128::MAX, 1);
    assert!(result.is_ok(), "must not overflow for i128::MAX, 1bp");
    let val = result.unwrap();
    assert!(val > 0, "result must be positive");
}

#[test]
fn prop_basis_points_invalid_bps() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(0i128..=i128::MAX, 10_001u32..=u32::MAX),
            |(amount, bps)| {
                prop_assert!(stellar_grants::math::basis_points_of(amount, bps).is_err());
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_basis_points_partition_never_exceeds_total() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(
                0i128..=i128::MAX / 2,
                prop::collection::vec(0u32..=10_000u32, 1..=10),
            ),
            |(total, shares)| {
                let mut sum = 0i128;
                for &bps in &shares {
                    let part = stellar_grants::math::basis_points_of(total, bps).unwrap();
                    sum = sum.saturating_add(part);
                }
                prop_assert!(
                    sum <= total,
                    "partition sum {} exceeded total {}",
                    sum,
                    total
                );
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_proportional_share_100pct() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(0i128..=i128::MAX), |total| {
            prop_assert_eq!(
                stellar_grants::math::proportional_share(total, 10_000).unwrap(),
                total,
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_proportional_share_zero() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(0i128..=i128::MAX), |total| {
            prop_assert_eq!(
                stellar_grants::math::proportional_share(total, 0).unwrap(),
                0,
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_proportional_share_imax_1bp() {
    let result = stellar_grants::math::proportional_share(i128::MAX, 1);
    assert!(result.is_ok(), "must not overflow for i128::MAX");
}
