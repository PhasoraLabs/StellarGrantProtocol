# Fix four issues in stellar-grants waitlist/registry/provenance modules

This PR bundles four independent fixes to the `stellar-grants` Soroban contract: two security gaps in the waitlist module (missing authorization and a costless mass-enrollment DoS), an integer-overflow panic in provenance pagination, and an O(n) performance regression in contributor registration.

---

## #922 — Add `require_auth` checks to waitlist module

**File:** `contracts/contracts/stellar-grants/src/waitlist.rs`

None of `waitlist.rs`'s three state-changing functions called `require_auth()`. `configure()` only checked address equality against the grant's stored owner, so a caller could pass the real owner's (public) address as the `owner` parameter and sabotage their waitlist config (e.g. set `max_waitlist_size = 0`) without ever holding the owner's key. `join()` and `leave()` had no auth check at all, letting anyone enroll or evict arbitrary third-party addresses on any grant's waitlist. Every other module in this codebase (treasury, syndication, emergency, oracle, lockup, clawback, ...) enforces `require_auth()` somewhere in its call chain; `waitlist.rs` was the sole exception.

- Added `owner.require_auth()` to `configure()` and `applicant.require_auth()` to `join()`/`leave()`.
- New tests: a stranger can't reconfigure another owner's waitlist by passing the owner's address, and a third party can't enroll or remove an address via `join()`/`leave()` without that address's own signature.

## #923 — Rate-limit waitlist join to prevent exhaustion DoS

**File:** `contracts/contracts/stellar-grants/src/waitlist.rs` (+ `constants.rs`, `types.rs`, `rate_limit.rs`)

Joining a waitlist required no deposit, stake, or reputation threshold. Even with `require_auth()` in place (previous fix), an attacker could still script one signed `join_waitlist` call per freshly generated address until `max_waitlist_size` is reached, permanently exhausting a grant's waitlist for legitimate applicants at only the cost of transaction fees.

- Gated `join()` behind `rate_limit::check_and_increment`, keyed by applicant address — the same mechanism already used for grant/milestone/bounty creation and contributor registration.
- Added a `WaitlistJoin` `RateLimitAction` variant (5 joins/hour by default, admin-exempt like every other rate-limited action).
- New tests: an applicant address is throttled after `RATE_LIMIT_WAITLIST_JOIN_MAX` joins across distinct grants within one window, while a single legitimate join is unaffected.

## #921 — Use checked pagination arithmetic in `provenance::get_by_address`

**File:** `contracts/contracts/stellar-grants/src/provenance.rs`

Unlike `pagination::paginate` — the shared helper this codebase documents as the canonical place for offset/limit pagination — `get_by_address` reimplemented pagination inline. It didn't clamp `limit` to `MAX_PAGE_SIZE`, and computed `offset + limit` with plain `u32` addition before checking against `len`. A caller passing large `offset`/`limit` values (both near `u32::MAX`) would overflow that addition and panic, violating this codebase's "never panic!, return Result" convention.

- Replaced the inline pagination logic with `pagination::paginate`, which clamps to `MAX_PAGE_SIZE` and uses saturating arithmetic.
- New test calls `get_by_address` with `offset`/`limit` near `u32::MAX` and confirms it returns cleanly instead of panicking.

## #920 — Remove redundant linear scan from contributor registration

**File:** `contracts/contracts/stellar-grants/src/registry.rs`

Every call to `register_contributor` loaded the entire global contributor index into memory and scanned it linearly for a duplicate before appending, making per-call cost O(n) and total registration cost across n users O(n²). The scan was also redundant: `lib.rs`'s `contributor_register` entrypoint already performs an O(1) duplicate check via `Storage::get_contributor` before ever calling into this function.

- Removed the linear scan; `register_contributor` now relies on the caller's O(1) check, documented in the function's doc comment.
- New test registers many pre-existing contributors and confirms adding one more is O(1) — no scan of the whole index.

---

## Test infrastructure fixes (pre-existing, unrelated to these issues)

While making the above changes testable, two pre-existing bugs surfaced: `waitlist.rs`'s and `registry.rs`'s unit tests called `Storage`-backed functions directly on a bare `Env`, which this soroban-sdk version rejects outside of `env.as_contract(...)`. This predates all four issues above (confirmed via `git stash`) and would otherwise have blocked `cargo test` for this PR. Fixed by registering the contract and wrapping each test body accordingly — no production code changes.

## Verification

```
cargo fmt --check
cargo clippy --lib -p stellar-grants -- -D warnings
cargo test --lib -p stellar-grants -- waitlist:: provenance:: registry:: rate_limit:: pagination:: constants::
```

All 54 tests across the six touched modules pass clean, including every new test added above. `cargo clippy --lib --tests -- -D warnings` reports zero errors in any file touched by this PR (all remaining clippy errors are pre-existing and confined to files this PR doesn't touch).

**Note on the wider test suite:** the same pre-existing `env.as_contract(...)` issue affects dozens of *other*, untouched modules across the crate — running the full `cargo test --lib` still fails several hundred pre-existing tests for reasons entirely unrelated to this PR. Out of scope here.

---

Closes #922
Closes #923
Closes #921
Closes #920
