# Contributing to StellarGrants Protocol 🌊

> **Drips Wave Program** — Fix issues, get merged, earn rewards at [drips.network/wave/stellar](https://drips.network/wave/stellar)

Welcome, and thank you for contributing to **StellarGrants Protocol** — a decentralized, milestone-based grant management system built with Rust and Soroban on the Stellar blockchain. This guide covers everything you need to go from zero to merged PR.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Fork & Clone](#fork--clone)
  - [Build & Test](#build--test)
- [How the Drips Wave Works](#how-the-drips-wave-works)
- [Finding an Issue](#finding-an-issue)
- [Claiming an Issue](#claiming-an-issue)
- [Branching Strategy](#branching-strategy)
- [Commit Message Format](#commit-message-format)
- [Pull Request Process](#pull-request-process)
- [Code Style & Linting](#code-style--linting)
- [Writing Tests](#writing-tests)
- [Contract-Specific Guidelines](#contract-specific-guidelines)
- [Documentation Guidelines](#documentation-guidelines)
- [Issue Labels Explained](#issue-labels-explained)
- [Getting Help](#getting-help)

---

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold a welcoming and respectful environment. Report violations to the maintainer via GitHub Issues or Discord.

---

## Getting Started

### Prerequisites

Make sure you have the following installed before contributing:

| Tool | Version | Purpose |
|------|---------|---------|
| [Rust](https://rustup.rs/) | `>= 1.78` | Smart contract language |
| [stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli) | Latest | Deploy & invoke contracts |
| `wasm32v1-none` target | — | Compile contracts to WASM |
| Node.js | `>= 18` | TypeScript client examples |
| Git | Any | Version control |

Install the WASM target:

```bash
rustup target add wasm32v1-none
```

Install the Stellar CLI:

```bash
cargo install --locked stellar-cli --features opt
```

### Fork & Clone

```bash
# 1. Fork the repo via GitHub UI, then:
git clone https://github.com/StellarGrant/StellarGrant-Contracts
cd StellarGrant-Contracts

# 2. Add upstream remote
git remote add upstream https://github.com/StellarGrant/StellarGrant-Contracts
```

### Build & Test

```bash
# Build the contract
make build

# Run all tests
make test

# Run linter
make lint

# Format code
make fmt

# Deploy to testnet
make deploy-testnet
```

Or using Cargo directly:

```bash
cargo build --target wasm32v1-none --release
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

---

## How the Drips Wave Works

The **Stellar Wave Program** on Drips is a monthly contribution sprint where:

1. **Maintainers** post scoped GitHub issues with clear acceptance criteria
2. **Contributors** claim issues, submit PRs, and earn **Wave Points** for merged work
3. **Points translate to real rewards** distributed at the end of each Wave cycle
4. A new Wave runs roughly every month — watch for announcements at [drips.network/wave/stellar](https://drips.network/wave/stellar)

> 💡 **Tip:** Subscribe to the [Drips Wave newsletter](https://drips.network/wave/newsletter) and join [their Discord](https://discord.gg/drips) to be notified when new Waves open.

**To be eligible for Wave rewards:**
- Your PR must be **merged** before the Wave closes
- The issue must carry the `drips-wave` label
- Your GitHub account must be registered on Drips

---

## Finding an Issue

Start here:

- **New to Soroban?** Filter by [`good first issue`](../../issues?q=is%3Aissue+label%3A%22good+first+issue%22) — these are well-scoped with step-by-step guidance
- **Know Rust?** Look for [`core`](../../issues?q=is%3Aissue+label%3Acore) or [`security`](../../issues?q=is%3Aissue+label%3Asecurity) labels
- **Love testing?** Filter by [`testing`](../../issues?q=is%3Aissue+label%3Atesting)
- **DevEx contributor?** Check [`documentation`](../../issues?q=is%3Aissue+label%3Adocumentation) and [`devex`](../../issues?q=is%3Aissue+label%3Adevex)

Browse the full issue list and sort by `Newest` or `Most commented` to find active discussions.

---

## Claiming an Issue

Before writing a single line of code:

1. **Check for existing assignees** — if someone is already assigned, don't duplicate work
2. **Comment on the issue** with something like:

   ```
   I'd like to work on this. My approach:
   - [brief plan of what you'll do]
   - ETA: [rough timeline]
   ```

3. **Wait for maintainer acknowledgment** — a maintainer will assign the issue to you (usually within 24–48h)
4. **Unassigned after 5 days?** If an assigned contributor goes silent, a maintainer may reassign — ping the issue thread first

> ⚠️ Do **not** open a PR for an unclaimed issue — it may be rejected if another contributor is already working on it.

---

## Branching Strategy

Always branch off `main`:

```bash
git checkout main
git pull upstream main
git checkout -b feature/issue-42-milestone-approve-vote
```

Branch naming convention:

| Type | Pattern | Example |
|------|---------|---------|
| New feature | `feature/issue-<N>-<short-name>` | `feature/issue-6-milestone-approve` |
| Bug fix | `fix/issue-<N>-<short-name>` | `fix/issue-33-overflow-check` |
| Documentation | `docs/issue-<N>-<short-name>` | `docs/issue-38-readme` |
| Tooling/CI | `chore/issue-<N>-<short-name>` | `chore/issue-49-github-actions` |
| Tests | `test/issue-<N>-<short-name>` | `test/issue-23-grant-create-tests` |

---

## Commit Message Format

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <short summary>

[optional body]

[optional footer: Closes #N]
```

**Types:**

| Type | When to use |
|------|-------------|
| `feat` | New contract function or feature |
| `fix` | Bug fix |
| `test` | Adding or updating tests |
| `docs` | Documentation only |
| `chore` | Tooling, CI, build changes |
| `refactor` | Code restructure without behavior change |
| `perf` | Performance improvement |
| `security` | Security fix or hardening |

**Examples:**

```bash
feat(contract): implement milestone_approve() with DAO vote logic

- Add reviewer vote storage with Map<Address, bool>
- Enforce quorum threshold before triggering payout
- Emit MilestoneApproved event on success

Closes #6
```

```bash
test(escrow): add fuzz tests for grant_fund() with random inputs

Closes #27
```

```bash
docs(readme): add quick start and deployment guide

Closes #38
```

> ✅ Keep the subject line under 72 characters. Use the body for *why*, not *what*.

---

## Pull Request Process

1. **Push your branch** to your fork:

   ```bash
   git push origin feature/issue-6-milestone-approve
   ```

2. **Open a PR** against `main` on the upstream repo

3. **Use this PR title format:** `feat(contract): implement milestone_approve() (#6)`

4. **Fill out the PR template** (auto-populated when you open a PR):

   ```markdown
   ## Summary
   Brief description of the changes.

   ## Related Issue
   Closes #6

   ## Changes Made
   - Added `milestone_approve()` function in `lib.rs`
   - Added vote storage accessors in `storage/helpers.rs`
   - Emits `MilestoneApproved` event

   ## Testing
   - [ ] Unit tests added/updated
   - [ ] `cargo test` passes locally
   - [ ] `cargo clippy` passes with no warnings
   - [ ] `cargo fmt` applied

   ## Notes for Reviewer
   Any context, tradeoffs, or open questions.
   ```

5. **CI must pass** — all checks (build, test, lint, fmt) must be green before review

6. **Request review** — tag `@maintainer` if no review after 48h

7. **Respond to feedback** — address comments with code or explanation; re-request review after updates

8. **Squash on merge** — maintainer will squash commits; your branch can have WIP commits

---

## Code Style & Linting

We enforce consistent style via CI. Run these before every push:

```bash
# Format
cargo fmt

# Lint (must be warning-free)
cargo clippy -- -D warnings
```

**Rust style rules:**
- Use `snake_case` for functions and variables
- Use `PascalCase` for types, structs, and enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefer explicit error types over `unwrap()` — use `Result<T, ContractError>`
- Add `/// rustdoc` comments to every public function
- Keep functions under 50 lines — extract helpers when needed
- No dead code (`#[allow(dead_code)]` requires maintainer approval)

**Soroban-specific rules:**
- Always call `env.storage()` through the `Storage` wrapper in `storage/helpers.rs`
- Use `require_auth()` at the top of every state-changing function
- Use `checked_add` / `checked_sub` for all balance math — never raw arithmetic
- Emit events via `env.events().publish()` for all state transitions
- Use the typed `DataKey` enum from `storage/keys.rs` for storage keys — never raw `Symbol`s in `lib.rs`

---

## Writing Tests

All PRs touching contract logic **must include tests**. We target ≥ 80% coverage.

**Test file location:** `contracts/stellar-grants/src/test.rs`

**Basic test structure:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_grant_create_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, StellarGrantsContract);
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let grant_id = client.grant_create(&owner, &String::from_str(&env, "My Grant"), &1000, &100, &3);

        assert_eq!(grant_id, 0);
    }
}
```

**Test coverage expectations by issue type:**

| Issue type | Minimum tests required |
|-----------|----------------------|
| New contract function | Happy path + at least 2 edge/error cases |
| Bug fix | Regression test that would have caught the bug |
| Security issue | Unauthorized caller test + boundary test |
| Refactor | Existing tests must still pass; no new tests required |

Run coverage locally:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

---

## Contract-Specific Guidelines

### Storage Keys

Storage is organized as a `storage/` module, not a single file:

- `storage/keys.rs` — the typed `#[contracttype]` key enums
- `storage/helpers.rs` — the `Storage` wrapper with typed accessors
- `storage/mod.rs` — re-exports the public storage surface

All storage keys must be defined as variants of the typed `DataKey` enum in
`storage/keys.rs` — never as raw `Symbol`s inlined in `lib.rs`. `DataKey` groups
related keys under per-domain sub-enums (`GrantKey`, `MilestoneKey`, `EscrowKey`,
`UserKey`, …), each annotated with `#[contracttype]` so Soroban derives a stable
XDR encoding:

```rust
// storage/keys.rs
#[contracttype]
#[derive(Clone)]
pub enum GrantKey {
    Data(u64),
    Counter,
    // ...
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Grant(GrantKey),
    Milestone(MilestoneKey),
    // ... protocol singletons, per-address keys, etc.
    Admin,
}
```

```rust
// ✅ Correct — typed key via the Storage wrapper / DataKey enum
env.storage().persistent().get(&DataKey::Grant(GrantKey::Data(grant_id)))

// ❌ Wrong — raw Symbol string in lib.rs
env.storage().persistent().get(&Symbol::new(env, "grant"))
```

### Auth Checks

Every function that mutates state must begin with an auth check:

```rust
pub fn milestone_approve(env: Env, grant_id: u64, milestone_idx: u32, reviewer: Address) {
    reviewer.require_auth(); // ← must be first
    // ...
}
```

### Events

Emit a structured event for every state change:

```rust
env.events().publish(
    (Symbol::new(&env, "MilestoneApproved"), grant_id),
    (milestone_idx, reviewer.clone()),
);
```

### Error Handling

Use the shared `ContractError` enum defined in `types.rs`:

```rust
pub enum ContractError {
    GrantNotFound = 1,
    Unauthorized = 2,
    MilestoneAlreadyApproved = 3,
    QuorumNotReached = 4,
    DeadlinePassed = 5,
    // ...
}
```

Return `Result<T, ContractError>` — never `panic!()` in production paths.

---

## Documentation Guidelines

- Every public function needs a `///` rustdoc comment with `# Arguments`, `# Returns`, and `# Errors` sections
- Update `ARCHITECTURE.md` if your PR changes the storage layout or auth model
- Update `README.md` if your PR adds or changes a CLI command or contract function signature
- New environment variables or config values must be documented in `.env.example`

**Rustdoc example:**

```rust
/// Approves a submitted milestone after reaching reviewer quorum.
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `grant_id` - The ID of the grant containing the milestone
/// * `milestone_idx` - Zero-based index of the milestone to approve
/// * `reviewer` - Address of the reviewer casting the vote
///
/// # Returns
/// `true` if the milestone is fully approved and payout was triggered, `false` if vote recorded but quorum not yet reached.
///
/// # Errors
/// * `ContractError::GrantNotFound` - if `grant_id` doesn't exist
/// * `ContractError::Unauthorized` - if `reviewer` is not in the reviewer list
/// * `ContractError::MilestoneAlreadyApproved` - if milestone was already approved
pub fn milestone_approve(env: Env, grant_id: u64, milestone_idx: u32, reviewer: Address) -> Result<bool, ContractError> {
```

---

## Issue Labels Explained

| Label | Meaning |
|-------|---------|
| `good first issue` | Well-scoped, beginner-friendly — ideal for your first Soroban contribution |
| `core` | Core contract logic: grant lifecycle, escrow, milestone state machine |
| `security` | Security-critical — auth, overflow, reentrancy, access control |
| `testing` | Unit, integration, or fuzz tests |
| `documentation` | README, rustdoc, guides, architecture docs |
| `tooling` | CI/CD, build scripts, CLI tooling |
| `performance` | WASM size, instruction count, fee optimization |
| `devex` | TypeScript bindings, example clients, CLI UX |
| `enhancement` | New feature or improvement to existing functionality |
| `bug` | Something broken or incorrect |
| `drips-wave` | Eligible for Drips Wave reward points |

---

## Getting Help

Stuck? Here's where to get support:

- **GitHub Discussions** — ask questions, propose ideas, share feedback
- **Issue comments** — ask clarifying questions directly on the issue you're working on
- **Drips Discord** — `#stellar-wave` channel at [discord.gg/drips](https://discord.gg/drips)
- **Stellar Developer Discord** — [discord.gg/stellardev](https://discord.gg/stellardev) for Soroban/SDK questions

> Please don't DM maintainers for support — use public channels so others can benefit from the answer too.

---

## Thank You 🙏

Every contribution — whether it's a one-line doc fix or a full DAO voting implementation — makes StellarGrants Protocol better. We're building open infrastructure for the Stellar ecosystem, one merged PR at a time.

**Fix. Merge. Earn. 🌊**