# StellarGrants Protocol

<div align="center">

**Milestone-based grant management on the Stellar blockchain — on-chain escrow, DAO voting, and contributor reputation, all in one open-source monorepo.**

[![CI](https://github.com/StellarGrant/stellargrant-fe/actions/workflows/ci.yml/badge.svg)](https://github.com/StellarGrant/stellargrant-fe/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Next.js](https://img.shields.io/badge/Next.js-16-black)](https://nextjs.org/)
[![Stellar SDK](https://img.shields.io/badge/Stellar%20SDK-13.x-7D00FF)](https://stellar.github.io/js-stellar-sdk/)
[![Contributors](https://img.shields.io/github/contributors/StellarGrant/stellargrant-fe)](https://github.com/StellarGrant/stellargrant-fe/graphs/contributors)


[Overview](#overview) • [Features](#features) • [Architecture](#architecture) • [Quick Start](#quick-start) • [Packages](#packages) • [Contributing](#contributing) • [Security](#security)

</div>

---

## Overview

StellarGrants Protocol is a fully decentralized grant-management system built on [Stellar (Soroban)](https://developers.stellar.org/docs/build/smart-contracts/overview). It allows grant creators to post milestone-gated bounties, contributors to submit work with on-chain proof, and a decentralized reviewer committee to vote on approvals — with automatic token payouts from on-chain escrow the moment consensus is reached.

All protocol state lives on the Soroban smart contract. The Next.js frontend reads contract state directly from Stellar RPC — no centralized backend is required for any core feature. The contract itself is large and modular (90+ Rust modules covering everything from quadratic funding to multisig escrow — see [Smart Contract Modules](#smart-contract-modules)), and an optional Express API adds indexing, notifications, and OAuth-based accounts on top.

### Who is it for?

| Role | What they do |
|------|--------------|
| **Grant Creator** | Post grants with budget, milestones, reviewer list, and token |
| **Contributor** | Browse open grants, submit milestone work with IPFS proof |
| **Reviewer** | Vote approve / reject on milestone submissions |
| **Funder** | Deposit XLM or any SEP-41 token (e.g. USDC) into a grant's on-chain escrow |

---

## Features

### Core Grant Lifecycle

- **Milestone-Based Escrow** — Funds are locked in the Soroban contract and released only when a milestone is approved by the reviewer quorum
- **Standard & High-Security Escrow** — `grant_create_high_security` gates payout release behind an on-chain multisig signer set in addition to reviewer voting
- **Automatic Payout** — No admin intervention: the contract executes the token transfer atomically as soon as the vote threshold is reached
- **Grant Renewal, Transfer & Forking** — Owners can propose renewals, transfer roles (e.g. replace a reviewer), or fork an existing grant record
- **Grant Pause & Timers** — Grants can be paused independently of the global circuit breaker, and deadline timers can auto-trigger default actions
- **Milestone Dependencies & Templates** — Milestones can be sequenced as a DAG (later ones unlock only once earlier ones are approved) and created from reusable templates
- **Batch Operations** — Submit multiple milestones, fund multiple grants, or add/remove a reviewer across many grants in a single transaction
- **Multi-Token Support** — Grants can be denominated in native XLM or any SEP-41 token (e.g. USDC)

### Governance, Voting & Delegation

- **DAO Voting** — Every milestone requires a configurable quorum of reviewer approvals before payout is triggered
- **Vote Delegation** — Reviewers can delegate their voting power to another address for a grant, which resolves back to the delegator on vote
- **Quadratic Voting** — Reviewers can be allocated voice credits and cast weighted quadratic votes on milestones
- **DAO Proposals** — Protocol-level changes (e.g. admin rotation) can be routed through a passed-and-executed DAO proposal instead of direct admin action
- **Dispute Resolution & Arbitration** — Contributors or funders can raise disputes on rejected milestones; a staked arbitration pool of arbiters votes on outcomes
- **Public/Open Review** — Non-reviewer community members can leave public review signals and helpful-vote feedback on submissions

### Funding Mechanisms

- **Quadratic Funding Matching Rounds** — Admins can create QF rounds with a matching pool; contributions are matched proportionally to broad community support
- **Crowdfunding Campaigns** — Grants can be crowdfunded with pledge tracking and refunds if a campaign doesn't reach its goal
- **Bounty Grants** — Simpler bounty-style grants for smaller, single-submission tasks
- **Syndication** — Multiple funders can co-lead a grant as a syndicate
- **Referral Program** — Referral codes and on-chain rewards for bringing in new contributors/funders
- **Waitlists & Whitelists** — Grants can gate contributor/funder participation behind a waitlist or an allow-list

### Escrow, Payments & Treasury

- **Streaming Payments** — Continuous per-ledger payment streams (create/withdraw/pause/resume/cancel) as an alternative to lump-sum milestone payouts
- **Split Payments** — A milestone payout can be split across multiple recipients
- **Protocol Fees & Treasury** — Configurable protocol fee is deducted and split across the reviewer reward pool, revenue-share pool, and treasury on every payout
- **Performance Bonds & Collateral** — Contributors can be required to post a bond or collateral before submitting milestones, forfeited on abandonment
- **Clawback & Lockup** — Disputed funds can be clawed back by an authorized arbiter role; tokens can be locked up/vested over time
- **Token Swaps** — Integration point for swapping tokens (e.g. DEX routing) as part of payout flows

### Reputation, Recognition & Verification

- **Contributor Reputation** — On-chain reputation scoring tracks completed/rejected milestones, with configurable decay over time
- **Soulbound Milestone NFTs** — Approved milestones mint a non-transferable NFT certificate for the contributor
- **Badges** — Automatic badge awards (e.g. first milestone, ten milestones, fifty milestones)
- **Reviewer Staking, Rewards & SLA** — Reviewers stake tokens to participate in a grant's quorum, earn reward-pool payouts, can be slashed for misbehavior, and are tracked against SLA response times
- **KYC / Compliance Gating** — Grants can require a minimum verification level (via an identity oracle) before high-value payouts release
- **Scoring Rubrics & Checklists** — Structured, weighted scoring dimensions and required-criteria checklists gate milestone approval

### Protocol Operations & Safety

- **Emergency Pause & Circuit Breakers** — Global admin pause plus per-module circuit breakers (e.g. pause only the Streaming module)
- **Rate Limiting** — Per-address, per-action rate limits (grant creation, milestone submission, registration, …)
- **Reentrancy Guards** — Critical state-changing flows are wrapped in a non-reentrant context
- **Immutable Audit Log** — Every state-changing action on a grant is appended to an on-chain audit log
- **Versioned Migrations** — Contract storage schema is versioned, with an admin-only migration path and full migration history
- **Role-Based Access Control** — SuperAdmin/ProtocolAdmin roles gate sensitive operations beyond the single global-admin model
- **Provenance & Data Export** — Append-only contribution provenance ledger, paginated data export, funder financial reports, analytics snapshots, and protocol-wide metrics
- **Cross-Contract & Cross-Chain Hooks** — External contract hooks on protocol events, an oracle interface for price feeds, and a grant-bridge/relayer pattern for cross-chain proofs

### Frontend Application

- **Wallet-First UX** — Connect Freighter or Albedo today; xBull and Stellar Passkeys (WebAuthn/Secp256r1) are wired in the UI as "coming soon"
- **Zero-Backend Core** — All state reads can go directly to Stellar RPC; the optional API is additive, not required
- **Real-Time Event Streaming** — Subscribe to contract events via Server-Sent Events (relayed by the optional API) or direct RPC polling for live vote counts and funding progress
- **Multi-Step Grant Creation** — Guided form with Zod validation, milestone builder, and budget configurator
- **IPFS Proof Submission** — Contributors upload milestone evidence to IPFS via Pinata; the CID is stored on-chain
- **Leaderboard, Dashboard & Profiles** — Contributor reputation board, a per-wallet dashboard, and public contributor profile pages with GitHub handle and skills
- **Responsive & Accessible** — Mobile-first Tailwind UI with ARIA labels, keyboard shortcuts, and dark theme
- **Storybook Component Library** — All UI components are documented and previewed in Storybook

### Optional Backend API

- **Indexing & Caching** — Indexes on-chain events into PostgreSQL for fast, filterable, paginated queries the RPC alone can't do cheaply
- **OAuth Accounts** — GitHub and Twitter/X OAuth login (via Passport), linked to a Stellar address
- **RBAC** — A separate role/permission system (`roles`, `user_roles`) for API-level authorization
- **Notifications & Webhooks** — Queued email notifications (SendGrid), outbound webhook subscriptions with delivery logs, and Socket.IO for live push
- **Admin & Ops Tooling** — Admin routes, protocol analytics, rate-limit alerting (Redis-backed, falls back to in-memory), Prometheus metrics, and Swagger API docs
- **Disputes, Appeals & Moderation** — Milestone appeals, dispute tracking, comment threads, and community/report entities layered on top of the on-chain state
- **Reconciliation** — A reconciliation service cross-checks indexed data against on-chain state via checkpoints

---

## Repository Layout

```
StellarGrantProtocol/              ← Monorepo root
├── web/                           ← Next.js 16 frontend (primary package)
├── contracts/                     ← Soroban smart contracts (Rust → WASM)
├── client/                       ← @stellargrants/client-sdk (TypeScript SDK)
├── backend/                      ← Optional Express + TypeORM caching API
├── docker-compose.yml            ← Postgres + API service
├── .github/workflows/ci.yml      ← GitHub Actions CI
├── TUTORIAL.md                   ← Beginner end-to-end walkthrough
├── CONTRIBUTING.md               ← Root contribution guide (this repo)
├── ARCHITECTURE.md               ← Deep-dive architecture reference
├── DEVELOPMENT.md                ← Full developer environment setup
├── CODE_OF_CONDUCT.md            ← Community standards
└── SECURITY.md                   ← Vulnerability reporting policy
```

| Package | Tech | Purpose |
|---------|------|---------|
| [`web/`](web/) | Next.js 16, React 19, TypeScript | Full-featured web UI — grant browsing, creation, funding, milestone voting |
| [`contracts/`](contracts/) | Rust, Soroban SDK | ~90-module smart contract: escrow, governance, funding, reputation, protocol safety |
| [`client/`](client/) | TypeScript, stellar-sdk | `@stellargrants/client-sdk` — typed SDK + CLI + Vue composables for Node, bundlers, or scripts |
| [`backend/`](backend/) | Express, TypeORM, PostgreSQL | Optional layer: indexing, OAuth accounts, notifications/webhooks, RBAC, admin & ops tooling |

---

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Browser (User)                           │
│                                                                 │
│  ┌──────────────────┐    ┌─────────────────┐                   │
│  │  Next.js Frontend│    │ Wallet Extension │                   │
│  │  (React / SSR)   │    │(Freighter/Albedo)│                   │
│  └────────┬─────────┘    └────────┬────────┘                   │
│           │ reads contract state   │ signs transactions          │
└───────────┼────────────────────────┼────────────────────────────┘
            │                        │
            ▼                        ▼
┌───────────────────────────────────────────────────────────────┐
│                      Stellar Network                           │
│                                                               │
│  ┌─────────────────────┐    ┌───────────────────────────────┐ │
│  │  Soroban RPC Node   │    │  Horizon API                  │ │
│  │  (simulateTx /      │    │  (account info / balances /   │ │
│  │   sendTx / events)  │    │   trustlines)                 │ │
│  └──────────┬──────────┘    └───────────────────────────────┘ │
│             │                                                  │
│             ▼                                                  │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │            StellarGrants Soroban Contract                │ │
│  │  grant_create · grant_fund · milestone_submit           │ │
│  │  milestone_vote · milestone_payout · dispute_raise      │ │
│  └──────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
            │
            ▼ (optional)
┌─────────────────────────┐
│  Express API (backend/) │  ← caching, indexing, SSE relay
│  PostgreSQL             │
└─────────────────────────┘
```

### Key Design Decisions

| Decision | Why |
|----------|-----|
| Zero-backend for core flows | All grant data is on-chain; no server can become a single point of failure or censor data |
| Soroban smart contract | Native Stellar programmability — SEP-41 tokens, events, deterministic execution |
| Next.js App Router with Server Components | SEO for grant pages + streaming data from RPC without client waterfalls |
| TanStack Query for client cache | Declarative loading/stale states, background refetch, and optimistic UI without Redux boilerplate |
| Zustand for wallet state | Minimal, persistent wallet session without prop drilling |
| IPFS for milestone proof | Decentralized proof storage; only the content hash (CID) is stored on-chain |

For a full architecture reference including rendering strategy, state management, and data flow diagrams, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Quick Start

### Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Node.js | >= 20 | Use [nvm](https://github.com/nvm-sh/nvm) to manage versions |
| npm | >= 10 | Lockfiles committed per package |
| Rust | stable | Required for smart contracts only |
| `wasm32v1-none` target | — | `rustup target add wasm32v1-none` |
| [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools) | latest | Required for contract deploy/invoke |
| [Freighter Wallet](https://freighter.app) | latest | Browser extension for testing wallet flows |

---

### 1 — Clone the Repository

```bash
git clone https://github.com/StellarGrant/stellargrant-fe.git
cd stellargrant-fe
```

---

### 2 — Frontend (Primary)

```bash
cd web
npm ci

# Copy environment template and fill in your values
cp .env.local.example .env.local
# See Configuration section below for required variables

# Start development server (Turbopack)
npm run dev
```

Open [http://localhost:3000](http://localhost:3000). The app reads from Stellar testnet by default.

To run the frontend alongside a mock API (for offline development):

```bash
npm run dev:mock      # starts both mock server (port 4000) and Next.js (port 3000)
```

---

### 3 — Smart Contracts

```bash
cd contracts

# Add WASM target if not already added
rustup target add wasm32v1-none

# Format check, lint, compile check (mirrors CI)
cargo fmt --all -- --check
cargo clippy --workspace --lib --target wasm32v1-none -- -D warnings
cargo check --workspace --target wasm32v1-none

# Run contract unit tests (not part of CI, but run locally before opening a PR)
cargo test

# Build WASM binary
cd contracts/stellar-grants
make build
```

#### Deploy to Testnet

```bash
cd contracts/contracts/stellar-grants
make build

stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_grants.wasm \
  --network testnet \
  --source-account YOUR_ACCOUNT_ALIAS \
  --alias stellar_grants

# Initialize contract state
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account YOUR_ACCOUNT_ALIAS \
  -- initialize
```

Copy the deployed contract address into `NEXT_PUBLIC_CONTRACT_ID` in your `.env.local`.

---

### 4 — TypeScript Client SDK

```bash
cd client
npm ci
npm run build
npm test
```

---

### 5 — Optional Express API

```bash
cd backend
npm ci

# Set up environment (requires PostgreSQL)
cp .env.example .env
# Edit DATABASE_URL in .env

npm run dev          # starts on port 4000
```

Or run the full stack with Docker:

```bash
docker compose up    # Postgres + API
```

---

## Configuration

### Frontend (`web/.env.local`)

Copy `web/.env.local.example` to `web/.env.local` and fill in the values — **never commit it**.

```env
# ── Stellar Network ───────────────────────────────────────────────────────────
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_CONTRACT_ID=
NEXT_PUBLIC_HORIZON_URL=https://horizon-testnet.stellar.org
NEXT_PUBLIC_STELLAR_RPC_URL=https://soroban-testnet.stellar.org
NEXT_PUBLIC_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

# ── Backend API ───────────────────────────────────────────────────────────────
NEXT_PUBLIC_API_URL=http://localhost:4000

# ── IPFS / Pinata ─────────────────────────────────────────────────────────────
# JWT from https://app.pinata.cloud/keys
# If omitted, useIPFS falls back to mock mode (no real upload, no crash).
NEXT_PUBLIC_PINATA_JWT=
```

> **Security**: Variables prefixed `NEXT_PUBLIC_` are bundled into the client. Never prefix secrets with `NEXT_PUBLIC_`.

### API (`backend/.env`)

Copy `backend/.env.example` to `backend/.env` and fill in the values.

```env
# ── Server ────────────────────────────────────────────────────────────────────
PORT=4000
NODE_ENV=development

# ── Database ──────────────────────────────────────────────────────────────────
DATABASE_URL=postgres://postgres:postgres@localhost:5432/stellargrant

# ── Stellar / Soroban ─────────────────────────────────────────────────────────
USE_MOCK_SOROBAN=true          # true = in-process mock client for local dev/tests
RPC_URL=https://soroban-testnet.stellar.org
CONTRACT_ID=                   # deployed StellarGrants contract address (C...)
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

# ── Admin ─────────────────────────────────────────────────────────────────────
ADMIN_ADDRESSES=               # comma-separated Stellar addresses with admin access

# ── Redis (optional — falls back to in-memory rate limiting when absent) ──────
REDIS_URL=

# ── Metrics ───────────────────────────────────────────────────────────────────
METRICS_ALLOWED_IPS=
METRICS_BASIC_AUTH_USER=
METRICS_BASIC_AUTH_PASSWORD=

# ── SendGrid (email notifications) ─────────────────────────────────────────────
SENDGRID_API_KEY=

# ── IPFS / Pinata ─────────────────────────────────────────────────────────────
PINATA_JWT=
PINATA_GATEWAY=https://gateway.pinata.cloud
```

---

## Packages

### `web/` — Next.js Frontend

The primary user-facing application. Key sub-directories:

```
app/                 Next.js App Router — pages and route handlers (see Pages & Routes)
components/          React components (UI primitives + domain-specific)
  ui/                shadcn/ui base components (Button, Card, Dialog …)
  grants/            Grant cards, creation form, funding progress
  milestones/        Milestone list, proof submission, vote panel
  wallet/            Wallet connect modal, wallet guard, wallet info/address
  contributors/      Contributor profile components
  leaderboard/       Contributor reputation table
  dispute/           Dispute submission and status
  settings/          User preference components
  landing/           Landing page sections
  layout/            Header, footer, sidebar, notification bell
hooks/               TanStack Query + Zustand powered custom hooks
lib/
  stellar/           Stellar SDK wrappers (RPC client, contract calls, event streaming)
  store/             Zustand stores (wallet session)
  wallets/           Adapter pattern — FreighterAdapter, AlbedoAdapter (xBull/Passkey UI stubs)
  schemas/           Zod validation schemas for forms and API responses
  ipfs/              Pinata upload helpers
  search/            Grant search helpers
  tokens/            Token metadata / balance helpers
  config/            Runtime env/config accessors
  errors/            Typed error helpers
  utils/             Misc utilities
types/               Shared TypeScript interfaces (Grant, Milestone, Contributor …)
tests/               Vitest unit and component tests
e2e/                 Playwright end-to-end tests
stories/             Storybook component stories
mock-server/         Standalone mock API used by `npm run dev:mock`
```

**Available Scripts**

| Command | Description |
|---------|-------------|
| `npm run dev` | Dev server with Turbopack |
| `npm run dev:mock` | Dev server + mock API concurrently |
| `npm run build` | Production build |
| `npm start` | Start production server |
| `npm run lint` | ESLint |
| `npm test` | Vitest (watch mode) |
| `npm run test:run` | Vitest (single run) |
| `npm run test:e2e` | Playwright E2E tests |
| `npm run mock` | Start mock API only |
| `npm run storybook` | Storybook on port 6006 |
| `npm run build-storybook` | Build static Storybook |

### `contracts/` — Soroban Contracts

A single Rust crate (`contracts/contracts/stellar-grants`) compiled to WASM and deployed to Stellar, organized as ~90 internal modules under `src/`. Core entry points contributors touch most:

| Function | Description |
|----------|-------------|
| `grant_create` / `grant_create_high_security` | Create a new grant (standard escrow, or gated behind an additional multisig signer set) |
| `grant_fund` | Deposit tokens into the grant's escrow |
| `milestone_submit` / `milestone_submit_batch` | Submit proof of work for one or more milestones (proof URL + description) |
| `milestone_vote` | Reviewer (or delegate) casts approve/reject vote; triggers payout when quorum reached |
| `milestone_reject` | Reviewer rejects a milestone with a reason |
| `cancel_grant` / `grant_complete` | Cancel a grant (refunding escrow per the configured refund policy) or finalize completion |
| `contributor_register` | Register a contributor's profile (name, bio, skills, GitHub URL) |

See [Smart Contract Modules](#smart-contract-modules) below for the full module map — grant lifecycle, governance/voting, funding mechanisms, escrow/payments, reputation, and protocol operations are each implemented as a separate module (e.g. `delegate.rs`, `refund.rs`, `snapshot.rs`, `matching.rs`, `streaming.rs`, `quadratic.rs`).

#### Smart Contract Modules

`src/` groups into roughly these areas (each a separate `.rs` module):

| Area | Modules |
|------|---------|
| **Grant lifecycle** | `factory`, `grant_pause`, `grant_timer`, `grant_renewal`, `grant_transfer`, `grant_tags`, `grant_index`, `fork`, `milestone_deps`, `milestone_extension`, `milestone_template`, `batch`, `batch_read` |
| **Governance & voting** | `dao`, `governance`, `quadratic`, `delegate`, `open_review`, `arbitration_pool`, `dispute` |
| **Funding mechanisms** | `matching` (quadratic funding), `crowdfund`, `bounty`, `syndication`, `referral`, `waitlist`, `whitelist` |
| **Escrow & payments** | `escrow`, `escrow_multisig`, `multisig`, `streaming`, `split_payment`, `fees`, `treasury`, `clawback`, `lockup`, `performance_bond`, `collateral`, `token_swap`, `refund`, `revenue_share` |
| **Reputation & verification** | `reputation`, `reputation_decay`, `badge`, `milestone_nft`, `scoring`, `checklist`, `evidence_schema`, `reviewer_pool`, `reviewer_reward`, `reviewer_sla`, `contributor_verification`, `compliance`, `auto_approve` |
| **Protocol operations & safety** | `access_control`, `emergency`, `circuit_breaker`, `rate_limit`, `reentrancy`, `audit`, `migration`, `versioning`, `config`, `params` |
| **Data, interop & reporting** | `data_export`, `analytics`, `metrics`, `provenance`, `funder_report`, `portfolio`, `merkle`, `cross_contract`, `grant_bridge`, `relay`, `registry`, `pagination`, `notification`, `hooks`, `invoice`, `license`, `insurance`, `oracle` |

For a deeper dive into individual modules, see [contracts/README.md](contracts/README.md), [contracts/THREAT_MODEL.md](contracts/THREAT_MODEL.md), and [contracts/BENCHMARK.md](contracts/BENCHMARK.md).

### `client/` — TypeScript SDK

`@stellargrants/client-sdk` provides a typed interface to the Soroban contract from Node.js, any bundler, or the command line. Useful for scripts, bots, and integration tests.

- **`StellarGrantsSDK`** — the core typed client (`src/StellarGrantsSDK.ts`)
- **CLI** — ships a `sg` binary (`src/cli.ts`) for scripting contract calls from the terminal
- **Vue composables** (optional peer dep on `vue`) — `useGrant`, `useGrants`, `useGrantBalances`, `useGrantHistory`, `useStellarGrants`, `useTransactionHistory`
- **Wallet adapters** — `FreighterAdapter`, `AlbedoAdapter`, `XBullAdapter`, `WalletConnectAdapter`
- **Batch builder, transaction tracker/retry, optimistic state manager** — helpers for building batched calls and tracking in-flight transactions
- **Typed errors** — `StellarGrantsError`, `TransactionFailedError`, `TransactionTimeoutError`, `MetadataValidationError` with parsed Soroban error codes

```bash
cd client && npm ci && npm run build
```

### `backend/` — Express API

Optional Express + TypeORM + PostgreSQL service. **Not required** for core read/write flows (the frontend can talk to Stellar RPC directly) — it adds:

- **Indexing/caching** of on-chain events for fast, paginated, filterable queries (`routes/grants.ts`, `milestone-*`, `disputes.ts`, `analytics.ts`, `search.ts`, `stats.ts`)
- **OAuth accounts** — GitHub/Twitter login via Passport, linked to a Stellar address (`routes/auth.ts`)
- **Notifications & webhooks** — SendGrid email queue, outbound webhook subscriptions + delivery logs, Socket.IO push (`services/notification-*`, `services/webhook-dispatcher.ts`, `routes/webhooks.ts`)
- **RBAC, admin & moderation** — roles/permissions, admin routes, milestone appeals, community/report entities (`routes/admin.ts`, `routes/roles.ts`, `routes/milestone-appeals.ts`, `routes/communities.ts`)
- **Ops** — Redis-backed rate-limit alerting (falls back to in-memory), Prometheus metrics, Swagger docs, a reconciliation service that cross-checks indexed data against on-chain state
- **`USE_MOCK_SOROBAN`** — an in-process mock Soroban client for local dev and tests without a live RPC dependency

---

## Pages & Routes

| Route | Description |
|-------|-------------|
| `/` | Landing page — protocol stats, featured grants, call to action |
| `/grants` | Paginated, filterable grant listing with status / token / sort filters |
| `/grants/[id]` | Grant detail — funding progress, milestone timeline, reviewer panel, event history |
| `/grants/create` | Multi-step grant creation form (wallet required) |
| `/grants/[id]/fund` | Fund a grant — deposit XLM or any supported SEP-41 token into escrow |
| `/grants/[id]/history` | Grant activity/audit history |
| `/grants/[id]/milestones` | Milestone overview for a grant |
| `/grants/[id]/milestones/[idx]` | Single milestone — proof viewer, vote panel, payout status |
| `/dashboard` | User dashboard — my grants, activity feed, pending actions |
| `/profile` | Connected wallet's profile — skills, reputation, grant history |
| `/contributors/[address]` | Public contributor profile page |
| `/leaderboard` | Global contributor reputation ranking |
| `/review` | Reviewer queue — pending milestones awaiting your vote |
| `/dispute` | Dispute management interface |
| `/search` | Full-text grant search |
| `/settings` | User preferences |

---

## Testing

### Unit & Component Tests (Vitest)

```bash
cd web
npm test             # watch mode
npm run test:run     # single pass with coverage
```

Tests live in `tests/` and co-located `*.test.tsx` files. Note: Vitest and Playwright are **not** currently run in CI (the `frontend` job only lints and builds) — run them locally before opening a PR.

### End-to-End Tests (Playwright)

```bash
cd web
npm run test:e2e             # headless
npm run test:e2e:headed      # with browser visible
```

E2E tests cover critical user flows: grant creation, funding, milestone submission, and reviewer voting.

### Contract Tests (Rust)

```bash
cd contracts
cargo test
```

Note: contract tests are **not** part of CI (CI only runs `fmt`/`clippy`/`check`) — run them locally before opening a PR.

### API Tests

```bash
cd backend
npm run test:e2e           # end-to-end API tests
npm run test:integration   # integration tests
```

---

## CI / CD

GitHub Actions workflow: [`.github/workflows/ci.yml`](.github/workflows/ci.yml)

`contracts`, `backend`, `frontend`, and `client-sdk` run on every push to `main` and every pull request; `client-docs` runs only when a GitHub Release is published.

| Job | Steps |
|-----|-------|
| **contracts** | `cargo fmt --check`, `cargo clippy --target wasm32v1-none` (deny warnings), `cargo check --target wasm32v1-none` — note: does **not** run `cargo test` |
| **backend** | `npm install` against a real Postgres 15 service container, run migrations, `test:e2e`, `test:integration`, `test:coverage` |
| **frontend** | `npm install`, `npm run lint`, `npm run build` — note: does **not** run Vitest or Playwright |
| **client-sdk** | `npm install`, `npm run test` (Jest) |
| **client-docs** | On release only: build and publish TypeDoc API docs to GitHub Pages |

Because CI doesn't cover contract/frontend tests, always run `cargo test` (contracts), `npm run test:run` and `npm run test:e2e` (web), and `npm run lint` / `npm run build` locally before opening a PR.

---

## Deployment

### Vercel (Recommended for Frontend)

```bash
npm install -g vercel
cd web
vercel           # preview
vercel --prod    # production
```

Set all `NEXT_PUBLIC_*` variables and server-side secrets in Vercel's Environment Variables dashboard. Use separate testnet values for Preview and mainnet values for Production.

### Docker (API + Postgres)

```bash
docker compose up --build
```

The `backend/Dockerfile` builds a Node 20-alpine image. PostgreSQL is provisioned as a compose service.

---

## Wave Program

The StellarGrants Protocol participates in the **Stellar Wave Program** on [Drips](https://drips.network/wave/stellar). Frontend and contract issues labeled `drips-wave` are eligible for Wave Point rewards.

**Tips for Wave contributors:**
- Comment on an issue to claim it before starting work
- Open a draft PR early to get feedback
- Include before/after screenshots for all UI changes
- Complete the full PR checklist before requesting review

---

## Contributing

We welcome contributions of all kinds — bug fixes, new features, documentation, tests, and contract improvements. With over 60 contributors, StellarGrants is an active, community-driven project.

- **Root contribution guide**: [CONTRIBUTING.md](CONTRIBUTING.md)
- **Frontend-specific guide**: [web/CONTRIBUTING.md](web/CONTRIBUTING.md)
- **Contract contribution guide**: [contracts/ContributionGuide.md](contracts/ContributionGuide.md)
- **Architecture reference**: [ARCHITECTURE.md](ARCHITECTURE.md)
- **Developer setup**: [DEVELOPMENT.md](DEVELOPMENT.md)
- **Code of Conduct**: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
- **Beginner tutorial**: [TUTORIAL.md](TUTORIAL.md)

---

## Security

- Run all tests and linters locally before pushing to public networks
- Review access control and numeric safety for every contract change
- Never commit private keys, seeds, or production secrets to this repository
- Report vulnerabilities via [GitHub Security Advisories](https://github.com/StellarGrant/stellargrant-fe/security/advisories) — see [SECURITY.md](SECURITY.md) for the full policy

---

## License

This project is licensed under the [MIT License](LICENSE).

---

<div align="center">

**Fix. Merge. Earn.** | [Stellar Wave Program](https://drips.network/wave/stellar)

Made with care for the Stellar ecosystem by [60+ open-source contributors](https://github.com/StellarGrant/stellargrant-fe/graphs/contributors)

</div>
