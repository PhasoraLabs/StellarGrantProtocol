/**
 * Grant fixtures for the mock API server.
 *
 * The fixtures deliberately mirror the frontend's canonical types in
 * `web/types/index.ts` (`Grant` / `Milestone`) so the mock server exercises
 * exactly the same code paths as production:
 *
 *   - `budget`, `funded`, `deadline`, `created_at` are `bigint`
 *   - `status` is the numeric enum (0 Pending · 1 Active · 2 In Progress ·
 *     3 Complete · 4 Cancelled) instead of a free-form string
 *   - `recipient`, `owner` and `reviewers` are Stellar addresses
 *   - `milestones` is the milestone *count* (per the `Grant` type); the full
 *     `Milestone[]` payload for each grant lives in `grantMilestones`
 *
 * JSON cannot carry `bigint`, so routes serialize these fields to strings
 * before responding — `useGrants` converts them back when parsing.
 */

import type { Grant, Milestone } from "../../types";

export type { Grant, Milestone };

// ── Addresses (valid ed25519 public keys) ─────────────────────────────────────

export const OWNER_ADDRESSES = [
  "GCK4LZF4RXGPTVCQFSAZ5RYS7CY26BSSML5TJLSBVWUNFJDTNGJ2LOWH",
  "GCZLRS6VV4I7XJGM4WBGSFBSZRH2VFKPTW43ONK4PLWPYDLCE7AYTZPU",
  "GBE5F52IVCZSTRFUWYR5IYGYPSNDMSUY77CEFIZAS5HJ24WVGMWUOTMT",
  "GB5KYYPFJW2R5QE5LMVQBMO2U3NF5LGAEWA6VQGEGCQ5JOUUXFF3KIU5",
  "GASIOGLGTYUGHNHRS42KQ2IDN3LL5DO2YPY5GXKCBQH5TH47W53UV2HW",
  "GDIA7B5HTOU7QTUPSFB53DDSFMYIIMRVUZGUDAEGRIXZ5FRBDR2QLHIO",
  "GAEGDGXRC5ZEWKPWPYIC5OZJEXUKG4JWMKK5T6DU7FRSBVHFSGSV62L4",
  "GB4N5H6HRZRJ3BR6ODSH5JI6Y42X2CP7LD5VZBK3NLAMMRPDG2V6GXFB",
] as const;

export const RECIPIENT_ADDRESSES = [
  "GASUAQLFSMVODTPT4GP55RASDBE6ZCN5J76XLO5LRQTNJDNTTNK2NRQK",
  "GAYXHUQSLRWM5SK6BNAST4HELZA3HI6JIBLL7KYU2OKSEG6E67IG4KQ4",
  "GAUUBKSL5WKRH35PIVWWYKZTOT6FRJX7EOI4NTE4XF3N3F3VAWNWXD65",
  "GA75H4PGENRTKD4BRHQCZTRFI6L2YYFYWT55PIRED6SNYL6FXR54MAX4",
  "GB74SURBLGZEI55JP4L5YAAUGSB2ZXSQHSJ4LKUDZEJ76IO3BE3TJOUP",
  "GDS4M6TZIDMTZ2V7PN5JS4QHKMZRDJEX7N64FI3NCGN6ZIPC6ZOOB6RT",
] as const;

export const REVIEWER_ADDRESSES = [
  "GCYSB234UPIF5NW5AJIFKK4YFU3NYOJXSXQOVKK6MEBUQJYQ64L5437J",
  "GCE2ES5ID4BQ7OWRGZQERA7EL6Z5T3SEGNEMDU4CUSIJXRAM2KKL66Z6",
  "GAPA6HRIY5ZI3MOILOLIAMADMJEJWYDCPNI4HGIG4GC4ZP2J3FWTNWPC",
  "GAXUDVAGZZQ3MWI77B4CELNJVJTRTCD6FKZLGHXONAUZREZLQHILFSND",
  "GDL36LDJYGHCTW7CFVVI4XKPWAWEW2TC4MSDAC362PVFFW3Y7LS7KVPK",
  "GCYVVNNP7GRX2DDH6FOCYO5R7AY7TMGNXX6LZB7I4L35JU556PRRWI35",
] as const;

// ── Well-known token keys (resolve through `getTokenMetadata`) ────────────────

export const XLM_TOKEN = "native";
export const USDC_TOKEN = "USDC";

// ── Unit helpers ──────────────────────────────────────────────────────────────

/** Whole XLM → stroops (7 decimals). */
const xlm = (whole: number): bigint => BigInt(Math.round(whole)) * 10000000n;

/** Whole USDC → base units (6 decimals). */
const usdc = (whole: number): bigint => BigInt(Math.round(whole)) * 1000000n;

/** ISO-8601 date → unix seconds (the unit `Grant.deadline` uses). */
const unix = (iso: string): bigint => BigInt(Math.floor(Date.parse(iso) / 1000));

const DAY = 86400n;

// ── Seed data ─────────────────────────────────────────────────────────────────

type MilestoneState = "pending" | "submitted" | "approved" | "paid";

interface MilestoneSeed {
  title: string;
  description: string;
  state: MilestoneState;
}

interface GrantSeed {
  id: string;
  owner: string;
  recipient: string;
  title: string;
  description: string;
  budget: bigint;
  /** Percentage of `budget` deposited so far (0–100). */
  fundedPct: number;
  deadline: string;
  /** Numeric `Grant.status` — see `web/types/index.ts`. */
  status: number;
  reviewers: readonly string[];
  createdAt: string;
  token: string;
  milestones: MilestoneSeed[];
}

const REVIEWERS_3 = REVIEWER_ADDRESSES.slice(0, 3);
const REVIEWERS_4 = REVIEWER_ADDRESSES.slice(0, 4);

const GRANT_SEEDS: GrantSeed[] = [
  {
    id: "g1",
    owner: OWNER_ADDRESSES[0],
    recipient: RECIPIENT_ADDRESSES[0],
    title: "Stellar Asset Sandbox",
    description:
      "A secure environment for testing new Stellar assets with pre-configured liquidity pools.",
    budget: xlm(50000),
    fundedPct: 42,
    deadline: "2026-12-31T23:59:59Z",
    status: 0, // Pending
    reviewers: REVIEWERS_3,
    createdAt: "2026-01-15T09:00:00Z",
    token: XLM_TOKEN,
    milestones: [
      { title: "Architecture Design", description: "Define system components and security model", state: "pending" },
      { title: "Core Engine", description: "Implement the sandbox core", state: "pending" },
      { title: "User Dashboard", description: "Frontend for managing assets", state: "pending" },
    ],
  },
  {
    id: "g2",
    owner: OWNER_ADDRESSES[1],
    recipient: RECIPIENT_ADDRESSES[1],
    title: "Soroban DeFi Aggregator",
    description: "Yield optimizer for Soroban-based AMMs and lending protocols.",
    budget: usdc(75000),
    fundedPct: 68,
    deadline: "2026-10-15T23:59:59Z",
    status: 1, // Active
    reviewers: REVIEWERS_4,
    createdAt: "2026-02-20T09:00:00Z",
    token: USDC_TOKEN,
    milestones: [
      { title: "Protocol Integration", description: "Connect to major Soroban protocols", state: "approved" },
      { title: "Strategy Implementation", description: "Yield farming algorithms", state: "submitted" },
      { title: "Audit and Security", description: "Third-party audit", state: "pending" },
    ],
  },
  {
    id: "g3",
    owner: OWNER_ADDRESSES[2],
    recipient: RECIPIENT_ADDRESSES[2],
    title: "NFT Marketplace for Artists",
    description: "A low-fee marketplace specifically designed for digital artists on Stellar.",
    budget: xlm(30000),
    fundedPct: 100,
    deadline: "2026-05-01T23:59:59Z",
    status: 3, // Complete
    reviewers: REVIEWERS_3,
    createdAt: "2025-11-10T09:00:00Z",
    token: XLM_TOKEN,
    milestones: [
      { title: "Smart Contract Deployment", description: "Deploy basic NFT contracts", state: "paid" },
      { title: "Marketplace UI", description: "Responsive web interface", state: "paid" },
      { title: "Launch and Marketing", description: "Onboard first 50 artists", state: "paid" },
    ],
  },
  {
    id: "g4",
    owner: OWNER_ADDRESSES[3],
    recipient: RECIPIENT_ADDRESSES[3],
    title: "Cross-border Payment Rails",
    description: "Simplified API for micro-payments between LATAM and Southeast Asia.",
    budget: usdc(120000),
    fundedPct: 33,
    deadline: "2027-01-20T23:59:59Z",
    status: 2, // In Progress (disputed milestones under review)
    reviewers: REVIEWERS_4,
    createdAt: "2026-03-05T09:00:00Z",
    token: USDC_TOKEN,
    milestones: [
      { title: "Regulatory Compliance", description: "Obtain necessary licenses", state: "approved" },
      { title: "Banking Integrations", description: "Connect to local rails", state: "submitted" },
      { title: "Beta Trial", description: "First 1000 transactions", state: "pending" },
    ],
  },
  {
    id: "g5",
    owner: OWNER_ADDRESSES[4],
    recipient: RECIPIENT_ADDRESSES[4],
    title: "Eco-friendly Gaming Platform",
    description: "Stellar-powered games where rewards are tied to real-world carbon offsetting.",
    budget: xlm(45000),
    fundedPct: 12,
    deadline: "2026-08-30T23:59:59Z",
    status: 4, // Cancelled
    reviewers: REVIEWERS_3,
    createdAt: "2026-04-12T09:00:00Z",
    token: XLM_TOKEN,
    milestones: [
      { title: "Game Engine Prototype", description: "Basic gameplay loop", state: "submitted" },
      { title: "Stellar Integration", description: "Off-setting oracle connection", state: "pending" },
      { title: "Content Creation", description: "Levels and assets", state: "pending" },
    ],
  },
];

// 42 more grants so pagination has 47 total (4 full pages of 12).
for (let i = 6; i <= 47; i++) {
  const isActive = i % 2 === 0;
  const token = i % 3 === 0 ? USDC_TOKEN : XLM_TOKEN;
  GRANT_SEEDS.push({
    id: `g${i}`,
    owner: OWNER_ADDRESSES[i % OWNER_ADDRESSES.length],
    recipient: RECIPIENT_ADDRESSES[i % RECIPIENT_ADDRESSES.length],
    title: `Community Grant #${i}`,
    description: `A community-driven initiative for Stellar expansion project #${i}.`,
    budget: token === USDC_TOKEN ? usdc(1000 * i) : xlm(1000 * i),
    fundedPct: isActive ? 50 + (i % 5) * 10 : 20,
    deadline: "2027-12-31T23:59:59Z",
    status: isActive ? 1 : 0,
    reviewers: REVIEWERS_3,
    createdAt: `2026-05-0${(i % 9) + 1}T09:00:00Z`,
    token,
    milestones: [
      { title: "Phase 1", description: "Initial delivery against the grant brief", state: "pending" },
      { title: "Phase 2", description: "Final delivery and hand-off", state: "pending" },
    ],
  });
}

// ── Builders ──────────────────────────────────────────────────────────────────

function toMilestone(seed: MilestoneSeed, idx: number, grant: GrantSeed): Milestone {
  const createdAt = unix(grant.createdAt);
  const deadline = unix(grant.deadline);
  const count = grant.milestones.length;
  // Split the budget evenly; the last milestone absorbs the rounding remainder
  // so the payouts always add up to exactly `grant.budget`.
  const base = grant.budget / BigInt(count);
  const amount = idx === count - 1 ? grant.budget - base * BigInt(count - 1) : base;

  const submitted = seed.state !== "pending";
  const approved = seed.state === "approved" || seed.state === "paid";
  const paid = seed.state === "paid";
  const submittedAt = submitted ? createdAt + DAY : null;
  const approvedAt = approved && submittedAt !== null ? submittedAt + DAY : null;

  return {
    idx,
    title: seed.title,
    description: seed.description,
    proof_hash: submitted ? `QmMockProofHash${grant.id}${idx}` : null,
    submitted,
    approved,
    paid,
    submitted_at: submittedAt,
    approved_at: approvedAt,
    paid_at: paid && approvedAt !== null ? approvedAt + DAY : null,
    deadline,
    token: grant.token,
    amount,
  };
}

function buildGrant(seed: GrantSeed): Grant {
  const deadline = unix(seed.deadline);
  const createdAt = unix(seed.createdAt);
  return {
    id: seed.id,
    owner: seed.owner,
    recipient: seed.recipient,
    title: seed.title,
    description: seed.description,
    budget: seed.budget,
    funded: (seed.budget * BigInt(seed.fundedPct)) / 100n,
    deadline,
    status: seed.status,
    milestones: seed.milestones.length,
    reviewers: [...seed.reviewers],
    created_at: createdAt,
    token: seed.token,
  };
}

// ── Exported fixtures ─────────────────────────────────────────────────────────

export const grants: Grant[] = GRANT_SEEDS.map(buildGrant);

/** Full milestone payloads keyed by grant id (the `Grant` type only stores a count). */
export const grantMilestones: Record<string, Milestone[]> = Object.fromEntries(
  GRANT_SEEDS.map((seed) => [
    seed.id,
    seed.milestones.map((milestone, idx) => toMilestone(milestone, idx, seed)),
  ]),
);
