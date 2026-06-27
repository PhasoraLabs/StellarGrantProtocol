/// Property-based tests for quadratic funding allocation invariants (#626).
///
/// Run via: `cargo test --test qf_properties` from the workspace root.
/// Uses proptest to verify mathematical properties of the QF formula
/// across randomly generated inputs (1000+ cases per property).
use proptest::prelude::*;

/// Integer square root using Newton's method — reference implementation.
/// Returns floor(sqrt(n)).
fn isqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Simulated QF score: sum of sqrt of contributions (each contribution is
/// measured in integer units). This mirrors the core of a standard QF formula
/// where the allocation is proportional to (sum_i(sqrt(c_i)))^2.
///
/// Each element in `contributions` is the contribution amount for one donor.
/// We use integer sqrt per contribution, then square the total.
fn qf_score(contributions: &[u128]) -> u128 {
    let sum_sqrt: u128 = contributions.iter().map(|&c| isqrt(c)).sum();
    sum_sqrt * sum_sqrt
}

/// Compute allocation for a project given its contributions and the total
/// matching pool. Allocation is proportional to the project's QF score
/// relative to the total QF score across all projects.
fn compute_allocations(
    project_scores: &[u128],
    pool_size: u128,
) -> Vec<u128> {
    let total_score: u128 = project_scores.iter().sum();
    if total_score == 0 {
        return project_scores.iter().map(|_| 0u128).collect();
    }
    project_scores
        .iter()
        .map(|&score| score * pool_size / total_score)
        .collect()
}

/// Credit cost for casting `votes` unit votes in QV: cost = votes^2.
fn credit_cost(votes: u32) -> u32 {
    votes.saturating_mul(votes)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]

    // ── Property 1: Total allocations ≤ matching pool ────────────────────────

    /// For any set of project scores and any pool size, the sum of
    /// allocations must never exceed the matching pool.
    #[test]
    fn prop_total_allocations_never_exceed_pool(
        scores in prop::collection::vec(0u128..=1_000_000_000u128, 1..=10),
        pool_size in 1u128..=10_000_000_000u128,
    ) {
        let allocations = compute_allocations(&scores, pool_size);
        let total_allocated: u128 = allocations.iter().sum();
        prop_assert!(
            total_allocated <= pool_size,
            "total allocated {} exceeded pool {}",
            total_allocated,
            pool_size,
        );
    }

    // ── Property 2: Monotonicity — more contributors → higher QF score ──────

    /// If project P has n2 > n1 contributors each donating the same amount D,
    /// then qf_score(n2) > qf_score(n1).
    #[test]
    fn prop_monotonicity_more_contributors_higher_score(
        d in 1u128..=1_000_000u128,
        n1 in 1u32..=100u32,
        delta in 1u32..=100u32,
    ) {
        let n2 = n1 + delta;
        let contribs_n1: Vec<u128> = vec![d; n1 as usize];
        let contribs_n2: Vec<u128> = vec![d; n2 as usize];
        let score1 = qf_score(&contribs_n1);
        let score2 = qf_score(&contribs_n2);
        prop_assert!(
            score2 > score1,
            "score with {} contributors ({}) was not greater than with {} ({})",
            n2, score2, n1, score1,
        );
    }

    // ── Property 3: Diminishing marginal returns for single large donor ─────

    /// Two donors each contributing D/2 must outscore one donor contributing D.
    /// sqrt(D/2) + sqrt(D/2) > sqrt(D), so the squared sum is larger.
    #[test]
    fn prop_diminishing_marginal_returns(
        d in 2u128..=1_000_000_000u128,
    ) {
        let half = d / 2;
        let score_single = qf_score(&[d]);
        let score_two = qf_score(&[half, d - half]);
        prop_assert!(
            score_two > score_single,
            "two donors at {}/{} (score={}) did not outscore one at {} (score={})",
            half, d - half, score_two, d, score_single,
        );
    }

    // ── Property 4: Integer square root approximation error bounded ──────────

    /// For all n in 0..u128::MAX, isqrt(n) must satisfy:
    ///   s * s <= n  AND  (s+1)*(s+1) > n
    #[test]
    fn prop_isqrt_is_exact_floor(n in 0u128..=u128::MAX) {
        let s = isqrt(n);
        prop_assert!(
            s * s <= n,
            "isqrt({}) = {} but {}*{} = {} > {}",
            n, s, s, s, s * s, n,
        );
        // (s+1)^2 > n, but only check when s < u128::MAX to avoid overflow
        if s < u128::MAX {
            let s1 = s + 1;
            prop_assert!(
                s1 * s1 > n,
                "isqrt({}) = {} but ({}+1)^2 = {} <= {}",
                n, s, s, s1 * s1, n,
            );
        }
    }

    // ── Property 5: Zero contributions → zero allocation ─────────────────────

    /// A project with zero contributions must receive zero allocation
    /// regardless of pool size.
    #[test]
    fn prop_zero_contributions_zero_allocation(
        pool_size in 1u128..=10_000_000_000u128,
    ) {
        let allocations = compute_allocations(&[0], pool_size);
        prop_assert_eq!(allocations[0], 0);
    }

    /// When all projects have zero scores, all allocations must be zero.
    #[test]
    fn prop_all_zero_scores_zero_allocations(
        num_projects in 1u32..=10u32,
        pool_size in 1u128..=10_000_000_000u128,
    ) {
        let scores = vec![0u128; num_projects as usize];
        let allocations = compute_allocations(&scores, pool_size);
        for alloc in &allocations {
            prop_assert_eq!(*alloc, 0u128);
        }
    }

    // ── Property: credit_cost is monotonically non-decreasing ────────────────

    /// credit_cost(v) must be non-decreasing in v.
    #[test]
    fn prop_credit_cost_monotonic(
        v1 in 0u32..=1000u32,
        delta in 0u32..=1000u32,
    ) {
        let v2 = v1.saturating_add(delta);
        prop_assert!(
            credit_cost(v2) >= credit_cost(v1),
            "credit_cost({}) = {} < credit_cost({}) = {}",
            v2, credit_cost(v2), v1, credit_cost(v1),
        );
    }

    /// credit_cost(0) == 0
    #[test]
    fn prop_credit_cost_zero() {
        prop_assert_eq!(credit_cost(0), 0);
    }

    // ── Property: QF score is monotonically increasing per contribution ─────

    /// Adding a positive contribution to a project always increases its QF score.
    #[test]
    fn prop_qf_score_increases_with_contribution(
        existing in prop::collection::vec(1u128..=100_000u128, 0..=5),
        new_contrib in 1u128..=100_000u128,
    ) {
        let score_before = qf_score(&existing);
        let mut with_new = existing.clone();
        with_new.push(new_contrib);
        let score_after = qf_score(&with_new);
        prop_assert!(
            score_after > score_before,
            "adding contribution {} did not increase score ({} -> {})",
            new_contrib, score_before, score_after,
        );
    }
}
