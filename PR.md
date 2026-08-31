# test: expand refund / dispute / delegate / fuzz coverage (#976, #977, #978, #979)

This PR is **test-only** — it touches five files, all under
`contracts/contracts/stellar-grants/tests/`, and changes **no production code**.
It bundles four independent test-coverage issues that all concern the same
contract.

| Issue | Area | File(s) |
|-------|------|---------|
| #976 | Refund-policy variant coverage | `tests/test_refund_policy.rs` |
| #977 | Dispute-resolution fund verification | `tests/test_milestone_dispute.rs`, `tests/test_reputation_and_dispute_fee.rs` |
| #978 | Delegate cycle-detection coverage | `tests/test_delegate_voting.rs` |
| #979 | Fuzz tests that never called the crate | `tests/fuzz/mod.rs` |

---

## #976 — Refund policy tests cover only 2 of 5 `RefundPolicyType` variants

`test_refund_policy.rs` previously exercised only `TimeWeighted` and the
no-policy fallback. `ProportionalToRemaining`, `PenaltyOnCancel` and `NoRefund`
were never referenced by name, and `NoRefund` (funder must get **exactly 0**)
was the highest-risk untested branch.

**Added** — one dedicated end-to-end test per remaining variant, each driven
through the real `grant_cancel` → `refund::execute_refund` path with exact
funder/owner split assertions and a "the two payouts sum to the gross escrow"
check (no double-payout, no leak):

- `test_full_refund_policy_returns_entire_escrow_to_funder` — funder 1000, owner 0.
- `test_proportional_to_remaining_refunds_unreleased_escrow` — with 0 milestones
  paid out, all escrow is unreleased, so the funder is refunded in full.
- `test_penalty_on_cancel_applies_penalty_bps_and_splits_remainder` —
  `penalty_bps = 2000` → funder 800, owner 200.
- `test_no_refund_policy_sends_full_escrow_to_owner_and_zero_to_funder` —
  funder **0**, owner 1000.
- `test_no_refund_policy_with_min_refund_floor_still_pays_funder_the_floor` —
  a 10% `min_refund_pct_bps` floor is honored even under `NoRefund` (funder 100,
  owner 900).

**Note on the partial ratio:** `ProportionalToRemaining` with
`milestones_paid_out > 0` (a genuinely fractional refund) is not reachable from
the mainline entry points — nothing increments `milestones_paid_out` without
also completing (and thereby closing) the grant. That exact-fraction case is a
good candidate for an inline `#[test]` in `src/refund.rs`, but `cargo test --lib`
does not currently compile on `main` (see **CI status** below), so inline unit
tests could not be verified and were left out of this PR.

## #977 — Dispute resolution tests never verify actual fund movement

`dispute::resolve_dispute` performs a real `escrow::release` (to the grant
owner, contributor-win) or `escrow::release_to_funders` (to funders,
funder-win) of the disputed milestone's exact amount, but every dispute test
asserted only the returned `DisputeStatus` enum. A regression that resolved the
status correctly while paying the wrong party — or the wrong amount — would
have passed.

**Changed** — `test_dispute_and_resolve_flow`,
`test_dispute_raise_and_resolve_for_contributor` and
`test_dispute_raise_and_resolve_for_funder` now snapshot token balances around
`dispute_resolve` and assert:

- the **winning** party's balance increases by exactly `milestone.amount`,
- the **losing** party's balance is unchanged,
- `escrow_balance` drops by exactly `milestone.amount`.

These three tests were also **not executing the resolution path at all** on
`main` — they predate two later changes and failed early:

1. `milestone_vote` now requires a satisfied acceptance-criteria checklist
   (`Error(Contract, #76) RequiredCriteriaNotMet`) — added the same
   `checklist_define_criteria` / `checklist_submit` / `checklist_review_criterion`
   setup that `tests/integration_lifecycle.rs::setup_checklist` already uses.
2. `dispute_assign_arbiter` checks the global admin, which `initialize` does not
   set (`Error(Contract, #2)`) — added `client.set_global_admin(&admin, &admin)`,
   again matching the working `integration_lifecycle.rs` dispute test.

Both are test-only setup fixes; no assertion or business logic was changed.

## #978 — Delegate cycle-detection tests only cover the trivial 2-hop case

`delegate::would_create_cycle` special-cases self-delegation and walks a
delegation chain of arbitrary length, but `test_delegation_cycle_is_rejected`
only exercised a direct `A→B, B→A` cycle.

**Added:**

- `test_self_delegation_is_rejected` — `delegate_vote(r, r, …)` rejected.
- `test_three_node_delegation_cycle_is_rejected` — `A→B→C`, then `C→A` rejected.
- `test_long_indirect_delegation_cycle_is_rejected` — an 80-node chain closed
  into a cycle is rejected, exercising the full multi-hop walk and the
  visited-set bookkeeping.

**On the walk-limit boundary:** `would_create_cycle` hard-codes
`max_chain_length = 256`, but a single on-chain invocation can only touch ~100
distinct ledger entries, so a cycle walk over more than ~100 delegation records
hits `Error(Budget, ExceededLimit)` ("total footprint ledger entries") long
before it reaches 256 — an exact-256 test is not runnable. The 80-node test
sits just under that real ceiling. The `max_chain_length` / long-valid-chain
gap remains tracked in #953.

## #979 — `tests/fuzz/mod.rs` "fuzz" tests never call the actual crate

`prop_grant_create_no_overflow`, `prop_grant_create_total_amount_validation`,
`prop_cancel_refund_sum_equals_escrow`, `prop_release_balance_conservation` and
`prop_quorum_bounds` only asserted properties about arithmetic **reimplemented
inline in the test**, so a real crate regression could never fail them.

**Rewritten** to drive `StellarGrantsContractClient` / real crate functions
with fuzzed inputs (case counts dialled down since each case spins up a fresh
contract), following the `fees_fuzz.rs` pattern:

| New name | What real code it now exercises |
|----------|--------------------------------|
| `prop_grant_create_rejects_overflowing_milestone_math` | `internal_grant_create`'s `checked_mul(...).ok_or(InvalidInput)` — asserts a clean `Err(Ok(_))` contract error, never a host trap from an unchecked multiply |
| `prop_grant_create_enforces_total_covers_milestones` | `internal_grant_create`'s `total_amount < total_required` check + the stored grant echoing the inputs |
| `prop_cancel_refund_sum_equals_escrow` | `escrow::refund_all`'s proportional split (incl. "last funder gets the remainder") via `grant_cancel` |
| `prop_release_balance_conservation` | `escrow::release` + `refund_all` + `fees::compute_fee` via `grant_complete` — `owner_payout + funder_refund == gross escrow`, escrow fully drained |
| `prop_quorum_bounds` | `governance::quorum_reached` (`approvals * 2 > reviewer_count`) via real milestone voting |

The eight pure-math `prop_basis_points_*` / `prop_proportional_share_*` tests in
the same file already called real crate functions and are unchanged.

---

## CI status — the `Contracts (Rust)` job is already red on `main`

**Every CI run on `main` for the last week fails at the `Clippy (WASM, lib
only)` step** (e.g. runs `33364533301`, `33302780551`, …). The `stellar-grants`
library has ~34 pre-existing compile errors from other contributors' recently
merged features:

- broken hook-payload construction using `soroban_sdk::Vec<u8>` (which the
  current soroban-sdk doesn't support) in `src/lib.rs` — 6 sites, from
  `f0f444f`;
- four `ContractError` variants referenced but never declared
  (`InsufficientClawbackAllowance`, `TooManyPublicReviews`, `DaoVoteRequired`,
  `SwapNotImplemented`) in `clawback.rs` / `open_review.rs` / `params.rs` /
  `token_swap.rs`;
- a removed `env.invoker()` call in `src/lockup.rs`;
- one type mismatch in `src/params.rs`.

None of this relates to these four test-only issues, and per the issue scope
this PR does **not** fix it. Consequences:

- The `Contracts (Rust)` job on this PR will show the **same** red X it shows on
  every recent merge into `main` — it fails at clippy, before the `Test` step
  this PR's changes live in.
- `backend`, `frontend` and `client-sdk` CI jobs are unaffected by this PR and
  continue to pass.

### How the new tests were verified

Because the library must compile to build any test target, the five affected
test binaries were built and run locally against a **throwaway local patch**
that fixes only the ~34 compile errors above (not included in this PR). Results:

```
test_refund_policy              6 passed, 1 failed   (see pre-existing failures)
test_delegate_voting            9 passed, 0 failed
test_milestone_dispute          2 passed, 0 failed
test_reputation_and_dispute_fee 4 passed, 0 failed
fuzz_amounts (this PR's 5)      5 passed, 0 failed
```

`cargo fmt --all -- --check` passes.

### Pre-existing test failures (NOT introduced by this PR)

Running the affected binaries surfaces failures that exist on `main`
independently of this change and are out of scope here:

- `test_refund_policy::test_time_weighted_refund_policy_on_partial_cancel` —
  the `TimeWeighted` split no longer produces the expected 500/500
  (unmodified by this PR).
- `fuzz::prop_basis_points_partition_never_exceeds_total` and
  `fees_fuzz::prop_proportional_share_sum_invariant` — pre-existing
  `math::basis_points_of` rounding-invariant failures (unmodified by this PR).

---

Closes #976
Closes #977
Closes #978
Closes #979
