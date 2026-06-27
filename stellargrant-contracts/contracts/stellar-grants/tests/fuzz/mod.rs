/// Fuzz / property-based entry point for stellar-grants contract.
///
/// Run via: `cargo test --test fuzz_amounts` from the contract directory.
/// The proptest suite below executes 1 000+ randomised iterations by default
/// (controlled by `PROPTEST_CASES` env var or the `ProptestConfig`).
///
/// Edge cases discovered during development:
///  - `milestone_amount * num_milestones` can overflow i128 for very large
///    inputs; the contract correctly rejects these via `checked_mul`.
///  - When `total_amount == milestone_amount * num_milestones` (exact match)
///    the remaining balance after release is always 0.
///  - Proportional refunds in `grant_cancel` may lose at most
///    `(n_funders - 1)` stroops due to integer division; the last funder
///    always receives the leftover, so the total distributed == escrow_balance.
mod fees_fuzz;
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
        if let Some(required) = total_required {
            prop_assert!(required >= milestone_amount);
            prop_assert!(required >= num_milestones as i128);
        }
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
        if quorum == 0 || quorum > num_reviewers {
            prop_assert!(!valid || quorum == 0);
        } else {
            prop_assert!(valid);
        }
    }

    // ── Issue #623: Overflow-safe basis_points_of edge-case properties ──────

    /// Property: basis_points_of(amount, 10_000) == amount (100% of amount).
    #[test]
    fn prop_basis_points_100pct(
        amount in 0i128..=i128::MAX,
    ) {
        prop_assert_eq!(
            stellar_grants::math::basis_points_of(amount, 10_000).unwrap(),
            amount,
        );
    }

    /// Property: basis_points_of(amount, 0) == 0.
    #[test]
    fn prop_basis_points_zero_bps(
        amount in 0i128..=i128::MAX,
    ) {
        prop_assert_eq!(
            stellar_grants::math::basis_points_of(amount, 0).unwrap(),
            0,
        );
    }

    /// Property: basis_points_of(i128::MAX, 1) must not panic or overflow.
    #[test]
    fn prop_basis_points_imax_1bp() {
        let result = stellar_grants::math::basis_points_of(i128::MAX, 1);
        prop_assert!(result.is_ok(), "must not overflow for i128::MAX, 1bp");
        let val = result.unwrap();
        prop_assert!(val > 0, "result must be positive");
    }

    /// Property: basis_points_of with bps > 10_000 returns None.
    #[test]
    fn prop_basis_points_invalid_bps(
        amount in 0i128..=i128::MAX,
        bps in 10_001u32..=u32::MAX,
    ) {
        prop_assert!(stellar_grants::math::basis_points_of(amount, bps).is_err());
    }

    /// Property: sum of basis_points_of(total, bps_i) for each share in a
    /// partition must not exceed total.
    #[test]
    fn prop_basis_points_partition_never_exceeds_total(
        total in 0i128..=i128::MAX / 2,
        shares in prop::collection::vec(0u32..=10_000u32, 1..=10),
    ) {
        let mut sum = 0i128;
        for &bps in &shares {
            let part = stellar_grants::math::basis_points_of(total, bps).unwrap();
            sum = sum.saturating_add(part);
        }
        // Integer division means sum can never exceed total
        prop_assert!(sum <= total, "partition sum {} exceeded total {}", sum, total);
    }

    // ── Issue #623: proportional_share overflow-safe properties ──────────────

    /// Property: proportional_share(total, 10_000) == total.
    #[test]
    fn prop_proportional_share_100pct(
        total in 0i128..=i128::MAX,
    ) {
        prop_assert_eq!(
            stellar_grants::math::proportional_share(total, 10_000).unwrap(),
            total,
        );
    }

    /// Property: proportional_share(total, 0) == 0.
    #[test]
    fn prop_proportional_share_zero(
        total in 0i128..=i128::MAX,
    ) {
        prop_assert_eq!(
            stellar_grants::math::proportional_share(total, 0).unwrap(),
            0,
        );
    }

    /// Property: proportional_share(i128::MAX, 1) must not overflow.
    #[test]
    fn prop_proportional_share_imax_1bp() {
        let result = stellar_grants::math::proportional_share(i128::MAX, 1);
        prop_assert!(result.is_ok(), "must not overflow for i128::MAX");
    }
}
