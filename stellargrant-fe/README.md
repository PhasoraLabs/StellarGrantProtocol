# StellarGrants Protocol - Frontend

<div align="center">

**A Next.js 14 web application for the StellarGrants Protocol smart contract on the Stellar blockchain**

[![Next.js](https://img.shields.io/badge/Next.js-14-black)](https://nextjs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-blue)](https://www.typescriptlang.org/)
[![Stellar SDK](https://img.shields.io/badge/Stellar%20SDK-12.x-7D00FF)](https://stellar.github.io/js-stellar-sdk/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

[Getting Started](#getting-started) • [Documentation](#documentation) • [Contributing](./CONTRIBUTING.md) • [Wave Program](#wave-program)

</div>

---

## 🎯 Overview

The StellarGrants frontend is a **zero-backend** Next.js 14 web application that provides a complete user interface for the StellarGrants Protocol smart contract on the Stellar blockchain. It enables grant creators, contributors, and reviewers to interact with on-chain grant management flows — including creating grants, funding escrow, submitting milestone work, and participating in DAO-based approval voting — all through a clean, responsive web interface.

The frontend communicates directly with Soroban smart contracts via the Stellar JavaScript SDK and supports Stellar wallet connections through **Freighter**, **xBull**, and **Stellar Passkeys** (WebAuthn/Secp256r1).

### Key Features

- 🔐 **Wallet-First UX** — Connect any Stellar-compatible wallet in one click
- ⛓️ **Zero-Backend Architecture** — All state lives on-chain; frontend reads from Stellar RPC
- 🔄 **Real-Time Updates** — Subscribe to contract events via streaming RPC
- 🎨 **Modern UI** — Built with Tailwind CSS and shadcn/ui components
- 📱 **Responsive Design** — Works seamlessly on desktop and mobile devices
- 🌊 **Wave-Friendly** — Each UI section maps directly to a GitHub issue for contributors

## 🚀 Getting Started

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Node.js | >= 18.17 | [nodejs.org](https://nodejs.org) or [nvm](https://github.com/nvm-sh/nvm) |
| pnpm | >= 8 | `npm install -g pnpm` |
| Freighter Wallet | Latest | [Chrome Extension](https://freighter.app) |

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/stellargrant-fe.git
cd StellarGrant-fe

# Install frontend dependencies
cd stellargrant-fe
pnpm install

# Copy environment variables
cp .env.example .env.local
# Edit .env.local with your testnet contract ID and API keys

# (Optional) Set up the mock API server
cd ../api
npm install
cd ../stellargrant-fe

# Start development server
pnpm dev
```

The frontend application will be available at [http://localhost:3000](http://localhost:3000).

### Running the Mock API Server

The project includes a mock API server for development and testing. It provides endpoints for caching grant state and validating signed writes.

```bash
# From the repository root
cd api
npm install
npm run dev
```

The mock server runs on port 4000 by default. You can configure this via the `PORT` environment variable.

**API Endpoints:**
- `GET /health` - Health check endpoint
- `GET /grants` - List all grants
- `GET /grants/:id` - Get grant by ID
- `POST /milestone_proof` - Submit milestone proof with signature validation

**Running Both Services:**

For full development with the mock API, run both services in separate terminals:

```bash
# Terminal 1 - API Server
cd api
npm run dev

# Terminal 2 - Frontend
cd stellargrant-fe
pnpm dev
```

### Environment Variables

Create a `.env.local` file in the root directory. **Never commit this file to version control.**

```env
# ── Stellar Network ───────────────────────────────────────
NEXT_PUBLIC_STELLAR_NETWORK=testnet
# Options: testnet | mainnet | futurenet

NEXT_PUBLIC_STELLAR_RPC_URL=https://soroban-testnet.stellar.org
# Mainnet: https://soroban.stellar.org

NEXT_PUBLIC_NETWORK_PASSPHRASE=Test SDF Network ; September 2015
# Mainnet: Public Global Stellar Network ; September 2015

# ── Contract ──────────────────────────────────────────────
NEXT_PUBLIC_CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
# Replace with your deployed StellarGrants contract address

# ── Token ─────────────────────────────────────────────────
NEXT_PUBLIC_NATIVE_TOKEN=CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
# Native XLM asset contract on testnet

NEXT_PUBLIC_USDC_TOKEN=CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
# USDC on testnet (Circle / Stellar)

# ── IPFS (for milestone proof uploads) ────────────────────
NEXT_PUBLIC_IPFS_GATEWAY=https://gateway.pinata.cloud/ipfs
PINATA_API_KEY=your_pinata_api_key
PINATA_SECRET_KEY=your_pinata_secret_key

# ── Analytics (optional) ──────────────────────────────────
NEXT_PUBLIC_POSTHOG_KEY=
NEXT_PUBLIC_POSTHOG_HOST=https://app.posthog.com
```

⚠️ **Security Note**: Variables prefixed with `NEXT_PUBLIC_` are exposed to the browser. Never prefix secrets (API keys, private keys) with `NEXT_PUBLIC_`. Server-only secrets like `PINATA_SECRET_KEY` are only available in API routes and server components.

## 📦 Tech Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| **Next.js** | 14.x (App Router) | React framework, SSR, file-based routing |
| **TypeScript** | 5.x | Type safety across components and contract calls |
| **@stellar/stellar-sdk** | 12.x | Stellar network, transactions, XDR encoding |
| **@stellar/freighter-api** | 2.x | Freighter browser wallet integration |
| **Tailwind CSS** | 3.x | Utility-first styling |
| **shadcn/ui** | Latest | Accessible component primitives (Radix UI) |
| **Zustand** | 4.x | Lightweight global state for wallet + grants |
| **TanStack Query** | 5.x | Async data fetching, caching, background sync |
| **Zod** | 3.x | Runtime validation for form inputs and API responses |
| **React Hook Form** | 7.x | Form state and validation |
| **Vitest** | 1.x | Unit and component tests |
| **Playwright** | Latest | End-to-end browser tests |

## 📁 Project Structure

```
StellarGrant-fe/
├── api/                          # Mock API server (optional for development)
│   ├── src/
│   │   ├── index.ts              # API entry point and bootstrap
│   │   ├── app.ts                # Express app configuration
│   │   ├── config/               # Environment configuration
│   │   ├── db/                   # TypeORM data source
│   │   ├── entities/             # Database entities
│   │   ├── routes/               # API route handlers
│   │   ├── services/             # Business logic
│   │   └── soroban/              # Stellar contract integration
│   ├── tests/                   # E2E tests
│   ├── Dockerfile
│   └── package.json
├── stellargrant-fe/              # Next.js frontend application
│   ├── app/                      # Next.js App Router
│   │   ├── layout.tsx            # Root layout (wallet provider, fonts)
│   │   ├── page.tsx              # Homepage / grant discovery
│   │   ├── grants/
│   │   │   ├── page.tsx          # Grant listing page
│   │   │   ├── [id]/
│   │   │   │   ├── page.tsx      # Grant detail view
│   │   │   │   ├── fund/page.tsx # Fund grant flow
│   │   │   │   └── milestones/
│   │   │   │       ├── page.tsx  # Milestone list
│   │   │   │       └── [idx]/page.tsx # Milestone detail + vote
│   │   │   └── create/page.tsx   # Create grant form
│   │   ├── profile/page.tsx      # Contributor profile
│   │   ├── leaderboard/page.tsx  # Contributor reputation board
│   │   └── api/                  # API routes (if any)
│   ├── components/
│   │   ├── ui/                   # shadcn/ui base components
│   │   ├── grants/               # Grant-specific components
│   │   ├── milestones/           # Milestone components
│   │   ├── wallet/               # Wallet connect components
│   │   └── layout/               # Header, Footer, Sidebar
│   ├── hooks/                    # Custom React hooks
│   │   ├── useWallet.ts          # Wallet connection and state
│   │   ├── useGrants.ts          # Grant data fetching
│   │   ├── useGrant.ts           # Single grant operations
│   │   ├── useMilestone.ts       # Milestone operations
│   │   ├── useContractTransaction.ts # Contract transaction handling
│   │   ├── useContractEvents.ts  # Event streaming
│   │   └── useIPFS.ts            # IPFS file upload
│   ├── lib/
│   │   ├── stellar/              # Stellar SDK wrappers
│   │   │   ├── client.ts         # RPC client singleton
│   │   │   ├── contract.ts       # Contract call helpers
│   │   │   └── events.ts         # Event streaming
│   │   ├── store/                # Zustand stores
│   │   ├── utils/                # Shared utilities
│   │   └── config/               # Configuration files
│   ├── types/                    # Global TypeScript types
│   ├── public/                   # Static assets
│   ├── tests/                    # Vitest unit tests
│   ├── e2e/                      # Playwright e2e tests
│   ├── .env.local                # Environment variables (not committed)
│   ├── .env.example              # Environment variable template
│   ├── next.config.ts
│   ├── tailwind.config.ts
│   └── package.json
├── client/                       # StellarGrants SDK (if applicable)
└── docker-compose.yml            # Docker compose configuration
```

## 🛠️ Available Scripts

### Frontend Scripts (stellargrant-fe/)

| Command | Description |
|---------|-------------|
| `pnpm dev` | Start development server with hot reload |
| `pnpm build` | Build optimized production bundle |
| `pnpm start` | Start production server |
| `pnpm lint` | Run ESLint across all files |
| `pnpm type-check` | Run TypeScript compiler (no emit) |
| `pnpm test` | Run Vitest unit and component tests |
| `pnpm test:e2e` | Run Playwright end-to-end tests |
| `pnpm test:coverage` | Generate Vitest coverage report |
| `pnpm format` | Run Prettier on all files |
| `pnpm analyze` | Bundle analyzer (@next/bundle-analyzer) |

### API Scripts (api/)

| Command | Description |
|---------|-------------|
| `npm run dev` | Start API development server with hot reload |
| `npm run build` | Build TypeScript to JavaScript |
| `npm start` | Start production API server |
| `npm run test:e2e` | Run API end-to-end tests |
| `npm run typeorm:sync` | Synchronize database schema |

## 🏗️ Architecture

### Zero-Backend Design

The StellarGrants frontend follows a **zero-backend architecture** — there is no custom API server. All data is sourced directly from the Stellar blockchain via RPC. Server Components handle initial data fetching, and Client Components manage wallet interaction and real-time updates.

### Data Flow

```
Browser                        Stellar Network
───────────────────────────    ─────────────────────────────

  Next.js Page (Server)   ──► Stellar RPC (simulateTransaction)
       │                           │                         
       │ initial data              │ read-only contract views
       ▼                           ▼                         
  React Component         ◄── ContractClient.ts             
  (Client)                         │                         
       │                           │                         
       │ wallet signs tx           │ submit transaction      
       ▼                           ▼                         
  Freighter / xBull  ──────► Stellar RPC (sendTransaction)  
  (Wallet Extension)                │                         
                                    │ events stream           
  EventListener ◄───────────────────┘                        
  (SSE / polling)
```

### Rendering Strategy

| Page / Component | Rendering | Reason |
|------------------|-----------|--------|
| `app/page.tsx` (Home) | Server Component | Static grant count + SEO metadata |
| `app/grants/page.tsx` | Server Component + ISR (60s) | Grant listing, revalidated frequently |
| `app/grants/[id]/page.tsx` | Server Component | Grant detail with SSR for SEO |
| Grant fund flow | Client Component | Wallet interaction required |
| Milestone vote UI | Client Component | Real-time vote count + wallet tx |
| Wallet connect button | Client Component | Browser-only wallet API |
| Event feed | Client Component (SSE) | Real-time contract events stream |

## 🔌 Wallet Integration

The frontend supports multiple Stellar wallet options:

### Freighter (Primary)

Freighter is the primary Stellar browser wallet. The app uses `@stellar/freighter-api` for connection and signing.

```typescript
import { isConnected, getAddress, signTransaction } from "@stellar/freighter-api";

// Check if Freighter is installed
const { isConnected: hasFreighter } = await isConnected();

// Get connected address
const { address } = await getAddress();

// Sign a transaction XDR
const { signedTxXdr } = await signTransaction(txXdr, {
  networkPassphrase: process.env.NEXT_PUBLIC_NETWORK_PASSPHRASE,
});
```

### Stellar Passkeys (WebAuthn)

StellarGrants supports Passkey-based authentication via Secp256r1 (WebAuthn), allowing users to sign transactions with their device biometrics instead of a seed phrase.

### xBull

Support for xBull wallet extension is also available.

## 📄 Pages & Routes

### Home (`/`)
Landing page showing protocol stats, featured grants, and a call-to-action for new contributors.

### Grant Listing (`/grants`)
Paginated, filterable list of all grants stored on-chain.

**Query Parameters:**
- `status`: `open | active | completed | cancelled`
- `token`: `XLM | USDC | all`
- `page`: number (pagination)
- `sort`: `newest | funded | deadline`
- `q`: string (search query)

### Grant Detail (`/grants/[id]`)
Full grant page showing metadata, funding progress, milestone list, reviewer panel, and event history.

### Create Grant (`/grants/create`)
Multi-step form for creating a new grant on-chain. Wallet connection required.

### Fund Grant (`/grants/[id]/fund`)
Dedicated funding flow for depositing XLM or USDC into a grant's escrow.

### Milestone Detail (`/grants/[id]/milestones/[idx]`)
Shows a single milestone's submitted proof, vote count, reviewer list, and action buttons.

### Profile (`/profile`)
Shows the connected wallet's contributor profile: registered GitHub handle, skills, grants participated in, milestones completed, and reputation score.

### Leaderboard (`/leaderboard`)
Contributor reputation board showing top contributors by reputation score.

## 🧪 Testing

### Unit & Component Tests (Vitest)

```bash
pnpm test
```

Tests live in `tests/` and co-located `*.test.tsx` files.

### End-to-End Tests (Playwright)

```bash
pnpm test:e2e
```

E2E tests live in `e2e/` and run against a testnet-connected dev server.

## 🚢 Deployment

### Vercel (Recommended)

```bash
# Install Vercel CLI
pnpm add -g vercel

# Deploy to preview
vercel

# Deploy to production
vercel --prod
```

Set all environment variables in the Vercel dashboard under **Project → Settings → Environment Variables**. Use separate values for Preview (testnet) and Production (mainnet) environments.

### Docker

```bash
# Build Docker image
docker build -t stellar-grants-frontend .

# Run locally
docker run -p 3000:3000 \
  -e NEXT_PUBLIC_STELLAR_NETWORK=testnet \
  -e NEXT_PUBLIC_CONTRACT_ID=CXXX... \
  stellar-grants-frontend
```

## 🌊 Wave Program

The StellarGrants frontend contributes directly to the **Stellar Wave Program** on Drips. Frontend-specific issues are labeled with `drips-wave` and are eligible for Wave Point rewards.

**Wave Tip**: Frontend issues with UI screenshots in the PR description earn faster reviews.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed contribution guidelines.

## 📚 Documentation

- [Frontend Documentation](./docs/FRONTEND.md) - Complete technical documentation
- [API Reference](./docs/API.md) - Contract interaction patterns
- [Component Library](./docs/COMPONENTS.md) - UI component documentation
- [Testing Guide](./docs/TESTING.md) - Testing strategies and examples

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for details on:

- Code of conduct
- Development workflow
- Pull request process
- Coding standards
- Issue reporting

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- **Protocol Contract**: [StellarGrants Smart Contract](https://stellar.expert/explorer/testnet/contract/CXXX...)
- **Stellar Documentation**: [developers.stellar.org](https://developers.stellar.org)
- **Soroban Docs**: [soroban.stellar.org](https://soroban.stellar.org)
- **Wave Program**: [drips.network/wave/stellar](https://drips.network/wave/stellar)
- **GitHub Issues**: [Issues](https://github.com/your-org/stellargrant-fe/issues)

## 🙏 Acknowledgments

- Built for the Stellar ecosystem
- Powered by Soroban smart contracts
- Supported by the Stellar Development Foundation
- Community-driven through the Wave Program

---

<div align="center">

**Fix. Merge. Earn. 🌊** | [drips.network/wave/stellar](https://drips.network/wave/stellar)

Made with ❤️ for the Stellar community

</div>
