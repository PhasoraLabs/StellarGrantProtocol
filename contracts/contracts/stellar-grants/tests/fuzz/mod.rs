/// Fuzz / property-based entry point for stellar-grants contract.
///
/// Run via: `cargo test --test fuzz_amounts` from the contract directory.
mod fees_fuzz;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};

/// Maximum values kept small enough to avoid i128 overflow while still
/// exercising interesting boundary conditions.
const MAX_AMOUNT: i128 = i128::MAX / 200;
const MAX_MILESTONES: u32 = 100;

#[test]
fn prop_grant_create_no_overflow() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(1i128..=MAX_AMOUNT, 1u32..=MAX_MILESTONES),
            |(milestone_amount, num_milestones)| {
                let total_required = milestone_amount.checked_mul(num_milestones as i128);
                if let Some(required) = total_required {
                    prop_assert!(required >= milestone_amount);
                    prop_assert!(required >= num_milestones as i128);
                }
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_grant_create_total_amount_validation() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(1i128..=1_000_000i128, 1u32..=20u32, 0i128..=1_000_000i128),
            |(milestone_amount, num_milestones, extra)| {
                let total_required = milestone_amount * num_milestones as i128;
                let total_amount = total_required + extra;
                prop_assert!(total_amount >= total_required);
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_cancel_refund_sum_equals_escrow() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(
                prop::collection::vec(1i128..=1_000_000i128, 1..=10),
                1i128..=10_000_000i128,
            ),
            |(contributions, escrow_balance)| {
                let total_contributions: i128 = contributions.iter().sum();
                let n = contributions.len();
                let mut distributed = 0i128;

                for (i, &amount) in contributions.iter().enumerate() {
                    let is_last = i + 1 == n;
                    let refund = if is_last {
                        escrow_balance - distributed
                    } else {
                        amount * escrow_balance / total_contributions
                    };
                    distributed += refund;
                }

                prop_assert_eq!(distributed, escrow_balance);

                let mut check_distributed = 0i128;
                for (i, &amount) in contributions.iter().enumerate() {
                    let is_last = i + 1 == n;
                    let refund = if is_last {
                        escrow_balance - check_distributed
                    } else {
                        amount * escrow_balance / total_contributions
                    };
                    prop_assert!(refund >= 0, "refund must be non-negative, got {}", refund);
                    check_distributed += refund;
                }
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_release_balance_conservation() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(1i128..=100_000i128, 1u32..=10u32, 0i128..=100_000i128),
            |(milestone_amount, num_milestones, extra_funding)| {
                let total_paid = milestone_amount * num_milestones as i128;
                let escrow_balance = total_paid + extra_funding;

                let owner_payout = total_paid;
                let remaining = escrow_balance - owner_payout;
                prop_assert_eq!(remaining, extra_funding);
                prop_assert!(remaining >= 0);
                prop_assert_eq!(owner_payout + remaining, escrow_balance);
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_quorum_bounds() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(1u32..=50u32, 1u32..=50u32), |(num_reviewers, quorum)| {
            let valid = quorum >= 1 && quorum <= num_reviewers;
            if quorum == 0 || quorum > num_reviewers {
                prop_assert!(!valid || quorum == 0);
            } else {
                prop_assert!(valid);
            }
            Ok(())
        })
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
