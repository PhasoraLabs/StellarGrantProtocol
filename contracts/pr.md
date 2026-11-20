## Summary

Implements four interconnected protocol improvements to the StellarGrant Soroban smart contract:

- **#513 — Voting & Quorum Logic Module**: Extracts milestone approval voting from `lib.rs` into a standalone `governance.rs` module with `cast_vote`, `quorum_reached`, `approval_percentage`, and `finalize_milestone`. `lib.rs::milestone_vote` now delegates entirely to `governance::cast_vote`. Includes unit tests for quorum edge cases (0 reviewers, single reviewer, exact 50%, 51%, all-reject path).

- **#520 — Global Contributor & Reviewer Registry**: Adds `registry.rs` with a protocol-wide contributor index and reviewer allowlist. Introduces `RegistryEntry` / `RegistryEntryType` types, `DataKey::ContributorIndex` / `DataKey::ReviewerAllowlist` storage keys, and new events `ReviewerApproved` / `ReviewerRevoked`. `lib.rs::contributor_register` now also writes to the global index. New public API: `get_contributors_page`, `contributor_count`, `is_approved_reviewer`, `approve_reviewer`, `revoke_reviewer`.

- **#525 — Protocol-Wide Compile-Time Constants**: Adds `constants.rs` with named constants for financial limits, governance parameters, timelock/ledger timing, string length caps, pagination, reputation, and milestone bounds. Magic numbers in `lib.rs` replaced with `constants::MAX_MILESTONES_PER_GRANT`, `constants::MAX_TITLE_LEN`, `constants::MAX_BIO_LEN`, and `constants::MAX_BATCH_SIZE`. Compile-time assertion tests verify key invariants.

- **#527 — Contract Upgrade & State Migration Framework**: Adds `migration.rs` providing `get_version`, `initialize_version`, `run_migration`, and `migration_history`. `ContractVersion` and `MigrationRecord` types added to `types.rs`; `DataKey::ContractVersion` and `DataKey::MigrationLog` added to `storage.rs`; `ContractMigrated` event added to `events.rs`. `lib.rs::initialize` now records the initial version (v1.0.0). `run_migration` is idempotent and dispatches to versioned migration functions (`migrate_v1_to_v2` placeholder). New public API: `get_contract_version`, `run_migration`, `migration_history`.

## Files changed

| File | Action |
|---|---|
| `src/constants.rs` | Created — compile-time constants (#525) |
| `src/governance.rs` | Created — voting/quorum module (#513) |
| `src/registry.rs` | Created — contributor & reviewer registry (#520) |
| `src/migration.rs` | Created — version migration framework (#527) |
| `src/types.rs` | Modified — added `MigrationRecord`, `ContractVersion`, `RegistryEntry`, `RegistryEntryType` |
| `src/storage.rs` | Modified — added 4 new `DataKey` variants and 8 new storage accessors |
| `src/events.rs` | Modified — added `ContractMigrated`, `ReviewerApproved`, `ReviewerRevoked` |
| `src/lib.rs` | Modified — wired all new modules, replaced magic numbers, exposed new entry points |
| `tests/migration_test.rs` | Created — 7 end-to-end migration tests |

## Test plan

- [x] All 22 existing unit tests continue to pass (`cargo test --lib`)
- [x] 7 new migration integration tests pass (`cargo test --test migration_test`)
- [x] 6 compile-time assertion tests in `constants::tests` pass
- [x] 9 quorum/percentage unit tests in `governance::tests` pass
- [x] Contract builds cleanly with no errors (`cargo build -p stellar-grants`)
- [ ] Verify `test_milestone_dispute`, `test_delegate_voting`, and `test_reputation_and_dispute_fee` suites (these reference contract methods not yet implemented and were failing before this PR)

## Breaking changes

- `initialize(Env)` → `initialize(Env, Address)` — deployer address now required; recorded as the initial version's deployer.
- `num_milestones` cap tightened from 100 → `MAX_MILESTONES_PER_GRANT` (20) per spec.
- Milestone batch size cap tightened from 20 → `MAX_BATCH_SIZE` (10) per spec.
- `contributor_register` no longer emits `contributor_registered` directly — the event is now emitted by `registry::register_contributor` to keep the registry self-contained.
