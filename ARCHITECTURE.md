# StellarGrants Protocol — Architecture Reference

This document covers the system architecture in depth: data flow, rendering strategy, state management, contract integration, and key design decisions. It is intended for contributors who want to understand _why_ the system is structured the way it is before making changes.

---

## Table of Contents

- [System Overview](#system-overview)
- [Monorepo Packages](#monorepo-packages)
- [Smart Contract Layer](#smart-contract-layer)
- [Frontend Architecture](#frontend-architecture)
  - [Rendering Strategy](#rendering-strategy)
  - [Data Flow](#data-flow)
  - [State Management](#state-management)
  - [Wallet Integration](#wallet-integration)
  - [IPFS Integration](#ipfs-integration)
  - [Real-Time Events](#real-time-events)
- [Optional API Layer](#optional-api-layer)
- [Client SDK](#client-sdk)
- [Security Architecture](#security-architecture)
- [Key Design Decisions](#key-design-decisions)

---

## System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                           Browser (User)                             │
│                                                                      │
│   ┌──────────────────────────┐    ┌──────────────────────────────┐  │
│   │    Next.js App           │    │   Wallet Extension           │  │
│   │  (Server + Client React) │    │   (Freighter / xBull)        │  │
│   └────────────┬─────────────┘    └──────────────┬───────────────┘  │
│                │ RPC reads                        │ sign XDR tx      │
└────────────────┼─────────────────────────────────┼──────────────────┘
                 │                                  │
                 ▼                                  ▼
┌──────────────────────────────────────────────────────────────────────┐
│                       Stellar Network                                │
│                                                                      │
│  ┌────────────────────┐        ┌───────────────────────────────────┐│
│  │  Soroban RPC Node  │        │  Horizon API                      ││
│  │  simulateTransaction│        │  (account info, balances,         ││
│  │  sendTransaction   │        │   trustlines, SEP-24)             ││
│  │  getEvents         │        └───────────────────────────────────┘│
│  └────────┬───────────┘                                              │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                StellarGrants Soroban Contract                   ││
│  │                                                                 ││
│  │  grant_create     grant_fund     milestone_submit               ││
│  │  milestone_vote   contributor_register   dispute_raise          ││
│  │                                                                 ││
│  │  Storage: grants · milestones · contributors · escrow balances  ││
│  │  Events:  GrantCreated · MilestoneSubmitted · VoteCast ·        ││
│  │           MilestonePaid · DisputeRaised                        ││
│  └─────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────┘
                 │
                 ▼ (optional — for caching and indexing)
┌──────────────────────────────┐
│  Express API  (backend/)     │
│  TypeORM + PostgreSQL        │
│  ← indexes on-chain events   │
│  ← caches contract reads     │
│  ← relays SSE to frontend    │
└──────────────────────────────┘
```

---

## Monorepo Packages

| Package | Language | Role |
|---------|----------|------|
| `web/` | TypeScript / Next.js | Primary web UI — all user-facing features |
| `contracts/` | Rust / Soroban SDK | On-chain logic: escrow, milestones, voting, payouts |
| `client/` | TypeScript | Typed SDK for programmatic contract interaction |
| `backend/` | TypeScript / Express | Optional caching and indexing layer |

---

## Smart Contract Layer

The Soroban contract is the single source of truth for all protocol state. It stores:

- **Grants**: owner, recipient, title, description, budget, deadline, status, reviewer list, token
- **Milestones**: per-grant list with submission proof (IPFS CID), vote counts, payout amounts, timestamps
- **Escrow balances**: token holdings per grant, released only on milestone approval
- **Contributors**: address → GitHub handle, skills, reputation score, stats

### Contract Events

The contract emits structured events on every state transition. The frontend consumes these for real-time updates:

| Event | Trigger |
|-------|---------|
| `GrantCreated` | `grant_create` called |
| `GrantFunded` | `grant_fund` called — includes funder, token, amount |
| `MilestoneSubmitted` | `milestone_submit` called — includes IPFS CID |
| `VoteCast` | `milestone_vote` called — includes reviewer, approve/reject |
| `MilestonePaid` | Vote quorum reached; payout executed automatically |
| `DisputeRaised` | `dispute_raise` called |

### Voting and Quorum

Each grant has a reviewer list set at creation time. A milestone is approved when the number of `approve` votes reaches the configured quorum threshold. The contract executes the token transfer atomically in the same call that casts the deciding vote — no separate payout step.

### Escrow

Tokens sent via `grant_fund` are held in the contract's storage, keyed by grant ID and token address. The contract never releases tokens without a valid quorum approval. This is enforced entirely on-chain; the frontend has no ability to bypass it.

---

## Frontend Architecture

### Rendering Strategy

Next.js 16 App Router with a mix of Server and Client Components:

| Route | Rendering | Why |
|-------|-----------|-----|
| `/` | Server Component | Static stats + SEO metadata; no wallet needed |
| `/grants` | Server Component + ISR (60s) | Grant listing benefits from SSR for SEO; revalidated frequently |
| `/grants/[id]` | Server Component | Full grant detail with SSR for shareable links |
| `/grants/create` | Client Component | Multi-step form with wallet integration |
| `/grants/[id]/fund` | Client Component | Wallet signing required |
| `/grants/[id]/milestones/[idx]` | Hybrid | Server fetches initial state; client handles vote interaction |
| `/leaderboard` | Server Component + ISR | Reputation scores; SEO-friendly |
| `/dashboard` | Client Component | Wallet address required to query user-specific data |
| `/review` | Client Component | Reviewer queue depends on connected wallet |
| Event feed | Client Component (SSE) | Real-time contract events |

### Data Flow

**Read path (Server Component):**
```
Next.js Server  →  lib/stellar/contract.ts (simulateTransaction)
                →  Soroban RPC (read-only)
                →  Decode ScVal response
                →  Return typed data (Grant / Milestone / Contributor)
                →  Render Server Component HTML
```

**Read path (Client Component, TanStack Query):**
```
useGrants() hook
  →  queryFn: lib/grants/api.ts (calls /api/grants or direct RPC)
  →  TanStack Query caches result (staleTime: 30s)
  →  Component renders from cache; background refetch on stale
```

**Write path (wallet transaction):**
```
User clicks action
  →  useContractTransaction() hook
  →  lib/stellar/contract.ts.buildTransaction()
  →  simulateTransaction (get resource fees + footprint)
  →  Assemble transaction with simulated data
  →  WalletAdapter.signTransaction() (prompts Freighter / xBull)
  →  sendTransaction to Soroban RPC
  →  Poll getTransaction() until SUCCESS or FAILED
  →  Invalidate relevant TanStack Query keys
  →  Show success toast / error toast
```

### State Management

Three distinct layers, each with its own persistence scope:

| Layer | Tool | What it manages | Persistence |
|-------|------|----------------|-------------|
| Server state | TanStack Query v5 | Grants, milestones, balances, contributors | Memory + background sync |
| Global UI state | Zustand | Wallet session (address, type, network) | `localStorage` |
| Local UI state | React `useState` | Form inputs, modal open/close, selected tab | Component lifetime |

**Zustand wallet store** (`lib/store/walletStore.ts`):
```typescript
interface WalletState {
  address: string | null;
  walletType: "freighter" | "albedo" | "xbull" | null;
  network: "testnet" | "mainnet" | "futurenet";
  isConnected: boolean;
  connect: (type: WalletType) => Promise<void>;
  disconnect: () => void;
}
```

**TanStack Query key structure:**
```
["grants"]                    → paginated grant list
["grant", id]                 → single grant
["milestones", grantId]       → all milestones for a grant
["milestone", grantId, idx]   → single milestone
["contributor", address]      → contributor profile
["leaderboard"]               → reputation rankings
["balance", address, token]   → wallet token balance
```

### Wallet Integration

The frontend uses an **adapter pattern** (`lib/wallets/`) to decouple wallet implementations from application code:

```typescript
interface WalletAdapter {
  connect(): Promise<string>;          // returns Stellar address
  getAddress(): Promise<string>;
  signTransaction(xdr: string, options: SignOptions): Promise<string>;
  disconnect(): void;
}
```

Concrete adapters: `FreighterAdapter`, `AlbedoAdapter`, `xBullAdapter`

**Freighter** (primary): uses `@stellar/freighter-api` for connection, address retrieval, and transaction signing.

**Stellar Passkeys** (WebAuthn/Secp256r1): signs transactions using device biometrics — no seed phrase. Implemented via the LAC (Launchtube Account Contract) pattern.

**Wallet session persistence**: Zustand persists the connected wallet type and address to `localStorage`. On next page load, the app attempts to reconnect automatically.

### IPFS Integration

Milestone proofs are uploaded to IPFS via **Pinata** (`lib/ipfs/`). Only the resulting content hash (CID) is stored on-chain. This keeps on-chain storage costs low while ensuring proof immutability.

**Upload flow:**
```
User selects file / pastes URL
  →  useIPFS().upload(file)
  →  Server-side API route (/api/ipfs/upload) — keeps Pinata secrets off client
  →  Pinata API: pin file → returns CID
  →  CID passed to milestone_submit contract call
```

**Viewing proofs:**
```
CID from contract
  →  NEXT_PUBLIC_IPFS_GATEWAY + "/" + CID
  →  ProofViewer component renders image / PDF / markdown
```

### Real-Time Events

The frontend offers two event channels depending on what's available:

1. **Server-Sent Events (SSE)** via `/api/events` route handler: relays contract events from the optional Express API, which indexes them from Stellar RPC.
2. **Direct RPC polling** via `useContractEvents` hook: uses `getEvents` RPC method with ledger cursor to stream events directly from Soroban RPC without the API layer.

Components subscribe to events via `useContractEvents(contractId, eventFilters)`. The hook decodes `ScVal` event data into typed `ContractEvent` objects using `lib/stellar/decode.ts`.

---

## Optional API Layer

`backend/` is a standalone Express + TypeORM service. It is **not required** for any core read or write flow. Use it when:

- You need faster grant/milestone queries than direct RPC allows (the API indexes events into PostgreSQL)
- You want server-side SSE relay for real-time events
- You need server-side signature validation before forwarding transactions

**API exposes:**

| Endpoint | Description |
|----------|-------------|
| `GET /grants` | Indexed grant list with pagination and filtering |
| `GET /grants/:id` | Single grant with milestones |
| `GET /milestones` | Milestones across grants |
| `GET /leaderboard` | Reputation rankings |
| `GET /contributors/:address` | Contributor profile |
| `GET /stats` | Protocol-wide statistics |
| `GET /events` | SSE stream of contract events |

The API does not write to the contract — that always happens from the browser via wallet signing.

---

## Client SDK

`client/` exports `@stellargrants/client-sdk`: a typed TypeScript wrapper around the Soroban contract for use in Node.js scripts, bots, or integration tests.

```typescript
import { StellarGrantsClient } from "@stellargrants/client-sdk";

const client = new StellarGrantsClient({
  rpcUrl: "https://soroban-testnet.stellar.org",
  contractId: "C...",
  networkPassphrase: "Test SDF Network ; September 2015",
});

const grants = await client.listGrants();
const grant = await client.getGrant("1");
```

The SDK uses the same `@stellar/stellar-sdk` as the frontend but exposes a higher-level API with full TypeScript types.

---

## Security Architecture

### Content Security Policy

`next.config.ts` sets strict CSP headers on all responses. Key directives:
- `default-src 'self'` — blocks all third-party scripts by default
- `connect-src` — whitelists Stellar RPC, Horizon, Pinata IPFS gateway
- `frame-ancestors 'none'` — prevents clickjacking
- `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin`

### Secret Isolation

- `NEXT_PUBLIC_*` variables are bundled into client-side JavaScript — only non-sensitive config goes here
- Pinata API key and secret are server-only (API routes / Server Components) — never shipped to browser
- Private keys and seeds are never stored anywhere in the application — wallets handle signing entirely

### Transaction Safety

- Every write-path transaction is **simulated first** (`simulateTransaction`) to get the exact resource fee and footprint before presenting it to the wallet for signing
- The wallet signs the assembled XDR; the frontend never constructs a transaction with a secret key
- The contract enforces all authorization — the frontend cannot bypass access controls by manipulating data

### Input Sanitization

- All user-provided rich text is passed through `DOMPurify` before rendering
- Markdown is rendered via `marked` with sanitized output
- Form inputs are validated client-side with Zod schemas before any contract call is attempted

---

## Key Design Decisions

### Zero-Backend for Core Flows

**Decision:** The frontend reads all state directly from Stellar RPC; no custom backend is required for viewing or interacting with grants.

**Why:** A centralized backend would become a single point of failure and a censorship vector. The Soroban contract is the authoritative state machine; reading from it directly removes an entire layer of trust. The optional `backend/` layer is additive, not load-bearing.

### Adapter Pattern for Wallets

**Decision:** Wallet implementations are wrapped behind a `WalletAdapter` interface rather than using wallet-specific APIs directly in components.

**Why:** Freighter, xBull, and Passkey wallets have different APIs. Abstracting them lets the rest of the app call `wallet.signTransaction(xdr)` without caring which wallet is connected. Adding a new wallet requires only a new adapter file.

### IPFS for Proof Storage

**Decision:** Milestone proof files are stored on IPFS (Pinata); only the CID is written to the contract.

**Why:** Storing binary blobs on-chain would be prohibitively expensive in Soroban storage fees. IPFS provides content-addressed, immutable storage. The CID on-chain is sufficient to verify that proof has not been tampered with.

### TanStack Query over Redux / SWR

**Decision:** Use TanStack Query v5 for all server state.

**Why:** Soroban RPC reads are async, can fail, and become stale. TanStack Query's loading/error/success states, background refetch, and query invalidation map perfectly to this model. Redux would require significant boilerplate for equivalent caching behavior. SWR lacks the query key invalidation API needed for coordinating state after write transactions.

### Zustand over Context for Wallet State

**Decision:** Wallet session lives in a Zustand store, not React Context.

**Why:** Wallet state is read by many unrelated components (header, grant pages, form actions). React Context requires a provider high in the tree and causes unnecessary re-renders when context value changes. Zustand's selector-based subscriptions prevent components from re-rendering on unrelated state changes.
