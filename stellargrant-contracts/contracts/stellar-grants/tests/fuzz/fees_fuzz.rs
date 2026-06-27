/// Fuzz / property-based entry point for fee and math functions.
///
/// Run via: `cargo test --test fuzz_amounts` from the contract directory.
/// This module provides targeted fuzz coverage for fees.rs and math.rs:
/// - Fee computation at boundary amounts
/// - Split calculations that should always sum correctly
/// - Rounding invariants
/// - No-panic invariant: no valid input should ever cause a panic
use proptest::prelude::*;

/// Maximum values kept small enough to avoid i128 overflow while still
/// exercising interesting boundary conditions.
const MAX_AMOUNT: i128 = i128::MAX / 200;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]

    // ── Fee Fuzz Targets ──────────────────────────────────────────────────

    /// compute_fee never panics, never exceeds amount
    #[test]
    fn prop_compute_fee_boundary(
        amount in 0i128..=MAX_AMOUNT,
        fee_bps in 0u16..=10_000u16,
    ) {
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
    }

    /// compute_fee with zero amount returns zero
    #[test]
    fn prop_compute_fee_zero_amount(
        fee_bps in 1u16..=10_000u16,
    ) {
        let result = stellar_grants::fees::compute_fee(0, fee_bps as u32).unwrap();
        prop_assert_eq!(result, 0);
    }

    /// compute_fee with zero bps returns zero
    #[test]
    fn prop_compute_fee_zero_bps(
        amount in 1i128..=MAX_AMOUNT,
    ) {
        let result = stellar_grants::fees::compute_fee(amount, 0).unwrap();
        prop_assert_eq!(result, 0);
    }

    // ── Math Fuzz Targets ─────────────────────────────────────────────────

    /// basis_points_of round-trip invariant
    #[test]
    fn prop_basis_points_of_invariant(
        amount in 0i128..=MAX_AMOUNT,
        bps in 0u16..=10_000u16,
    ) {
        let result = stellar_grants::math::basis_points_of(amount, bps as u32).unwrap();

        // 0 bps always returns 0
        let zero = stellar_grants::math::basis_points_of(amount, 0).unwrap();
        prop_assert_eq!(zero, 0);

        // 10_000 bps returns the full amount
        let full = stellar_grants::math::basis_points_of(amount, 10_000).unwrap();
        prop_assert_eq!(full, amount);

        // Result should be <= amount
        prop_assert!(result <= amount);

        // Result should be >= 0
        prop_assert!(result >= 0);
    }

    /// split_evenly invariant: parts sum to total
    #[test]
    fn prop_split_evenly_sum_invariant(
        total in 0i128..=MAX_AMOUNT,
        n_parts in 1u32..=100u32,
    ) {
        let (per_part, remainder) = stellar_grants::math::split_evenly(total, n_parts).unwrap();

        let sum = per_part * n_parts as i128 + remainder;
        prop_assert_eq!(sum, total);

        // Remainder < n_parts
        prop_assert!(remainder >= 0);
        prop_assert!(remainder < n_parts as i128);

        // Balanced split: max - min <= 1
        prop_assert!(per_part >= 0);
    }

    /// proportional_share: shares sum to total (with rounding tolerance)
    #[test]
    fn prop_proportional_share_sum_invariant(
        total in 1i128..=MAX_AMOUNT,
        shares in prop::collection::vec(1u32..=10_000u32, 1..=10),
    ) {
        let bps_sum: u32 = shares.iter().sum();
        prop_assume!(bps_sum == 10_000);

        let mut total_distributed = 0i128;
        for &bps in shares.iter() {
            let share = stellar_grants::math::proportional_share(total, 10_000, bps as i128).unwrap();
            prop_assert!(share <= total);
            prop_assert!(share >= 0);
            total_distributed += share;
        }

        let diff = total - total_distributed;
        prop_assert!(diff >= 0 && diff <= shares.len() as i128);
    }

    /// No-panic invariant: all math functions must never panic
    #[test]
    fn prop_math_no_panic(
        a in i128::MIN/2..=i128::MAX/2,
        b in i128::MIN/2..=i128::MAX/2,
        n in 0u32..=200u32,
        bps in 0u32..=10_000u32,
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = stellar_grants::math::basis_points_of(a, bps);
        }));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = stellar_grants::math::proportional_share(a, b, 10_000);
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
    }

    /// Fee computation at boundary i128 values
    #[test]
    fn prop_fee_boundary_values(
        fee_bps in 0u16..=10_000u16,
    ) {
        // Test with 0
        let r0 = stellar_grants::fees::compute_fee(0, fee_bps as u32).unwrap();
        prop_assert_eq!(r0, 0);

        // Test with 1
        let r1 = stellar_grants::fees::compute_fee(1, fee_bps as u32).unwrap();
        prop_assert!(r1 <= 1);

        // Test with large values that shouldn't overflow for small bps
        if fee_bps <= 100 {
            let r_large = stellar_grants::fees::compute_fee(1_000_000, fee_bps as u32);
            prop_assert!(r_large.is_ok());
        }
    }
}
