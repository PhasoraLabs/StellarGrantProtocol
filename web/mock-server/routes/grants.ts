import { Router } from "express";
import {
  grants,
  grantMilestones,
  type Grant,
  type Milestone,
} from "../fixtures/grants";

const router = Router();

// ── Query-string ↔ Grant.status (numeric, see web/types/index.ts) ─────────────

/** Query value → numeric `Grant.status` (0 Pending · 1 Active · 2 In Progress · 3 Complete · 4 Cancelled). */
const STATUS_BY_QUERY: Record<string, number> = {
  pending: 0,
  open: 0,
  active: 1,
  review: 2,
  "in progress": 2,
  in_progress: 2,
  disputed: 2,
  completed: 3,
  complete: 3,
  cancelled: 4,
  canceled: 4,
};

/** Numeric `Grant.status` → the backend's status label (used by `GET /grants/:id`). */
const STATUS_LABEL: Record<number, string> = {
  0: "pending",
  1: "active",
  2: "in_progress",
  3: "completed",
  4: "cancelled",
};

/** Query token value → the token key stored on `Grant.token`. */
const TOKEN_BY_QUERY: Record<string, string> = {
  xlm: "native",
  native: "native",
  usdc: "usdc",
};

const SORTERS: Record<string, (a: Grant, b: Grant) => number> = {
  newest: (a, b) => Number(b.created_at - a.created_at),
  funded: (a, b) => (b.funded === a.funded ? 0 : b.funded > a.funded ? 1 : -1),
  deadline: (a, b) => Number(a.deadline - b.deadline),
};

// ── JSON helpers ──────────────────────────────────────────────────────────────

/** JSON has no `bigint` — serialize the `Grant` amount/timestamp fields as strings. */
function serializeGrant(grant: Grant) {
  return {
    ...grant,
    budget: grant.budget.toString(),
    funded: grant.funded.toString(),
    deadline: grant.deadline.toString(),
    created_at: grant.created_at.toString(),
  };
}

function toIso(timestamp: bigint): string {
  return new Date(Number(timestamp) * 1000).toISOString();
}

/**
 * Adapt a fixture to the indexing-API shape `fetchGrantById`
 * (`lib/grants/api.ts`) expects from `GET /grants/:id`, mirroring the real
 * backend's `{ data: <grant> }` envelope.
 */
function toApiGrantDetail(grant: Grant, milestones: Milestone[]) {
  const submitted = milestones.filter((m) => m.submitted).length;
  const overdue = milestones.filter(
    (m) => !m.paid && m.deadline != null && m.deadline * 1000n < BigInt(Date.now()),
  ).length;

  return {
    id: grant.id,
    title: grant.title,
    description: grant.description,
    status: STATUS_LABEL[grant.status] ?? "pending",
    recipient: grant.recipient,
    totalAmount: grant.budget.toString(),
    updatedAt: toIso(grant.created_at),
    isWatched: false,
    milestones: milestones.map((m) => ({
      idx: m.idx,
      title: m.title,
      description: m.description,
      deadline: toIso(m.deadline ?? grant.deadline),
      proof_hash: m.proof_hash,
      submitted: m.submitted,
      approved: m.approved,
      paid: m.paid,
      submitted_at: m.submitted_at !== null ? toIso(m.submitted_at) : null,
      approved_at: m.approved_at !== null ? toIso(m.approved_at) : null,
      paid_at: m.paid_at !== null ? toIso(m.paid_at) : null,
      overdue: !m.paid && m.deadline != null && m.deadline * 1000n < BigInt(Date.now()),
      token: m.token,
      amount: m.amount?.toString(),
    })),
    milestoneSummary: {
      total: milestones.length,
      submitted,
      overdue,
    },
  };
}

// ── Routes ────────────────────────────────────────────────────────────────────

/**
 * GET /grants — paginated + filtered list.
 *
 * Responds with the `GrantPage` envelope `useGrants.ts` consumes:
 * `{ grants, total, nextPage }`.
 */
router.get("/", (req, res) => {
  const page = Math.max(parseInt(req.query.page as string, 10) || 1, 1);
  const limit = Math.max(parseInt(req.query.limit as string, 10) || 12, 1);
  const status = req.query.status as string | undefined;
  const token = req.query.token as string | undefined;
  const sort = req.query.sort as string | undefined;
  const q = (req.query.q as string | undefined)?.toLowerCase();

  let filteredGrants = [...grants];

  if (status) {
    const wanted = status
      .split(",")
      .map((s) => STATUS_BY_QUERY[s.trim().toLowerCase()])
      .filter((s): s is number => s !== undefined);
    filteredGrants = filteredGrants.filter((g) => wanted.includes(g.status));
  }

  if (token && token.toLowerCase() !== "all") {
    const wantedToken = TOKEN_BY_QUERY[token.toLowerCase()] ?? token.toLowerCase();
    filteredGrants = filteredGrants.filter((g) => g.token?.toLowerCase() === wantedToken);
  }

  if (q) {
    filteredGrants = filteredGrants.filter(
      (g) =>
        g.title.toLowerCase().includes(q) || g.description.toLowerCase().includes(q),
    );
  }

  if (sort && SORTERS[sort]) {
    filteredGrants.sort(SORTERS[sort]);
  }

  const startIndex = (page - 1) * limit;
  const paginatedGrants = filteredGrants.slice(startIndex, startIndex + limit);

  res.json({
    grants: paginatedGrants.map(serializeGrant),
    total: filteredGrants.length,
    nextPage: startIndex + limit < filteredGrants.length ? page + 1 : null,
  });
});

// GET /grants/:id - Single grant, in the indexing-API envelope the Next.js
// detail route (`app/api/grants/[id]/route.ts`) forwards to the client.
router.get("/:id", (req, res) => {
  const grant = grants.find((g) => g.id === req.params.id);
  if (!grant) {
    return res.status(404).json({ error: "Grant not found" });
  }
  res.json({ data: toApiGrantDetail(grant, grantMilestones[grant.id] ?? []) });
});

// GET /grants/:id/funders - Mock funders
router.get("/:id/funders", (req, res) => {
  const funders = [
    { address: "GC4IY7WBM355X4NQEDO3KGU3IG6NQNCYZJZHHYTFDSRDC3ZCYCOTJBD6", amount: "10000000000", token: "native" },
    { address: "GDNORLTEMJI5NSRWWWEVGVR3YDMWDSHRLVYOZR7HFB7T4MXQ2UYE5YVI", amount: "5000000000", token: "native" },
    { address: "GAEUOB4W3CTY7ORZKPGTEIEGILUYPOD5PKCAUAAGWSYFHLPTTU5LU37I", amount: "2500000000", token: "native" },
  ];
  res.json({ data: funders });
});

export default router;
