# StellarGrants Protocol — Developer Setup Guide

This guide walks you through setting up a complete local development environment for the StellarGrants monorepo, covering all four packages. By the end you will have a running frontend connected to the Stellar testnet, an optional local API, and the ability to compile and test the Soroban contracts.

For architecture context read [ARCHITECTURE.md](ARCHITECTURE.md). For contribution workflow read [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Repository Setup](#repository-setup)
- [Frontend Setup](#frontend-setup)
- [Smart Contract Setup](#smart-contract-setup)
- [API Setup (Optional)](#api-setup-optional)
- [Client SDK Setup](#client-sdk-setup)
- [Running Everything Together](#running-everything-together)
- [Testing](#testing)
- [Storybook](#storybook)
- [Common Issues](#common-issues)
- [Useful Commands Reference](#useful-commands-reference)

---

## Prerequisites

### Node.js (Frontend, API, SDK)

Install Node.js 20 or later. Use [nvm](https://github.com/nvm-sh/nvm) to manage versions:

```bash
nvm install 20
nvm use 20
node --version   # v20.x.x
```

### Rust (Smart Contracts)

Install Rust via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Add the WASM compilation target
rustup target add wasm32-unknown-unknown

# Verify
rustc --version
cargo --version
```

### Stellar CLI (Contract Deploy/Invoke)

```bash
cargo install --locked stellar-cli --features opt

# Verify
stellar --version
```

See [Stellar CLI docs](https://developers.stellar.org/docs/tools/developer-tools/cli/install-stellar-cli) for alternative install methods.

### Freighter Wallet

Install the [Freighter browser extension](https://freighter.app) in Chrome or Firefox. You will need this to test wallet connect flows locally.

After installation:
1. Create a new wallet (or import an existing one)
2. Switch to **Testnet** in Freighter settings
3. Fund your testnet address using [Stellar Friendbot](https://friendbot.stellar.org)

### PostgreSQL (API only)

Only required if you plan to run the `backend/` package. Either install PostgreSQL locally or use Docker:

```bash
# Docker (easiest)
docker run -d \
  --name stellargrant-postgres \
  -e POSTGRES_USER=stellargrant \
  -e POSTGRES_PASSWORD=stellargrant \
  -e POSTGRES_DB=stellargrant \
  -p 5432:5432 \
  postgres:16-alpine
```

Or just run `docker compose up postgres` from the repository root.

---

## Repository Setup

```bash
# 1. Fork the repository on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/stellargrant-fe.git
cd stellargrant-fe

# 2. Add the upstream remote
git remote add upstream https://github.com/StellarGrant/stellargrant-fe.git

# 3. Verify remotes
git remote -v
# origin    https://github.com/YOUR_USERNAME/stellargrant-fe.git
# upstream  https://github.com/StellarGrant/stellargrant-fe.git
```

---

## Frontend Setup

The frontend lives in `web/` and is the package most contributors will work in.

### 1. Install Dependencies

```bash
cd web
npm ci
```

`npm ci` is preferred over `npm install` because it installs exact versions from `package-lock.json`.

### 2. Environment Variables

```bash
cp .env.local.example .env.local
```

Open `.env.local` and fill in the required values:

```env
# Required — Stellar testnet
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_STELLAR_RPC_URL=https://soroban-testnet.stellar.org
NEXT_PUBLIC_NETWORK_PASSPHRASE=Test SDF Network ; September 2015
NEXT_PUBLIC_HORIZON_URL=https://horizon-testnet.stellar.org

# Required — the deployed contract address
# Use a pre-deployed testnet contract or deploy your own (see Smart Contract Setup)
NEXT_PUBLIC_CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Testnet token addresses (provided defaults work with testnet)
NEXT_PUBLIC_NATIVE_TOKEN=CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
NEXT_PUBLIC_USDC_TOKEN=CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA

# Optional — Pinata (only needed if you're testing milestone proof uploads)
NEXT_PUBLIC_IPFS_GATEWAY=https://gateway.pinata.cloud/ipfs
PINATA_API_KEY=
PINATA_SECRET_KEY=

# Optional — local API backend
NEXT_PUBLIC_API_URL=http://localhost:4000
```

### 3. Start the Development Server

```bash
npm run dev
```

The app starts at [http://localhost:3000](http://localhost:3000) with Turbopack hot reload.

Alternatively, run with the mock API server simultaneously:

```bash
npm run dev:mock
# Starts mock API on :4000 and Next.js on :3000 concurrently
```

### 4. Verify the Setup

- Open [http://localhost:3000](http://localhost:3000)
- The landing page should load without console errors
- Navigate to `/grants` — grant cards should appear (testnet data)
- Click **Connect Wallet** and connect your Freighter wallet
- Run the checks:

```bash
npm run lint         # ESLint — should produce no errors
npm run test:run     # Vitest — all tests should pass
npm run build        # Production build — should complete without errors
```

---

## Smart Contract Setup

The contracts live in `contracts/` and are written in Rust targeting `wasm32-unknown-unknown`.

### 1. Check and Lint

```bash
cd contracts
cargo fmt --all -- --check
cargo clippy --workspace --lib --target wasm32-unknown-unknown -- -D warnings
cargo check --workspace --target wasm32-unknown-unknown
```

These three commands mirror CI exactly.

### 2. Run Contract Tests

```bash
cargo test
```

For a specific contract:

```bash
cargo test -p stellar-grants
```

### 3. Build the WASM Binary

```bash
cd contracts/stellar-grants
make build
# Output: target/wasm32v1-none/release/stellar_grants.wasm
```

### 4. Deploy to Testnet

Generate and fund test accounts:

```bash
stellar keys generate alice --network testnet
stellar keys fund alice --network testnet
```

Deploy:

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_grants.wasm \
  --network testnet \
  --source-account alice \
  --alias stellar_grants
```

The CLI prints the deployed contract address. Copy it into `NEXT_PUBLIC_CONTRACT_ID` in your `.env.local`.

Initialize the contract:

```bash
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account alice \
  -- initialize
```

### 5. Invoke Contract Functions (CLI Testing)

```bash
# Create a grant
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account alice \
  -- grant_create \
  --owner $(stellar keys address alice) \
  --title "Test Grant" \
  --description "A test grant for local development" \
  --total_amount 1000 \
  --per_milestone 500 \
  --milestones 2

# Fund the grant (grant_id returned from above)
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account alice \
  -- grant_fund \
  --grant_id 1 \
  --funder $(stellar keys address alice) \
  --amount 1000
```

For a complete walkthrough see [TUTORIAL.md](TUTORIAL.md).

---

## API Setup (Optional)

The Express API lives in `backend/`. It is not required for frontend development but provides faster queries and SSE relay.

### 1. Install Dependencies

```bash
cd backend
npm ci
```

### 2. Configure Environment

```bash
cp .env.example .env
```

```env
PORT=4000
DATABASE_URL=postgres://stellargrant:stellargrant@localhost:5432/stellargrant
```

### 3. Sync Database Schema

```bash
npm run typeorm:sync
```

Or run migrations:

```bash
npm run migration:run
```

### 4. Start the API

```bash
npm run dev
```

The API starts at [http://localhost:4000](http://localhost:4000). Verify: `curl http://localhost:4000/health`

### Docker Alternative

```bash
# From the repository root
docker compose up
# Starts Postgres on :5432 and the API on :4000
```

---

## Client SDK Setup

The TypeScript SDK lives in `client/`.

```bash
cd client
npm ci
npm run build     # compiles TypeScript to dist/
npm test          # runs Jest tests
```

To use the SDK in a script:

```typescript
import { StellarGrantsClient } from "./dist/index.js";

const client = new StellarGrantsClient({
  rpcUrl: "https://soroban-testnet.stellar.org",
  contractId: process.env.CONTRACT_ID!,
  networkPassphrase: "Test SDF Network ; September 2015",
});

const grants = await client.listGrants();
console.log(grants);
```

To generate SDK documentation:

```bash
npm run docs
```

---

## Running Everything Together

For full local development with all services:

**Terminal 1 — Postgres + API:**
```bash
docker compose up          # or: cd backend && npm run dev (if Postgres already running)
```

**Terminal 2 — Frontend:**
```bash
cd web
npm run dev
```

Or as a single command using the mock server (no Postgres needed):

```bash
cd web
npm run dev:mock
```

---

## Testing

### Frontend Unit Tests (Vitest)

```bash
cd web
npm test              # watch mode — re-runs on file change
npm run test:run      # single pass with coverage report
```

Test files live in `tests/` and alongside source as `*.test.tsx`.

### Frontend E2E Tests (Playwright)

```bash
cd web

# Install Playwright browsers (first time only)
npx playwright install

npm run test:e2e              # headless
npm run test:e2e:headed       # opens browser window — useful for debugging
```

E2E tests require a running dev server. The Playwright config starts one automatically via `webServer`.

### Contract Tests (Rust)

```bash
cd contracts
cargo test
```

### API Tests

```bash
cd backend
npm run test:e2e          # E2E tests (requires running API + Postgres)
npm run test:integration  # integration tests
```

### Coverage

Frontend coverage:

```bash
cd web
npm run test:run -- --coverage
# Coverage report in coverage/
```

Contract coverage (requires cargo-tarpaulin):

```bash
cargo install cargo-tarpaulin
cd contracts
cargo tarpaulin --workspace --lib --target x86_64-unknown-linux-gnu --engine llvm --out Html
```

---

## Storybook

Browse and develop UI components in isolation:

```bash
cd web
npm run storybook
```

Opens at [http://localhost:6006](http://localhost:6006).

Build a static Storybook for deployment:

```bash
npm run build-storybook
```

---

## Common Issues

### `NEXT_PUBLIC_CONTRACT_ID` not set

The app boots without a contract ID but all RPC calls will fail. Set `NEXT_PUBLIC_CONTRACT_ID` in `.env.local` to a deployed testnet contract address.

### Freighter not detecting testnet

In Freighter settings, ensure the network is set to **Testnet**. The app checks `NEXT_PUBLIC_STELLAR_NETWORK` against the wallet's reported network and shows a mismatch warning.

### `cargo clippy` fails with WASM target errors

Ensure the WASM target is installed:

```bash
rustup target add wasm32-unknown-unknown
```

### `npm ci` fails with peer dependency errors

The project pins to specific versions. Do not use `npm install` — always use `npm ci` to install from the lockfile.

### Playwright tests time out

Playwright starts the Next.js dev server before running tests. If port 3000 is already in use, tests fail. Stop any running dev server first.

### Mock API returns stale data

The mock server (`mock-server/`) serves static fixture data. If you've changed the type definitions, update the fixtures in `mock-server/data/`.

### `DATABASE_URL` connection refused

Ensure PostgreSQL is running. If using Docker: `docker compose up postgres`. Check the connection string in `backend/.env`.

---

## Useful Commands Reference

```bash
# ── Frontend ─────────────────────────────────────────────────────────
cd web

npm run dev                # dev server with Turbopack hot reload
npm run dev:mock           # dev server + mock API concurrently
npm run build              # production build
npm start                  # serve production build
npm run lint               # ESLint
npm test                   # Vitest watch
npm run test:run           # Vitest single pass + coverage
npm run test:e2e           # Playwright E2E (headless)
npm run test:e2e:headed    # Playwright E2E (with browser)
npm run storybook          # Storybook dev server
npm run build-storybook    # build static Storybook

# ── Contracts ────────────────────────────────────────────────────────
cd contracts

cargo fmt --all                               # format all Rust code
cargo fmt --all -- --check                   # check without writing
cargo clippy --workspace --lib \
  --target wasm32-unknown-unknown \
  -- -D warnings                             # lint
cargo check --workspace \
  --target wasm32-unknown-unknown            # type check
cargo test                                   # run all tests
cd contracts/stellar-grants && make build    # build WASM

# ── Stellar CLI ──────────────────────────────────────────────────────

stellar keys generate <name> --network testnet
stellar keys fund <name> --network testnet
stellar keys address <name>

stellar contract deploy \
  --wasm <path.wasm> \
  --network testnet \
  --source-account <name> \
  --alias <alias>

stellar contract invoke \
  --id <alias-or-address> \
  --network testnet \
  --source-account <name> \
  -- <function_name> [--arg value ...]

# ── API ──────────────────────────────────────────────────────────────
cd backend

npm run dev                # dev server with tsx watch
npm run build              # compile TypeScript
npm start                  # start compiled server
npm run typeorm:sync       # sync DB schema
npm run migration:run      # run migrations
npm run test:e2e           # E2E tests
npm run test:integration   # integration tests

# ── Client SDK ───────────────────────────────────────────────────────
cd client

npm run build              # compile TypeScript
npm test                   # run Jest tests
npm run docs               # generate TypeDoc

# ── Docker ───────────────────────────────────────────────────────────
# From repository root:
docker compose up               # Postgres + API
docker compose up postgres      # Postgres only
docker compose down             # stop all services
docker compose down -v          # stop + delete volumes (wipes DB)
```
