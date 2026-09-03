# Security Model — StellarGrant Contracts

## Privilege Vectors

### Grant Owner
- Can **create** grants with milestone structure and reviewers
- Can **submit** milestones with proof URLs
- Can **cancel** active grants (triggers proportional refunds)
- Cannot vote on their own milestones
- Cannot modify reviewers after creation

### Reviewers
- Can **approve** or **reject** submitted milestones
- Each reviewer votes exactly once per milestone (double-vote prevention)
- Must meet minimum stake requirement when staking is configured
- Subject to slashing by admin for malicious behavior

### Funders
- Can **fund** active grants (tokens held in escrow)
- Receive **proportional refunds** on grant cancellation or completion remainder
- Cannot withdraw funds unilaterally

### Admin
- Can **set staking config** (minimum stake, treasury address)
- Can **slash** malicious reviewers
- Can **set identity oracle** for KYC verification
- Cannot modify grants, vote on milestones, or move escrow funds directly

## Trust Assumptions

1. **Soroban runtime**: We trust the Stellar/Soroban VM for correct execution, storage isolation, and cryptographic primitives.
2. **Token contracts**: We trust SAC token contracts (USDC, XLM) to correctly implement `transfer()` semantics.
3. **Reviewer honesty**: The quorum mechanism (majority vote) assumes that at least 51% of reviewers act honestly.
4. **Admin integrity**: Admin operations (slashing, KYC oracle) are trusted. In production, admin should be a multisig or DAO.
5. **Identity oracle**: When KYC is enabled, we trust the oracle contract to correctly report verification status.

## Audit Checklist

- [ ] All `require_auth()` calls are present on state-mutating functions
- [ ] Overflow protection via `checked_add` on all balance operations
- [ ] Double-vote prevention in `milestone_vote` and `milestone_reject`
- [ ] Milestone index bounds checking (`idx < total_milestones`)
- [ ] State machine enforcement (Active → Completed/Cancelled only)
- [ ] Proportional refund math verified (no rounding exploits)
- [ ] Token transfer ordering: state updated before/after external calls
- [ ] No unbounded iteration (milestones capped at 100, batch funding at 20)
- [ ] Storage keys cannot collide across different data types
- [ ] Reviewer staking prevents zero-stake voting
- [ ] Slashed funds correctly routed to treasury (not burned)
