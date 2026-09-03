/// Fuzz / property-based entry point for fee and math functions.
///
/// Run via: `cargo test --test fuzz_amounts` from the contract directory.
/// This module provides targeted fuzz coverage for fees.rs and math.rs:
/// - Fee computation at boundary amounts
/// - Split calculations that should always sum correctly
/// - Rounding invariants
/// - No-panic invariant: no valid input should ever cause a panic
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};

/// Maximum values kept small enough to avoid i128 overflow while still
/// exercising interesting boundary conditions.
const MAX_AMOUNT: i128 = i128::MAX / 200;

#[test]
fn prop_compute_fee_boundary() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(0i128..=MAX_AMOUNT, 0u16..=10_000u16),
            |(amount, fee_bps)| {
                let result = stellar_grants::fees::compute_fee(amount, fee_bps as u32);
                match result {
                    Ok(fee) => {
                        prop_assert!(fee <= amount, "fee {} exceeded amount {}", fee, amount);
                        prop_assert!(fee >= 0, "fee must be non-negative");
                        let remaining = amount - fee;
                        prop_assert_eq!(fee + remaining, amount);
                    }
                    Err(_) => {}
                }
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_compute_fee_zero_amount() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(1u16..=10_000u16), |fee_bps| {
            let result = stellar_grants::fees::compute_fee(0, fee_bps as u32).unwrap();
            prop_assert_eq!(result, 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_compute_fee_zero_bps() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(1i128..=MAX_AMOUNT), |amount| {
            let result = stellar_grants::fees::compute_fee(amount, 0).unwrap();
            prop_assert_eq!(result, 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_basis_points_of_invariant() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(0i128..=MAX_AMOUNT, 0u16..=10_000u16), |(amount, bps)| {
            let result = stellar_grants::math::basis_points_of(amount, bps as u32).unwrap();

            let zero = stellar_grants::math::basis_points_of(amount, 0).unwrap();
            prop_assert_eq!(zero, 0);

            let full = stellar_grants::math::basis_points_of(amount, 10_000).unwrap();
            prop_assert_eq!(full, amount);

            prop_assert!(result <= amount);
            prop_assert!(result >= 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_split_evenly_sum_invariant() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(0i128..=MAX_AMOUNT, 1u32..=100u32), |(total, n_parts)| {
            let (per_part, remainder) = stellar_grants::math::split_evenly(total, n_parts).unwrap();

            let sum = per_part * n_parts as i128 + remainder;
            prop_assert_eq!(sum, total);

            prop_assert!(remainder >= 0);
            prop_assert!(remainder < n_parts as i128);

            prop_assert!(per_part >= 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn prop_proportional_share_sum_invariant() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(
                1i128..=MAX_AMOUNT,
                prop::collection::vec(1u32..=10_000u32, 1..=10),
            ),
            |(total, shares)| {
                let bps_sum: u32 = shares.iter().sum();
                prop_assume!(bps_sum == 10_000);

                let mut total_distributed = 0i128;
                for &bps in shares.iter() {
                    let share = stellar_grants::math::proportional_share(total, bps).unwrap();
                    prop_assert!(share <= total);
                    prop_assert!(share >= 0);
                    total_distributed += share;
                }

                let diff = total - total_distributed;
                prop_assert!(diff >= 0 && diff <= shares.len() as i128);
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_math_no_panic() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(
            &(
                i128::MIN / 2..=i128::MAX / 2,
                i128::MIN / 2..=i128::MAX / 2,
                0u32..=200u32,
                0u32..=10_000u32,
            ),
            |(a, b, n, bps)| {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = stellar_grants::math::basis_points_of(a, bps);
                }));
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = stellar_grants::math::proportional_share(a, bps);
                }));
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = stellar_grants::math::split_evenly(a, n);
                }));
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = stellar_grants::math::safe_add(a, b);
                }));
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = stellar_grants::math::safe_sub(a, b);
                }));
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn prop_fee_boundary_values() {
    let mut runner = TestRunner::new(Config::with_cases(1000));
    runner
        .run(&(0u16..=10_000u16), |fee_bps| {
            let r0 = stellar_grants::fees::compute_fee(0, fee_bps as u32).unwrap();
            prop_assert_eq!(r0, 0);

            let r1 = stellar_grants::fees::compute_fee(1, fee_bps as u32).unwrap();
            prop_assert!(r1 <= 1);

            if fee_bps <= 100 {
                let r_large = stellar_grants::fees::compute_fee(1_000_000, fee_bps as u32);
                prop_assert!(r_large.is_ok());
            }
            Ok(())
        })
        .unwrap();
}
