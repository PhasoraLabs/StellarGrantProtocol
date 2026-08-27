
# Stellar Grants Event Schema

This contract emits typed Soroban `#[contractevent]` events with a consistent schema for off-chain indexers and subgraphs.

## Event Structure

**Topics:**
- `[contract_id, EventName, grant_id, ...]` (for grant/milestone events)
- `[contract_id, EventName]` (for contract-level events)

**Payload:**
- All business fields (e.g., funder, amount, reviewer, feedback, etc.)
- Standard metadata: `event_version`, `grant_id`, `timestamp`

## Event Types

### Contract Lifecycle Events
- **ContractInitialized**: Emitted when `initialize` completes (global admin and council stored, storage version set to `1`).
  - Topics: `[contract_id, "ContractInitialized", 0]`
  - Payload: `event_version`, `grant_id`, `council`, `timestamp`
- **ContractUpgraded**: Emitted when contract config changes without swapping WASM (e.g. `admin_change`, `set_council`).
  - Topics: `[contract_id, "ContractUpgraded", 0]`
  - Payload: `event_version`, `grant_id`, `actor`, `component`, `timestamp`
  - Current `component` values include `"admin_changed"` and `"council_updated"`.
- **ContractWasmUpgraded**: Emitted immediately before `env.deployer().update_current_contract_wasm` in `admin_upgrade` (after storage version is incremented).
  - Topics: `[contract_id, "ContractWasmUpgraded", 0]`
  - Payload: `event_version`, `grant_id`, `admin`, `new_wasm_hash` (32 bytes), `new_storage_version`, `timestamp`

### Grant Events
- **GrantCreated**: Grant created.
  - Topics: `[contract_id, "GrantCreated", grant_id]`
  - Payload: `event_version`, `grant_id`, `owner`, `title`, `total_amount`, `timestamp`
- **GrantFunded**: Grant funded.
  - Topics: `[contract_id, "GrantFunded", grant_id]`
  - Payload: `event_version`, `grant_id`, `funder`, `amount`, `new_balance`, `timestamp`
- **PayerReceipt**: Receipt for funder contribution.
  - Topics: `[contract_id, "PayerReceipt", grant_id]`
  - Payload: `event_version`, `grant_id`, `funder`, `token`, `amount`, `memo`, `timestamp`
- **GrantMetadataUpdated**: Grant metadata updated.
  - Topics: `[contract_id, "GrantMetadataUpdated", grant_id]`
  - Payload: `event_version`, `grant_id`, `owner`, `title`, `description`, `timestamp`
- **GrantCancelled**: Grant cancelled.
  - Topics: `[contract_id, "GrantCancelled", grant_id]`
  - Payload: `event_version`, `grant_id`, `owner`, `reason`, `refund_amount`, `timestamp`
- **GrantCompleted**: Grant completed.
  - Topics: `[contract_id, "GrantCompleted", grant_id]`
  - Payload: `event_version`, `grant_id`, `total_paid`, `remaining_balance`, `timestamp`

### Milestone Events
- **MilestoneSubmitted**: Milestone submitted.
  - Topics: `[contract_id, "MilestoneSubmitted", grant_id, milestone_idx]`
  - Payload: `event_version`, `grant_id`, `milestone_idx`, `description`, `timestamp`
- **MilestoneVoted**: Reviewer voted on milestone.
  - Topics: `[contract_id, "MilestoneVoted", grant_id, milestone_idx]`
  - Payload: `event_version`, `grant_id`, `milestone_idx`, `reviewer`, `approve`, `feedback`, `timestamp`
- **MilestoneRejected**: Milestone rejected.
  - Topics: `[contract_id, "MilestoneRejected", grant_id, milestone_idx]`
  - Payload: `event_version`, `grant_id`, `milestone_idx`, `reviewer`, `reason`, `timestamp`
- **MilestoneStatusChanged**: Milestone state changed.
  - Topics: `[contract_id, "MilestoneStatusChanged", grant_id, milestone_idx]`
  - Payload: `event_version`, `grant_id`, `milestone_idx`, `new_state`, `timestamp`
- **MilestonePaid**: Milestone payout executed.
  - Topics: `[contract_id, "MilestonePaid", grant_id, milestone_idx]`
  - Payload: `event_version`, `grant_id`, `milestone_idx`, `amount`, `timestamp`
- **PayeeReceipt**: Receipt for payout to recipient.
  - Topics: `[contract_id, "PayeeReceipt", grant_id]`
  - Payload: `event_version`, `grant_id`, `recipient`, `token`, `amount`, `milestone_idx`, `timestamp`
- **QuorumReached**: Milestone voting quorum reached.
  - Topics: `[contract_id, "QuorumReached", grant_id, milestone_idx]`
  - Payload: `event_version`, `grant_id`, `milestone_idx`, `approvals`, `quorum`, `timestamp`

### Refund and Contributor Events
- **RefundIssued**: Refund issued to funder.
  - Topics: `[contract_id, "RefundIssued", grant_id]`
  - Payload: `event_version`, `grant_id`, `funder`, `amount`, `timestamp`
- **ContributorRegistered**: Contributor registered.
  - Topics: `[contract_id, "ContributorRegistered", 0]`
  - Payload: `event_version`, `grant_id`, `contributor`, `name`, `timestamp`
- **ReputationUpdated**: Contributor reputation updated.
  - Topics: `[contract_id, "ReputationUpdated", grant_id]`
  - Payload: `event_version`, `grant_id`, `milestone_idx`, `contributor`, `new_reputation_score`, `total_earned`, `timestamp`

## Indexing Guidance

- Filter by event type and `grant_id` in topics.
- Use `event_version` for schema migration support.
- All state-modifying functions emit a unique, well-documented event.
- See contract code for full event struct definitions.
