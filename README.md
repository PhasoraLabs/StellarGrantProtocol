# StellarGrants Protocol

Monorepo for **milestone-based grants on Stellar (Soroban)**: on-chain escrow, milestones, and voting, with a Next.js app, optional Express API, and a TypeScript client SDK.

## Contents

- [Repository layout](#repository-layout)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Quick start](#quick-start)
- [Configuration](#configuration)
- [Smart contracts](#smart-contracts)
- [CI](#ci)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)

## Repository layout

| Path | Description |
|------|-------------|
| [`stellargrant-contracts/`](stellargrant-contracts/) | Soroban smart contracts (Rust): build to WASM, tests, deploy via Stellar CLI. |
| [`stellargrant-fe/`](stellargrant-fe/) | Next.js frontend: reads contract state via Stellar RPC; wallets sign transactions in the browser. See [`stellargrant-fe/README.md`](stellargrant-fe/README.md) for app-specific docs. |
| [`client/`](client/) | `@stellargrants/client-sdk` — TypeScript SDK for Soroban contract interactions (`@stellar/stellar-sdk`). |
| [`api/`](api/) | Express + TypeScript API (PostgreSQL via TypeORM): optional middleware for caching and server-side flows. |

## Architecture

- **On-chain:** Soroban contract implements grants, milestones, escrow, approvals/voting, and events.
- **Frontend:** Primary data path is **direct to Stellar RPC** from the browser; no dedicated backend is required for core reads and signed writes. The Next.js app may include small **Route Handlers** (for example streaming or server-only concerns).
- **API:** The `api/` service is a separate process for endpoints that benefit from a database or server-side logic; use it when your deployment needs that layer.
- **SDK:** The `client/` package is for programmatic contract access from Node or bundlers, independent of the web UI.

## Prerequisites

**Smart contracts**

- Rust (stable; CI uses `dtolnay/rust-toolchain@stable`)
- `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools) for deploy/invoke

**Frontend, API, and client SDK**

- **Node.js 20+** recommended (Next.js 16 in `stellargrant-fe`)
- npm (lockfiles are committed under `stellargrant-fe/`, `api/`, and `client/`)

**API only**

- PostgreSQL reachable via `DATABASE_URL` when running the API locally or in production

## Quick start

Work in each package from the repository root as needed.

### Smart contracts — format, lint, and compile check

```bash
cd stellargrant-contracts
rustup target add wasm32-unknown-unknown

cargo fmt --all -- --check
cargo clippy --workspace --lib --target wasm32-unknown-unknown -- -D warnings
cargo check --workspace --target wasm32-unknown-unknown
```

These mirror the [CI workflow](.github/workflows/ci.yml).

### Smart contracts — tests

```bash
cd stellargrant-contracts
cargo test
```

Optional WASM build (when your workflow uses the contract Makefile):

```bash
cd stellargrant-contracts/contracts/stellar-grants
make build
make test
```

### Smart contracts — coverage (optional)

With [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) installed:

```bash
cd stellargrant-contracts
cargo tarpaulin --workspace --lib --target x86_64-unknown-linux-gnu --engine llvm --out Xml
```

Adjust flags to match your workspace layout if needed.

### Frontend

```bash
cd stellargrant-fe
npm ci
npm run dev
```

Dev server: [http://localhost:3000](http://localhost:3000) (default Next.js port).

### TypeScript client SDK

```bash
cd client
npm ci
npm run build
npm test
```

### API

```bash
cd api
npm ci
npm run dev
```

Default port **4000** (overridable with `PORT`). Ensure PostgreSQL is running and `DATABASE_URL` is set if you use persistence beyond defaults — see [`api/src/config/env.ts`](api/src/config/env.ts).

## Configuration

### Frontend environment variables

Create `stellargrant-fe/.env.local` (do not commit). Only variables used in code are listed here; see [`stellargrant-fe/lib/stellar/client.ts`](stellargrant-fe/lib/stellar/client.ts) and [`stellargrant-fe/lib/stellar/contract.ts`](stellargrant-fe/lib/stellar/contract.ts).

**Required for a non-default network or contract**

- `NEXT_PUBLIC_STELLAR_RPC_URL` — Soroban HTTP RPC endpoint
- `NEXT_PUBLIC_NETWORK_PASSPHRASE` — network passphrase for signing
- `NEXT_PUBLIC_CONTRACT_ID` — deployed contract address

**Optional**

- `NEXT_PUBLIC_NATIVE_TOKEN`, `NEXT_PUBLIC_USDC_TOKEN` — asset contract IDs when your UI uses them
- `NEXT_PUBLIC_IPFS_GATEWAY` — gateway base URL for milestone proofs
- `NEXT_PUBLIC_POSTHOG_KEY`, `NEXT_PUBLIC_POSTHOG_HOST` — analytics

Anything prefixed with `NEXT_PUBLIC_` is exposed to the browser. Do not put secrets in those variables.

### API environment variables

- `PORT` — listen port (default `4000`)
- `DATABASE_URL` — PostgreSQL connection string

## Smart contracts — deploy (example)

After building WASM (for example via `make build` under `stellargrant-contracts/contracts/stellar-grants`):

**Testnet**

```bash
cd stellargrant-contracts/contracts/stellar-grants
make build

stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_grants.wasm \
  --network testnet \
  --source-account YOUR_SECRET_KEY
```

**Mainnet**

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_grants.wasm \
  --network mainnet \
  --source-account YOUR_SECRET_KEY
```

Store keys outside the repo; follow the contract’s initialization steps after deploy.

## CI

GitHub Actions workflow: [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

**Current behavior**

- Runs on **pull requests** and on **pushes to `main`**.
- **Contracts** job: `cargo fmt` (check), `cargo clippy` (WASM, library targets, deny warnings), `cargo check` (WASM).

Frontend and API jobs are not in this workflow yet; run `npm run lint` and `npm run build` locally under `stellargrant-fe/` (and `npm run build` under `api/`) before opening a PR.

## Contributing

- Contracts: format with `cargo fmt`; keep `cargo clippy` clean. See [`stellargrant-contracts/ContributionGuide.md`](stellargrant-contracts/ContributionGuide.md).
- Frontend: see [`stellargrant-fe/CONTRIBUTING.md`](stellargrant-fe/CONTRIBUTING.md).

## Security

- Run tests and linters locally before deploying to public networks.
- Review access control and numeric safety in contract changes.
- Never commit private keys, seeds, or production secrets.

Report vulnerabilities via [GitHub Security Advisories](https://docs.github.com/code-security/security-advisories) or maintainer contact, as your project prefers.

## License

MIT License...
