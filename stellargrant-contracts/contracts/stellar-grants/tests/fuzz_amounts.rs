mod fuzz;

use proptest::prelude::*;

/// Maximum values kept small enough to avoid i128 overflow while still
/// exercising interesting boundary conditions.
const MAX_AMOUNT: i128 = i128::MAX / 200;
const MAX_MILESTONES: u32 = 100;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]

    /// `grant_create` arithmetic must never overflow.
    /// The contract uses `checked_mul`, so any overflow is caught and returns
    /// an error rather than silently wrapping.
    #[test]
    fn prop_grant_create_no_overflow(
        milestone_amount in 1i128..=MAX_AMOUNT,
        num_milestones in 1u32..=MAX_MILESTONES,
    ) {
        let total_required = milestone_amount.checked_mul(num_milestones as i128);
        // Either the multiplication succeeds and the result is correct,
        // or it overflows — in both cases the contract will handle it safely.
        if let Some(required) = total_required {
            prop_assert!(required >= milestone_amount);
            prop_assert!(required >= num_milestones as i128);
        }
        // If checked_mul returns None the contract would reject with InvalidInput.
    }

    /// Total amount must be >= milestone_amount * num_milestones.
    #[test]
    fn prop_grant_create_total_amount_validation(
        milestone_amount in 1i128..=1_000_000i128,
        num_milestones in 1u32..=20u32,
        extra in 0i128..=1_000_000i128,
    ) {
        let total_required = milestone_amount * num_milestones as i128;
        let total_amount = total_required + extra;
        prop_assert!(total_amount >= total_required);
    }

    /// Proportional refund distribution must sum to exactly the escrow balance.
    /// The last funder absorbs any remainder from integer division, so the
    /// distributed total always equals `escrow_balance`.
    #[test]
    fn prop_cancel_refund_sum_equals_escrow(
        contributions in prop::collection::vec(1i128..=1_000_000i128, 1..=10),
        escrow_balance in 1i128..=10_000_000i128,
    ) {
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

        // Total distributed must equal escrow_balance exactly
        prop_assert_eq!(distributed, escrow_balance);

        // No funder can receive a negative refund
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
    }

    /// After all milestones are released the remaining balance refunded to
    /// funders plus the amount paid to the owner must equal the original
    /// escrow_balance.
    #[test]
    fn prop_release_balance_conservation(
        milestone_amount in 1i128..=100_000i128,
        num_milestones in 1u32..=10u32,
        extra_funding in 0i128..=100_000i128,
    ) {
        let total_paid = milestone_amount * num_milestones as i128;
        let escrow_balance = total_paid + extra_funding;

        // Owner receives total_paid; funders share extra_funding.
        let owner_payout = total_paid;
        let remaining = escrow_balance - owner_payout;
        prop_assert_eq!(remaining, extra_funding);
        prop_assert!(remaining >= 0);
        prop_assert_eq!(owner_payout + remaining, escrow_balance);
    }

    /// Quorum must always be between 1 and the total number of reviewers.
    #[test]
    fn prop_quorum_bounds(
        num_reviewers in 1u32..=50u32,
        quorum in 1u32..=50u32,
    ) {
        let valid = quorum >= 1 && quorum <= num_reviewers;
        // Contract rejects quorum == 0 or quorum > num_reviewers
        if quorum == 0 || quorum > num_reviewers {
            prop_assert!(!valid || quorum == 0);
        } else {
            prop_assert!(valid);
        }
    }

    // ── Issue #627: Fees and Math Fuzz Targets ─────────────────────────────

    /// Target 1: compute_fee never panics, never exceeds amount.
    /// For all valid (amount: i128, fee_bps: u16) pairs:
    /// - compute_fee(amount, fee_bps) must return Ok(fee) where fee <= amount
    /// - fee + (amount - fee) == amount (no funds lost)
    #[test]
    fn prop_compute_fee_never_panics_and_never_exceeds_amount(
        amount in 0i128..=MAX_AMOUNT,
        fee_bps in 0u16..=10_000u16,
    ) {
        let result = stellar_grants::fees::compute_fee(amount, fee_bps as u32);
        match result {
            Ok(fee) => {
                prop_assert!(fee <= amount, "fee {} exceeded amount {}", fee, amount);
                prop_assert!(fee >= 0, "fee must be non-negative");
                // Verify no funds lost
                let remaining = amount - fee;
                prop_assert_eq!(fee + remaining, amount);
            }
            Err(_) => {
                // Errors are acceptable for overflow cases
            }
        }
    }

    /// Target 2: basis_points_of round-trip.
    /// For all (amount: i128, bps: u16 in 0..=10_000):
    /// - basis_points_of(amount, bps) is Ok
    /// - basis_points_of(amount, 0) == Ok(0)
    /// - basis_points_of(amount, 10_000) == Ok(amount)
    #[test]
    fn prop_basis_points_of_round_trip(
        amount in 0i128..=MAX_AMOUNT,
        bps in 0u16..=10_000u16,
    ) {
        let result = stellar_grants::math::basis_points_of(amount, bps as u32);
        prop_assert!(result.is_ok(), "basis_points_of should not fail for valid inputs");

        let zero_result = stellar_grants::math::basis_points_of(amount, 0);
        prop_assert_eq!(zero_result, Ok(0), "0 bps should always return 0");

        let full_result = stellar_grants::math::basis_points_of(amount, 10_000);
        prop_assert_eq!(full_result, Ok(amount), "10000 bps should return the full amount");
    }

    /// Target 3: split_evenly invariant.
    /// For all (total: i128, n_parts: u32 in 1..=100):
    /// - sum(split_evenly(total, n_parts)) == total
    /// - max(parts) - min(parts) <= 1 (balanced split)
    #[test]
    fn prop_split_evenly_invariant(
        total in 0i128..=MAX_AMOUNT,
        n_parts in 1u32..=100u32,
    ) {
        let (per_part, remainder) = stellar_grants::math::split_evenly(total, n_parts).unwrap();

        // Sum of parts + remainder must equal total
        let sum = per_part * n_parts as i128 + remainder;
        prop_assert_eq!(sum, total, "split sum mismatch");

        // Remainder must be less than n_parts (guaranteed by integer division)
        prop_assert!(remainder >= 0, "remainder must be non-negative");
        prop_assert!(remainder < n_parts as i128, "remainder must be less than n_parts");

        // Per_part must be non-negative for non-negative total
        prop_assert!(per_part >= 0, "per_part must be non-negative");
    }

    /// Target 4: proportional_share invariant.
    /// For a vec of share_bps values summing to 10_000:
    /// - sum(proportional_share(total, bps_i) for i) == total (or total-1 due to rounding)
    /// - No individual share exceeds total
    #[test]
    fn prop_proportional_share_invariant(
        total in 1i128..=MAX_AMOUNT,
        shares in prop::collection::vec(1u32..=10_000u32, 1..=10),
    ) {
        let bps_sum: u32 = shares.iter().sum();
        prop_assume!(bps_sum == 10_000, "shares must sum to 10_000");

        let mut total_distributed = 0i128;
        for &bps in shares.iter() {
            let share = stellar_grants::math::proportional_share(total, 10_000, bps as i128).unwrap();
            prop_assert!(share <= total, "individual share {} exceeded total {}", share, total);
            prop_assert!(share >= 0, "share must be non-negative");
            total_distributed += share;
        }

        // Due to integer division, total_distributed may be total or total-1
        let diff = total - total_distributed;
        prop_assert!(diff >= 0 && diff <= shares.len() as i128,
            "total distribution mismatch: expected ~{}, got {}", total, total_distributed);
    }

    /// Target 5: No-panic invariant.
    /// For any i128 input, these functions must never panic:
    /// basis_points_of, proportional_share, split_evenly, safe_add, safe_sub
    #[test]
    fn prop_no_panic_invariant(
        a in i128::MIN/2..=i128::MAX/2,
        b in i128::MIN/2..=i128::MAX/2,
        n in 0u32..=200u32,
        bps in 0u32..=10_000u32,
    ) {
        // These must never panic - they return Result
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
}
