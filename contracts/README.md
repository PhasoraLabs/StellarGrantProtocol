# StellarGrants Protocol 🌊

> **Decentralized, milestone-based grant management on Stellar blockchain**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org/)
[![Soroban](https://img.shields.io/badge/soroban-25.0-green.svg)](https://soroban.stellar.org/)

**StellarGrants Protocol** is a decentralized, on-chain grant management system built using Soroban smart contracts (Rust) on the Stellar blockchain. It enables open-source projects, DAOs, and organizations to create milestone-based grants, manage contributor payouts transparently, and govern approvals through decentralized voting — all with sub-5-second finality and ultra-low fees thanks to Stellar.

## 🌟 Features

### Core Functionality
- **Milestone-Based Grants**: Create grants with multiple milestones, each requiring approval before payout
- **Token Escrow**: Secure token holding with automatic payout upon milestone approval
- **Decentralized Voting**: DAO-based governance for grant and milestone approvals
- **Multi-Token Support**: Support for XLM, USDC, and custom tokens
- **Time-Based Deadlines**: Optional milestone deadlines with expiry checks, expired-fund claims, and reviewer-approved extensions
- **Heartbeat Mechanism**: Automatic inactivity tracking (30-day inactive, 60-day cancellation trigger)
- **Machine-Readable Receipts**: Standardized `PayerReceipt` and `PayeeReceipt` events for automated accounting
- **Transparent Events**: All state changes emit events for off-chain indexing

### Security & Reliability
- **Reentrancy Protection**: Industry-standard security patterns
- **Overflow Protection**: Checked arithmetic for all operations
- **Access Control**: Role-based permissions for `Admin`, `GrantCreator`, `Reviewer`, and `Pauser`
- **Global Blacklist**: Administrative power to block malicious addresses from contract interaction
- **Heartbeat Enforcement**: Ensuring grant recipients maintain active communication with the protocol
- **Audit-Ready**: Comprehensive security best practices

### Developer Experience
- **TypeScript SDK**: Developer-friendly client library (planned)
- **Comprehensive Testing**: Unit, integration, and fuzz tests
- **CI/CD Pipeline**: Automated testing and deployment
- **Well-Documented**: Extensive documentation and examples

## 📋 Table of Contents

- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [Getting Started](#getting-started)
- [Building & Testing](#building--testing)
- [Deployment](#deployment)
- [Usage Examples](#usage-examples)
- [Contributing](#contributing)
- [Documentation](#documentation)
- [License](#license)

## 🏗️ Architecture

### Contract Modules

The StellarGrants contract is organized into modular components:

- **`lib.rs`**: Main contract implementation with public functions
- **`types.rs`**: Data structures, error types, and type definitions
- **`storage.rs`**: Storage key helpers and data persistence
- **`events.rs`**: Event definitions and emission helpers
- **`test.rs`**: Unit tests for contract functions

### Grant Lifecycle

```
1. Grant Creation
   └─> Owner creates grant with milestones
   
2. Funding
   └─> Funders deposit tokens into escrow
   
3. Milestone Submission
   └─> Recipient submits milestone with proof
   
4. Review & Voting
   └─> Community review opens first, then reviewers vote on milestone
   
5. Approval & Payout
   └─> If quorum is reached, the milestone enters a challenge window before payout
```

### Key Concepts

- **Grants**: A funding opportunity with defined milestones
- **Milestones**: Individual deliverables that unlock payments
- **Escrow**: Secure token holding until milestone approval
- **Quorum**: Minimum votes required for milestone approval
- **Reviewers**: Authorized addresses that can vote on milestones

## 📁 Project Structure

```
StellarGrant/
├── contracts/
│   └── stellar-grants/          # Core Soroban contract
│       ├── src/
│       │   ├── lib.rs           # Main contract implementation
│       │   ├── types.rs         # Data structures and errors
│       │   ├── events.rs        # Event definitions
│       │   ├── storage.rs       # Storage helpers
│       │   └── test.rs          # Unit tests
│       ├── Cargo.toml           # Contract dependencies
│       └── Makefile             # Build commands
├── tests/                        # Integration tests
├── client/                       # TypeScript SDK (planned)
├── scripts/                      # Deployment scripts
├── .github/workflows/            # CI/CD pipelines
├── issues/                       # Detailed issue descriptions
│   ├── issue1.md                # Grant creation
│   ├── issue2.md                # Milestone system
│   └── ...                      # More issues
├── Cargo.toml                   # Workspace configuration
├── ContributionGuide.md         # Contributing guidelines
└── README.md                    # This file
```

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

| Tool | Version | Purpose |
|------|---------|---------|
| [Rust](https://rustup.rs/) | `>= 1.78` | Smart contract language |
| [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli) | Latest | Deploy & invoke contracts |
| `wasm32v1-none` target | — | Compile Soroban contracts to WASM |
| Node.js | `>= 18` | For TypeScript SDK (optional) |
| Git | Any | Version control |

### Installation

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install WASM target**:
   ```bash
   rustup target add wasm32v1-none
   ```

3. **Install Stellar CLI**:
   ```bash
   cargo install --locked stellar-cli --features opt
   ```

4. **Clone the repository**:
   ```bash
   git clone https://github.com/StellarGrant/StellarGrant-Contracts.git
   cd StellarGrant-Contracts
   ```

## 🔨 Building & Testing

### Build the Contract

```bash
# From contracts/
stellar contract build --package stellar-grants --locked
```

The compiled WASM file will be in `target/wasm32v1-none/release/stellar_grants.wasm`.

### Optimized WASM Build

```bash
stellar contract build --package stellar-grants --locked --optimize
```

### WASM Size Benchmark

Measured on this branch with `stellar contract build` (see [`BENCHMARK.md`](./BENCHMARK.md) for methodology and caveats):

| Build | Size |
|------|------|
| Release (`--optimize=false`, with spec shaking) | `511,030` bytes |
| Optimized (`--optimize`) | `447,150` bytes |
| Optimizer delta | `63,880` bytes smaller (`12.5%`) |

`stellar contract build` enables Soroban SDK spec shaking (`SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2`). On the current code, `--optimize` still removes an additional `63,880` bytes (`12.5%`) from the release WASM.

The workspace release profile already uses size-focused settings:

```toml
[profile.release]
opt-level = "s"
overflow-checks = false
debug = 0
strip = true
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = "fat"
incremental = false
```

### Run Tests

```bash
# Run all tests
make test

# Or using Cargo
cargo test -p stellar-grants --locked

# Run with output
cargo test -- --nocapture
```

### Code Quality Checks

```bash
# Format code
make fmt
# or
cargo fmt --all

# Lint code (must be warning-free)
make lint
# or
cargo clippy -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## 🔐 RBAC Initialization

`initialize(admin, council)` now bootstraps the global `Admin` and `Pauser` roles for the `admin` address. After initialization, role management is done through:

- `grant_role(admin, account, role)`
- `revoke_role(admin, account, role)`
- `renounce_role(account, role)`
- `has_role(account, role)`
- `get_access_control(account)`

Recommended bootstrap flow:

```rust
client.initialize(&admin, &council);
client.grant_role(&admin, &creator, &stellar_grants::Role::GrantCreator);
client.grant_role(&admin, &reviewer, &stellar_grants::Role::Reviewer);
client.grant_role(&admin, &ops, &stellar_grants::Role::Pauser);
```

Core grant flows now honor these roles without changing the contract structure:

- `GrantCreator`: optional gate for grant creation
- `Reviewer`: optional gate for reviewer voting and extension approvals
- `Pauser`: global pause and unpause authority
- `Admin`: upgrade, treasury, council, and other protocol-level controls

## 🧪 Fuzz Testing

### Running Fuzzers

Fuzz testing is used to catch edge cases, arithmetic overflows, and unpredictable states in the core grant lifecycle. We use [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz).

#### Prerequisites
- Install cargo-fuzz:
  ```bash
  cargo install cargo-fuzz
  ```

#### How to Run Fuzzers

From the `contracts` directory:

```bash
cd fuzz
# Run grant lifecycle fuzz target
cargo fuzz run grant_lifecycle
# Run milestone submit fuzz target
cargo fuzz run milestone_submit
# Run milestone vote fuzz target
cargo fuzz run milestone_vote
```

Let each fuzzer run for at least 1 hour to ensure no panics or crashes are found.

#### Adding New Fuzz Targets
- Add a new file in `fuzz/fuzz_targets/` and register it in `fuzz/Cargo.toml` as a new `[[bin]]` entry.

#### Invariants Checked
- Total funds escrowed should always equal the sum of unapproved milestone amounts.
- A reviewer shouldn't be able to vote twice.
- State should not enter panic conditions under valid but extreme i128 values.

See also: [ContributionGuide.md](ContributionGuide.md) for more details.

## 🚢 Deployment

### Deploy to Testnet

```bash
# Build the contract first
cd contracts/stellar-grants
make build

# Deploy to testnet
stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_grants.wasm \
  --network testnet \
  --source-account YOUR_SECRET_KEY
```

### Deploy to Mainnet

⚠️ **Warning**: Only deploy to mainnet after thorough testing and security audit.

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_grants.wasm \
  --network mainnet \
  --source-account YOUR_SECRET_KEY
```

### Initialize the Contract

After deployment, initialize the contract:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --network testnet \
  --source-account YOUR_SECRET_KEY \
  -- \
  initialize \
  --admin GLOBAL_ADMIN_ADDRESS \
  --council COUNCIL_ADDRESS
```

The **global admin** is the contract-wide role used for council rotation, WASM upgrades, staking configuration, and other administrative entrypoints. The **council** resolves milestone disputes.

## 💡 Usage Examples

### Creating a Grant

```rust
use soroban_sdk::{Address, String, Env};

let env = Env::default();
let contract_id = env.register_contract(None, StellarGrantsContract);
let client = StellarGrantsContractClient::new(&env, &contract_id);

let owner = Address::generate(&env);
let grant_id = client.grant_create(
    &owner,
    &String::from_str(&env, "Open Source Project Grant"),
    &String::from_str(&env, "Funding for Q1 development milestones"),
    &10000i128,  // Total amount
    &2500i128,   // Per milestone
    &4u32,       // Number of milestones
)?;
```

### Funding a Grant

```rust
// Approve token transfer
token_client.approve(&funder, &contract_id, &10000i128, &1000u32);

// Fund the grant
client.grant_fund(&grant_id, &funder, &10000i128)?;
```

### Submitting a Milestone

```rust
client.milestone_submit(
    &grant_id,
    &0u32, // Milestone index
    &String::from_str(&env, "Completed feature X"),
    &String::from_str(&env, "https://github.com/..."), // Proof URL
)?;
```

### Voting on a Milestone

```rust
let approved = client.milestone_vote(
    &grant_id,
    &0u32,
    &reviewer,
    &true, // Approve
)?;

// If quorum reached, approved = true and payout triggered automatically
```

## 🤝 Contributing

We welcome contributions! StellarGrants Protocol is part of the **Drips Wave Program** — contribute and earn rewards.

### Quick Start for Contributors

1. **Read the [Contribution Guide](ContributionGuide.md)** for detailed instructions
2. **Browse [Issues](issues/README.md)** to find work that interests you
3. **Claim an issue** by commenting on it
4. **Follow the branching strategy**: `feature/issue-N-short-name`
5. **Submit a PR** with tests and documentation

### Issue Categories

- **Core Contract Logic**: Grant creation, milestone management
- **Token & Finance**: Escrow, payments, token integration
- **Governance & DAO**: Voting, quorum, reviewer management
- **Testing**: Unit, integration, fuzz tests
- **Security**: Audit preparation, security hardening
- **Events & Indexing**: Event system improvements
- **Tooling & CI/CD**: GitHub Actions, deployment scripts
- **Performance**: WASM size, gas optimization
- **SDK & Interface**: TypeScript client library
- **Advanced Features**: Deadlines, multi-token support

### Code Style

- Follow Rust style guidelines
- Use `snake_case` for functions, `PascalCase` for types
- Add rustdoc comments to all public functions
- Run `cargo fmt` and `cargo clippy` before committing
- Write tests for all new functionality

### Getting Help

- **GitHub Discussions**: Ask questions and propose ideas
- **Issue Comments**: Discuss specific issues
- **Drips Discord**: `#stellar-wave` channel
- **Stellar Developer Discord**: For Soroban/SDK questions

See [ContributionGuide.md](ContributionGuide.md) for complete guidelines.

## 📚 Documentation

### Project Documentation

- **[Contribution Guide](ContributionGuide.md)**: Complete guide for contributors
- **[Issues Directory](issues/README.md)**: Detailed issue descriptions
- **[Architecture Docs](ARCHITECTURE.md)**: System architecture (coming soon)

### External Resources

- [Soroban Documentation](https://developers.stellar.org/docs/build/smart-contracts/overview)
- [Stellar Developer Docs](https://developers.stellar.org/)
- [Soroban Examples](https://github.com/stellar/soroban-examples)
- [Drips Wave Program](https://drips.network/wave/stellar)

## 🔒 Security

Security is a top priority. Before deployment:

- ✅ All arithmetic uses checked operations
- ✅ Reentrancy protection implemented
- ✅ Access control enforced
- ✅ Comprehensive test coverage
- ✅ Security audit recommended

**Report security vulnerabilities** via GitHub Security Advisories or contact maintainers directly.

## 📊 Project Status

### ✅ Completed
- Project structure and scaffolding
- Basic contract framework
- Module organization (types, events, storage)
- Issue tracking system
- Heartbeat Mechanism implementation
- Blacklist System for security enforcement
- Machine-Readable Receipt System
- Comprehensive test suite (64 tests)

### 📋 Planned
- TypeScript SDK
- Frontend interface
- Multi-token support
- Advanced governance features

See [Issues](issues/README.md) for detailed roadmap.

## 🌊 Drips Wave Program

StellarGrants is part of the **Drips Wave Program**. Contributors can earn rewards for merged PRs:

- Fix issues labeled `drips-wave`
- Get your PR merged
- Earn Wave Points
- Receive rewards at cycle end

Learn more: [drips.network/wave/stellar](https://drips.network/wave/stellar)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built on [Soroban](https://soroban.stellar.org/) and [Stellar](https://www.stellar.org/)
- Part of the [Drips Wave Program](https://drips.network/wave/stellar)
- Inspired by transparent, decentralized grant management

## 🔗 Links

- **Repository**: [github.com/StellarGrant/StellarGrant-Contracts](https://github.com/StellarGrant/StellarGrant-Contracts)
- **Documentation**: [developers.stellar.org](https://developers.stellar.org/)
- **Drips Wave**: [drips.network/wave/stellar](https://drips.network/wave/stellar)
- **Stellar Discord**: [discord.gg/stellardev](https://discord.gg/stellardev)

## 📞 Contact

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For questions and discussions
- **Discord**: Join the Stellar developer community

---

**Built with ❤️ for the Stellar ecosystem**

*Fix. Merge. Earn. 🌊*
