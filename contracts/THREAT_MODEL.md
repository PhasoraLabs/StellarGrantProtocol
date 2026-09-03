# Threat Model — StellarGrant Contracts

## Attack Vector Analysis

### 1. Reviewer Collusion (Milestone Approval Fraud)

**Threat**: A group of colluding reviewers approves a fraudulent milestone to release escrow funds.

**Quorum formula**: `threshold = (total_reviewers / 2) + 1`

**Mitigation**:
- Reviewers must stake tokens to participate (staking mechanism, issue #42)
- Malicious reviewers can be slashed by admin, losing their entire stake
- Economic deterrent: cost of staking > potential gain from fraud when stake >= milestone payout
- Grant creators choose reviewers, creating a web-of-trust selection process

**Proof of immunity**: For `n` reviewers, an attacker needs `(n/2)+1` colluding reviewers. With staking at `S` per reviewer, the attack cost is `((n/2)+1) * S`. If `S >= milestone_amount`, the attack is economically irrational since the total stake at risk exceeds the payout.

---

### 2. Escrow Drain via Overflow

**Threat**: An attacker manipulates funding amounts to cause integer overflow, resulting in incorrect escrow balances.

**Mitigation**:
- All balance additions use `checked_add()` which returns `None` on overflow
- `checked_add` failure maps to `ContractError::InvalidInput`, reverting the transaction
- Soroban's `i128` provides a range of ±1.7×10³⁸, far exceeding any realistic token amount
- The `overflow-checks = true` Cargo profile catches any unchecked arithmetic at runtime

**Proof**: Given `i128::MAX ≈ 1.7 × 10³⁸` and realistic token supplies (< 10¹⁸ stroops), overflow is impossible in normal operation. `checked_add` provides defense-in-depth.

---

### 3. Unbounded Iteration Gas Exhaustion

**Threat**: An attacker creates a grant with maximum milestones or funds from many addresses, causing gas exhaustion during iteration (e.g., refund loops).

**Mitigation**:
- Milestones capped at 100 (`num_milestones` validation in `grant_create`)
- Batch funding capped at 20 operations per call
- Refund loops iterate over `funders` vector which grows with each unique funder
- Soroban CPU instruction limits provide a hard ceiling on computation

**Bound analysis**: With 100 milestones × 100 funders, the maximum iteration count is bounded and well within Soroban's per-invocation CPU budget (~100M instructions).

---

### 4. Unauthorized State Transition

**Threat**: A non-owner attempts to cancel a grant, or a non-reviewer votes on a milestone.

**Mitigation**:
- `grant_cancel` checks `owner == caller` via `require_auth()`
- `milestone_vote` and `milestone_reject` validate `reviewer ∈ grant.reviewers`
- `milestone_submit` validates `owner == caller`
- All auth checks happen before any state mutation (check-effects-interactions pattern)

**Proof**: Soroban's `require_auth()` is enforced by the VM — signatures are verified at the protocol level, not in contract code. A forged auth would require breaking Ed25519.

---

### 5. Double Refund on Cancellation

**Threat**: Calling `grant_cancel` multiple times to drain escrow.

**Mitigation**:
- First call sets `status = Cancelled` and refunds all funders
- Second call fails at `if grant.status != GrantStatus::Active` check
- State transition is atomic: status update and refunds happen in the same invocation
- No partial cancel states exist

**Proof**: The state machine is `Active → {Cancelled, Completed}` with no reverse transitions. Once `status != Active`, all cancel/complete paths are blocked.

## Dangerous Components (Audit Priority)

| Component | Risk | File | Lines |
|-----------|------|------|-------|
| Token transfer in `grant_fund` | Escrow accounting | lib.rs | `grant_fund()` |
| Proportional refund math | Rounding errors | lib.rs | `grant_cancel()`, `grant_complete()` |
| Funder iteration in refunds | Gas exhaustion | lib.rs | cancel/complete loops |
| Quorum threshold calc | Off-by-one | lib.rs | `milestone_vote()` |
| Batch funding iteration | Gas + overflow | lib.rs | `fund_batch()` |
| Reviewer stake/slash flow | Fund routing | lib.rs | `stake_to_review()`, `slash_reviewer()` |
