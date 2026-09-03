# Stellar Grants Event Schema

This contract emits typed Soroban `#[contractevent]` events (plus a small set of
legacy `env.events().publish(...)` events). This document is generated from the
actual event structs and publish call sites in `src/` so an indexer can follow
on-chain activity without guessing names.

## Event structure

Typed `#[contractevent]` events in this crate do **not** use `#[topic]` field
attributes. The event identifier is the **struct name**. All listed fields are
in the event body (payload). There is no `event_version` field on these structs.

Legacy `env.events().publish(topics, data)` events are documented separately at
the end; their topic tuples are the first argument to `publish`.

The following names from an earlier draft of this file **are not emitted** and
must not be indexed:

- `ContractInitialized` — initialization does not emit a dedicated event
- `ContractUpgraded` / `ContractWasmUpgraded` — WASM/config upgrades are
  represented by **ContractMigrated** (see `events.rs`)
- `GrantMetadataUpdated` — there is no metadata-update function or event
- `QuorumReached` — milestone voting does not emit a quorum event; watch
  **MilestoneVoted** / **MilestoneStatusChanged** / **MilestonePaid** instead

## Typed `#[contractevent]` events

### Contract lifecycle

- **ContractMigrated**: Storage/contract migration ran (`events.rs`). Replaces the obsolete ContractUpgraded / ContractWasmUpgraded names.
  - Payload: `from_version: u32`, `to_version: u32`, `run_by: Address`, `timestamp: u64`
- **ContractPaused**: Protocol paused by admin.
  - Payload: `admin: Address`, `reason: String`, `timestamp: u64`
- **ContractUnpaused**: Protocol unpaused by admin.
  - Payload: `admin: Address`, `timestamp: u64`
- **ParamChanged**: A protocol parameter was changed.
  - Payload: `key: Symbol`, `set_by: Address`, `timestamp: u64`
- **IndexCapReached**: A grant index list hit `MAX_INDEX_ENTRIES` and the new id was dropped.
  - Payload: `grant_id: u64`, `timestamp: u64`

### Grants

- **GrantCreated**: Grant created.
  - Payload: `grant_id: u64`, `owner: Address`, `title: String`, `total_amount: i128`, `timestamp: u64`
- **GrantFunded**: Grant funded.
  - Payload: `grant_id: u64`, `funder: Address`, `amount: i128`, `new_balance: i128`, `timestamp: u64`
- **GrantCancelled**: Grant cancelled.
  - Payload: `grant_id: u64`, `owner: Address`, `reason: String`, `refund_amount: i128`, `timestamp: u64`
- **GrantCompleted**: Grant completed.
  - Payload: `grant_id: u64`, `total_paid: i128`, `remaining_balance: i128`, `timestamp: u64`
- **GrantForked**: Grant forked into a new grant.
  - Payload: `original_grant_id: u64`, `forked_grant_id: u64`, `timestamp: u64`
- **PayerReceipt**: Machine-readable receipt for a funder contribution.
  - Payload: `grant_id: u64`, `funder: Address`, `token: Address`, `amount: i128`, `memo: Option<String>`, `timestamp: u64`
- **PayeeReceipt**: Machine-readable receipt for a payout.
  - Payload: `grant_id: u64`, `recipient: Address`, `token: Address`, `amount: i128`, `milestone_idx: Option<u32>`, `timestamp: u64`
- **RefundIssued**: Refund issued to a funder.
  - Payload: `grant_id: u64`, `funder: Address`, `amount: i128`
- **RefundExecuted**: Refund policy execution completed.
  - Payload: `grant_id: u64`, `funder: Address`, `amount: i128`
- **FeeCollected**: Protocol fee collected on a payout.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `fee_amount: i128`, `token: Address`, `treasury: Address`, `timestamp: u64`

### Milestones

- **MilestoneSubmitted**: Milestone submitted.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `description: String`, `timestamp: u64`
- **MilestoneVoted**: Reviewer voted on a milestone.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `reviewer: Address`, `approve: bool`, `feedback: Option<String>`, `timestamp: u64`
- **MilestoneRejected**: Milestone rejected.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `reviewer: Address`, `reason: String`, `timestamp: u64`
- **MilestoneStatusChanged**: Milestone state changed (also emitted when a grant timer fires).
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `new_state: MilestoneState`, `timestamp: u64`
- **MilestonePaid**: Milestone payout executed.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `amount: i128`, `timestamp: u64`
- **ExtensionRequested**: `milestone_extension.rs` — deadline extension requested.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `requested_by: Address`, `new_deadline: u64`
- **ExtensionApproved**: Deadline extension approved.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `new_deadline: u64`
- **ExtensionDenied**: Deadline extension denied.
  - Payload: `grant_id: u64`, `milestone_idx: u32`
- **ExtensionWithdrawn**: Deadline extension withdrawn.
  - Payload: `grant_id: u64`, `milestone_idx: u32`
- **ChecklistSubmitted**: `checklist.rs` — acceptance checklist submitted.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `submitted_at: u64`
- **CriterionReviewed**: A checklist criterion was reviewed.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `criterion_idx: u32`, `approved: bool`

### Contributors, reviewers, and reputation

- **ContributorRegistered**: Contributor registered.
  - Payload: `contributor: Address`, `name: String`, `timestamp: u64`
- **ReputationUpdated**: Contributor reputation updated.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `contributor: Address`, `new_reputation_score: u64`, `total_earned: i128`, `timestamp: u64`
- **ReviewerApproved**: Reviewer approved at protocol level.
  - Payload: `reviewer: Address`, `approved_by: Address`, `timestamp: u64`
- **ReviewerRevoked**: Reviewer revoked at protocol level.
  - Payload: `reviewer: Address`, `revoked_by: Address`, `timestamp: u64`
- **ReviewerAddedToGrant**: Reviewer added to a grant.
  - Payload: `grant_id: u64`, `reviewer: Address`, `timestamp: u64`
- **ReviewerRemovedFromGrant**: Reviewer removed from a grant.
  - Payload: `grant_id: u64`, `reviewer: Address`, `timestamp: u64`
- **PublicReviewSubmitted**: Open/public review submitted.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `reviewer: Address`, `timestamp: u64`
- **ReviewMarkedHelpful**: A public review was marked helpful.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `reviewer: Address`, `voter: Address`, `timestamp: u64`
- **ContributorVerified**: `contributor_verification.rs`.
  - Payload: `subject: Address`, `verifier: Address`, `level: VerificationLevel`, `expires_at: Option<u64>`
- **VerificationRevoked**: Contributor verification revoked.
  - Payload: `subject: Address`, `revoked_by: Address`
- **BadgeAwarded**: `badge.rs`.
  - Payload: `contributor: Address`, `badge_type: BadgeType`, `grant_id: Option<u64>`, `awarded_at: u64`

### Disputes and arbitration

- **DisputeRaised**: Dispute raised on a milestone.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `raised_by: Address`, `timestamp: u64`
- **ArbiterAssigned**: Arbiter assigned to a dispute.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `arbiter: Address`, `timestamp: u64`
- **ArbiterVoted**: Arbiter voted on a dispute.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `arbiter: Address`, `favor_contributor: bool`, `timestamp: u64`
- **DisputeResolved**: Dispute resolved.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `resolved_for_contributor: bool`, `timestamp: u64`
- **DisputeCancelled**: Dispute cancelled.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `cancelled_by: Address`, `timestamp: u64`
- **ArbiterJoined**: `arbitration_pool.rs` — arbiter joined the pool.
  - Payload: `arbiter: Address`, `stake: i128`
- **ArbiterLeft**: Arbiter left the pool.
  - Payload: `arbiter: Address`, `returned: i128`
- **PanelAssigned**: Arbitration panel assigned.
  - Payload: `case_id: u32`, `dispute_id: u32`, `panel_size: u32`
- **ArbiterVoteCast**: Pool arbiter voted on a case.
  - Payload: `case_id: u32`, `arbiter: Address`, `favor_contributor: bool`
- **CaseFinalized**: Arbitration case finalized.
  - Payload: `case_id: u32`, `outcome: bool`
- **RewardsSettled**: Arbitration rewards/slashes settled.
  - Payload: `case_id: u32`, `total_slashed: i128`

### Clawback

- **ClawbackInitiated**: Clawback initiated.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `target: Address`, `amount: i128`, `token: Address`, `initiated_by: Address`, `dispute_window_ends: u64`, `timestamp: u64`
- **ClawbackApproved**: Clawback approved.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `approver: Address`, `timestamp: u64`
- **ClawbackDisputed**: Clawback disputed.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `disputed_by: Address`, `timestamp: u64`
- **ClawbackExecuted**: Clawback executed.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `amount_recovered: i128`, `token: Address`, `treasury: Address`, `timestamp: u64`
- **ClawbackCancelled**: Clawback cancelled.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `cancelled_by: Address`, `timestamp: u64`
- **ClawbackAllowanceAuthorized**: Clawback token allowance authorized.
  - Payload: `grant_id: u64`, `contributor: Address`, `token: Address`, `amount: i128`, `live_until_ledger: u32`, `timestamp: u64`

### Treasury and DAO

- **TreasuryDeposited**: Treasury deposit.
  - Payload: `token: Address`, `from: Address`, `amount: i128`, `new_balance: i128`, `timestamp: u64`
- **TreasuryWithdrawn**: Treasury withdrawal.
  - Payload: `token: Address`, `to: Address`, `amount: i128`, `new_balance: i128`, `admin: Address`, `timestamp: u64`
- **TreasuryReallocated**: Treasury reallocation.
  - Payload: `from_token: Address`, `to_token: Address`, `amount: i128`, `admin: Address`, `timestamp: u64`
- **DaoProposalCreated**: DAO proposal created.
  - Payload: `proposal_id: u64`, `proposer: Address`, `title: String`, `voting_deadline: u64`, `timestamp: u64`
- **DaoVoteCast**: DAO vote cast.
  - Payload: `proposal_id: u64`, `voter: Address`, `support: bool`, `weight: u64`, `timestamp: u64`
- **DaoProposalFinalized**: DAO proposal finalized.
  - Payload: `proposal_id: u64`, `passed: bool`, `votes_for: u64`, `votes_against: u64`, `timestamp: u64`
- **DaoProposalExecuted**: DAO proposal executed.
  - Payload: `proposal_id: u64`, `executed_by: Address`, `timestamp: u64`
- **DaoProposalCancelled**: DAO proposal cancelled.
  - Payload: `proposal_id: u64`, `cancelled_by: Address`, `timestamp: u64`

### Bounties

- **BountyCreated**: Bounty created.
  - Payload: `bounty_id: u64`, `owner: Address`, `title: String`, `prize_amount: i128`, `submission_deadline: u64`, `timestamp: u64`
- **BountySubmissionReceived**: Bounty submission received.
  - Payload: `bounty_id: u64`, `submitter: Address`, `timestamp: u64`
- **BountyAwarded**: Bounty awarded.
  - Payload: `bounty_id: u64`, `winner: Address`, `prize_amount: i128`, `timestamp: u64`
- **BountyCancelled**: Bounty cancelled.
  - Payload: `bounty_id: u64`, `cancelled_by: Address`, `refund_amount: i128`, `timestamp: u64`

### Multisig

- **MultisigProposalCreated**: Multisig proposal created.
  - Payload: `proposal_id: u32`, `grant_id: u64`, `created_by: Address`, `threshold: u32`, `timestamp: u64`
- **MultisigSigned**: Multisig signature recorded.
  - Payload: `proposal_id: u32`, `signer: Address`, `approved: bool`, `total_weight_signed: u32`, `timestamp: u64`
- **MultisigExecuted**: Multisig proposal executed.
  - Payload: `proposal_id: u32`, `grant_id: u64`, `executed_by: Address`, `timestamp: u64`
- **MultisigProposalExpired**: Multisig proposal expired.
  - Payload: `proposal_id: u32`, `timestamp: u64`

### Compliance, invoices, and RBAC

- **ComplianceAttested**: Compliance attested.
  - Payload: `subject: Address`, `attested_by: Address`, `level: u32`, `expires_at: u64`, `timestamp: u64`
- **ComplianceRevoked**: Compliance attestation revoked.
  - Payload: `subject: Address`, `revoked_by: Address`, `timestamp: u64`
- **InvoiceSubmitted**: Invoice submitted.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `invoice_number: String`, `total: i128`, `timestamp: u64`
- **InvoiceApproved**: Invoice approved.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `approved_by: Address`, `timestamp: u64`
- **InvoiceRejected**: Invoice rejected.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `rejected_by: Address`, `reason: String`, `timestamp: u64`
- **InvoiceResubmitted**: Invoice resubmitted.
  - Payload: `grant_id: u64`, `milestone_idx: u32`, `total: i128`, `timestamp: u64`
- **RoleGranted**: RBAC role granted (`role` is the `Role` enum as `u32`).
  - Payload: `holder: Address`, `role: u32`, `granted_by: Address`, `timestamp: u64`
- **RoleRevoked**: RBAC role revoked.
  - Payload: `holder: Address`, `role: u32`, `revoked_by: Address`, `timestamp: u64`
- **RoleRenounced**: RBAC role renounced.
  - Payload: `holder: Address`, `role: u32`, `timestamp: u64`

### Crowdfund

- **CrowdfundCreated**: Crowdfund campaign created.
  - Payload: `campaign_id: u64`, `owner: Address`, `title: String`, `target_amount: i128`, `deadline: u64`, `timestamp: u64`
- **CrowdfundPledged**: Pledge received.
  - Payload: `campaign_id: u64`, `backer: Address`, `amount: i128`, `total_pledged: i128`, `timestamp: u64`
- **CrowdfundSucceeded**: Campaign succeeded.
  - Payload: `campaign_id: u64`, `total_pledged: i128`, `timestamp: u64`
- **CrowdfundFailed**: Campaign failed.
  - Payload: `campaign_id: u64`, `total_pledged: i128`, `timestamp: u64`
- **CrowdfundRefunded**: Backer refunded.
  - Payload: `campaign_id: u64`, `backer: Address`, `amount: i128`, `timestamp: u64`
- **CrowdfundCancelled**: Campaign cancelled.
  - Payload: `campaign_id: u64`, `cancelled_by: Address`, `total_pledged: i128`, `timestamp: u64`

### NFTs, collateral, and whitelist

- **NftMinted**: Milestone NFT minted.
  - Payload: `token_id: u32`, `grant_id: u64`, `milestone_idx: u32`, `owner: Address`, `timestamp: u64`
- **NftTransferred**: Milestone NFT transferred.
  - Payload: `token_id: u32`, `from: Address`, `to: Address`, `timestamp: u64`
- **CollateralDeposited**: Collateral deposited.
  - Payload: `grant_id: u64`, `contributor: Address`, `amount: i128`, `timestamp: u64`
- **CollateralReleased**: Collateral released.
  - Payload: `grant_id: u64`, `contributor: Address`, `amount: i128`, `timestamp: u64`
- **CollateralForfeited**: Collateral forfeited.
  - Payload: `grant_id: u64`, `contributor: Address`, `amount: i128`, `reason: String`, `timestamp: u64`
- **WhitelistAddressAdded**: Address added to a whitelist.
  - Payload: `address: Address`, `scope: WhitelistScope`, `timestamp: u64`
- **WhitelistAddressRemoved**: Address removed from a whitelist.
  - Payload: `address: Address`, `scope: WhitelistScope`, `timestamp: u64`

### Waitlist and templates

- **WaitlistJoined**: Applicant joined a grant waitlist.
  - Payload: `grant_id: u64`, `applicant: Address`, `position: u32`, `timestamp: u64`
- **WaitlistPromoted**: Applicant promoted from the waitlist.
  - Payload: `grant_id: u64`, `applicant: Address`, `position: u32`, `timestamp: u64`
- **WaitlistLeft**: Applicant left the waitlist.
  - Payload: `grant_id: u64`, `applicant: Address`, `timestamp: u64`
- **TemplateSaved**: Grant/milestone template saved.
  - Payload: `template_id: u64`, `owner: Address`, `name: String`, `timestamp: u64`
- **TemplateDeleted**: Template deleted.
  - Payload: `template_id: u64`, `owner: Address`, `timestamp: u64`
- **TemplateUsed**: Template used.
  - Payload: `template_id: u64`, `timestamp: u64`

### Streaming payments (`streaming.rs`)

- **StreamCreated**: Payment stream created.
  - Payload: `stream_id: u32`, `grant_id: u64`, `sender: Address`, `recipient: Address`, `rate_per_ledger: i128`, `deposited: i128`, `end_ledger: u32`
- **StreamWithdrawn**: Stream withdrawal.
  - Payload: `stream_id: u32`, `recipient: Address`, `amount: i128`
- **StreamCancelled**: Stream cancelled.
  - Payload: `stream_id: u32`, `sender_refund: i128`, `recipient_payout: i128`
- **StreamPaused**: Stream paused.
  - Payload: `stream_id: u32`, `paused_at_ledger: u32`
- **StreamResumed**: Stream resumed.
  - Payload: `stream_id: u32`, `new_end_ledger: u32`

### Circuit breaker (`circuit_breaker.rs`)

- **BreakerTripped**: Module circuit breaker tripped.
  - Payload: `module: ProtocolModule`, `tripped_by: Address`, `reason: String`
- **BreakerReset**: Circuit breaker reset by admin.
  - Payload: `module: ProtocolModule`, `reset_by: Address`
- **BreakerAutoReset**: Circuit breaker auto-reset.
  - Payload: `module: ProtocolModule`

### Hooks (`hooks.rs`)

- **HookTriggered**: Registered hook invoked (`event` is `HookEvent` as `u32`).
  - Payload: `event: u32`, `hook_index: u32`, `success: bool`
- **HookRegisteredEvent**: Hook registered.
  - Payload: `event: u32`, `hook_index: u32`, `target_contract: Address`

### Insurance (`insurance.rs`)

- **PolicyPurchased**: Insurance policy purchased.
  - Payload: `grant_id: u64`, `policyholder: Address`, `coverage_amount: i128`, `premium_paid: i128`
- **ClaimFiled**: Insurance claim filed.
  - Payload: `claim_id: u32`, `grant_id: u64`, `claimant: Address`, `claimed_amount: i128`
- **ClaimApproved**: Insurance claim approved.
  - Payload: `claim_id: u32`, `payout_amount: i128`
- **ClaimRejected**: Insurance claim rejected.
  - Payload: `claim_id: u32`

### Performance bonds (`performance_bond.rs`)

- **BondRequired**: Bond required on a grant.
  - Payload: `bond_id: u32`, `grant_id: u64`, `bond_amount: i128`
- **BondPosted**: Bond posted.
  - Payload: `bond_id: u32`, `grant_id: u64`, `guarantor: Address`
- **BondReleased**: Bond released.
  - Payload: `bond_id: u32`, `grant_id: u64`
- **BondClaimed**: Bond claimed.
  - Payload: `bond_id: u32`, `grant_id: u64`, `payout_amount: i128`

### Referrals (`referral.rs`)

- **ReferralCodeCreated**: Referral code created.
  - Payload: `referrer: Address`, `code_hash: Bytes`
- **ReferralApplied**: Referral code applied.
  - Payload: `referred: Address`, `referrer: Address`, `code_hash: Bytes`
- **ReferralRewardEarned**: Referral reward earned.
  - Payload: `referrer: Address`, `referred: Address`, `token: Address`, `amount: i128`
- **ReferralRewardsClaimed**: Referral rewards claimed.
  - Payload: `referrer: Address`, `token: Address`, `amount: i128`
- **ReferralCodeDeactivated**: Referral code deactivated.
  - Payload: `referrer: Address`, `code_hash: Bytes`

### Revenue share (`revenue_share.rs`)

- **EpochFinalized**: Revenue epoch finalized.
  - Payload: `epoch_id: u32`, `total_revenue: i128`, `total_stake_weight: i128`
- **RevenueClaimed**: Staker claimed epoch revenue.
  - Payload: `staker: Address`, `epoch_id: u32`, `amount: i128`

### Delegation (`delegate.rs`)

- **DelegationCreated**: Voting/review delegation created.
  - Payload: `delegator: Address`, `delegate: Address`, `created_at: u64`
- **DelegationRevoked**: Delegation revoked.
  - Payload: `delegator: Address`, `revoked_at: u64`

### Token swap (`token_swap.rs`)

- **SwapExecuted**: DEX swap executed.
  - Payload: `from_token: Address`, `to_token: Address`, `amount_in: i128`, `amount_out: i128`, `slippage_bps: u32`
- **SwapAndFundExecuted**: Swap-and-fund a grant.
  - Payload: `grant_id: u64`, `funder: Address`, `input_token: Address`, `input_amount: i128`, `swapped_amount: i128`
- **SwapAndPayExecuted**: Swap-and-pay a recipient.
  - Payload: `grant_id: u64`, `recipient: Address`, `grant_token: Address`, `preferred_token: Address`, `amount_out: i128`

## Legacy `env.events().publish` events

These are not `#[contractevent]` structs. Indexers should match the **topic
symbols** below.

### Grant pause (`grant_pause.rs`)

- **grant_paused**: Topics: `("grant_paused", grant_id)`; payload: `caller: Address`
- **grant_unpaused**: Topics: `("grant_unpaused", grant_id)`; payload: `caller: Address`

### Versioning / amendments (`versioning.rs`)

- **amendment_proposed**: Topics: `("amendment_proposed", grant_id)`; payload: `(owner: Address, amendment_version: u32)`
- **amendment_approved**: Topics: `("amendment_approved", grant_id)`; payload: `amendment_version: u32`
- **amendment_applied**: Topics: `("amendment_applied", grant_id)`; payload: `amendment_version: u32`

### Syndication (`syndication.rs`)

- **syndicate_formed**: Topics: `("syndicate_formed", grant_id)`; payload: `(lead: Address, target_total: i128)`
- **member_joined**: Topics: `("member_joined", grant_id)`; payload: `(member: Address, amount: i128, share_bps: u32)`
- **syndicate_closed**: Topics: `("syndicate_closed", grant_id)`; payload: `(lead: Address, deposited: i128)`
- **member_withdrew**: Topics: `("member_withdrew", grant_id)`; payload: `(member: Address, amount: i128)`

### Notifications (`notification.rs`)

- **notification**: Topics: `("notification", event: u32, scope_type, scope_data)`; payload: `payload: u128`

### Reviewer SLA (`reviewer_sla.rs`)

- **sla/reg**: Topics: `("sla", "reg", milestone_id)`; payload: `(reviewer: Address, deadline: u64)`
- **sla/breach**: Topics: `("sla", "breach", milestone_id)`; payload: `reviewer: Address`

## Indexing guidance

- Prefer matching typed events by **struct name** (e.g. `GrantCreated`), not by
  undocumented aliases.
- Use `grant_id` / `bounty_id` / `campaign_id` / `proposal_id` in the payload to
  shard streams. These IDs are not always topics.
- Re-check this file against `#[contractevent]` and `.publish(` in `src/` when
  adding new events.
- Source of truth for field types: the `pub struct` next to `#[contractevent]`
  (mostly `events.rs`, plus the module files named in each section).
