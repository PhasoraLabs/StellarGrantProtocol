/**
 * Issue #1287 — mock-server grant fixtures must share the real data model.
 *
 * Covers:
 *   fixtures/grants.ts   — Grant/Milestone fixtures match web/types/index.ts
 *   routes/grants.ts     — list responds with the GrantPage envelope
 *                          ({ grants, total, nextPage }) that useGrants reads
 *   hooks/useGrants.ts   — the mock payload parses without throwing and the
 *                          serialized bigint fields are revived
 *   lib/grants/api.ts    — GET /grants/:id still feeds the production
 *                          detail mapper (mapApiGrantDetail)
 */

import { describe, it, expect } from "vitest";
import { StrKey } from "@stellar/stellar-sdk";

import { grants, grantMilestones } from "@/mock-server/fixtures/grants";
import grantsRouter from "@/mock-server/routes/grants";
import { normalizeGrantPage } from "@/hooks/useGrants";
import { mapApiGrantDetail } from "@/lib/grants/api";
import type { Grant, Milestone } from "@/types";

// ── Express router test harness ───────────────────────────────────────────────

interface TestResponse {
  statusCode: number;
  body?: unknown;
}

interface RouteHandle {
  handle: (req: unknown, res: unknown) => void;
}
interface RouteLayer {
  route?: { path: string; stack: RouteHandle[] };
}
interface RouterInternals {
  stack: RouteLayer[];
}

/** Invoke an Express route handler directly with a plain req/res pair. */
function invokeGet(
  path: string,
  { query = {}, params = {} }: { query?: Record<string, string>; params?: Record<string, string> } = {},
): { status: number; body: Record<string, unknown> } {
  const stack = (grantsRouter as unknown as RouterInternals).stack;
  const layer = stack.find((l) => l.route?.path === path);
  if (!layer?.route) throw new Error(`Route ${path} not found`);

  const res: TestResponse & {
    status: (code: number) => TestResponse & { json: (body: unknown) => void };
    json: (body: unknown) => void;
  } = {
    statusCode: 200,
    status(code: number) {
      this.statusCode = code;
      return this;
    },
    json(body: unknown) {
      this.body = body;
    },
  };

  layer.route.stack[0].handle({ query, params }, res);
  return { status: res.statusCode, body: res.body as Record<string, unknown> };
}

/** Extract the `grants` array from a list-route body. */
function listGrants(body: Record<string, unknown>): unknown[] {
  return Array.isArray(body.grants) ? body.grants : [];
}

const isValidAddress = (value: unknown): boolean =>
  typeof value === "string" && StrKey.isValidEd25519PublicKey(value);

// ── Fixture shape ─────────────────────────────────────────────────────────────

describe("mock grant fixtures match web/types/index.ts", () => {
  it("every fixture is a real Grant (bigint amounts, numeric status, reviewers)", () => {
    expect(grants.length).toBe(47); // 4 pages of 12 for pagination tests

    for (const grant of grants) {
      expect(typeof grant.id).toBe("string");
      expect(typeof grant.budget).toBe("bigint");
      expect(typeof grant.funded).toBe("bigint");
      expect(typeof grant.deadline).toBe("bigint");
      expect(typeof grant.created_at).toBe("bigint");
      expect(typeof grant.status).toBe("number");
      expect([0, 1, 2, 3, 4]).toContain(grant.status);
      expect(typeof grant.milestones).toBe("number");
      expect(Array.isArray(grant.reviewers)).toBe(true);
      expect(grant.reviewers.length).toBeGreaterThanOrEqual(3);
      expect(isValidAddress(grant.owner)).toBe(true);
      expect(isValidAddress(grant.recipient)).toBe(true);
      for (const reviewer of grant.reviewers) {
        expect(isValidAddress(reviewer)).toBe(true);
      }
      expect(grant.funded).toBeLessThanOrEqual(grant.budget);
      expect(grant.deadline).toBeGreaterThan(grant.created_at);
    }
  });

  it("every fixture milestone matches the Milestone type", () => {
    for (const grant of grants) {
      const milestones: Milestone[] = grantMilestones[grant.id] ?? [];
      expect(milestones).toHaveLength(grant.milestones);

      milestones.forEach((milestone, i) => {
        expect(milestone.idx).toBe(i);
        expect(typeof milestone.submitted).toBe("boolean");
        expect(typeof milestone.approved).toBe("boolean");
        expect(typeof milestone.paid).toBe("boolean");
        expect(typeof milestone.amount).toBe("bigint");
        expect(milestone.submitted_at === null || typeof milestone.submitted_at === "bigint").toBe(true);
        expect(milestone.paid_at === null || typeof milestone.paid_at === "bigint").toBe(true);
        if (milestone.paid) expect(milestone.approved && milestone.submitted).toBe(true);
      });

      // Payouts always add up to exactly the grant budget.
      const total = milestones.reduce((sum, m) => sum + (m.amount ?? 0n), 0n);
      expect(total).toBe(grant.budget);
    }
  });
});

// ── List route → GrantPage envelope ───────────────────────────────────────────

describe("GET /grants (mock route)", () => {
  it("responds with the GrantPage envelope useGrants.ts reads", () => {
    const { status, body } = invokeGet("/", { query: { page: "1", limit: "12" } });

    expect(status).toBe(200);
    expect(Array.isArray(body.grants)).toBe(true);
    expect(body.grants).toHaveLength(12);
    expect(body.total).toBe(47);
    expect(body.nextPage).toBe(2);
    // The legacy envelope field is gone — `grants`, not `data`.
    expect(body.data).toBeUndefined();
  });

  it("serializes bigint fields to strings so JSON can carry them", () => {
    const { body } = invokeGet("/", { query: { page: "1" } });
    const grant = listGrants(body)[0] as Grant;

    expect(typeof grant.budget).toBe("string");
    expect(typeof grant.funded).toBe("string");
    expect(typeof grant.deadline).toBe("string");
    expect(typeof grant.created_at).toBe("string");
    expect(typeof grant.status).toBe("number");
  });

  it("returns the last page with nextPage: null", () => {
    const { body } = invokeGet("/", { query: { page: "4", limit: "12" } });
    expect(body.grants).toHaveLength(11); // 47 = 3 × 12 + 11
    expect(body.nextPage).toBeNull();
  });

  it("filters by status query using numeric Grant.status", () => {
    const { body } = invokeGet("/", { query: { status: "active" } });
    expect(listGrants(body).length).toBeGreaterThan(0);
    for (const grant of listGrants(body) as Grant[]) {
      expect(grant.status).toBe(1);
    }
  });

  it("filters by token query (XLM → native)", () => {
    const { body } = invokeGet("/", { query: { token: "XLM" } });
    expect(listGrants(body).length).toBeGreaterThan(0);
    for (const grant of listGrants(body) as Grant[]) {
      expect(grant.token).toBe("native");
    }
  });

  it("filters by free-text query", () => {
    const { body } = invokeGet("/", { query: { q: "soroban" } });
    expect(listGrants(body).length).toBeGreaterThan(0);
    for (const grant of listGrants(body) as Grant[]) {
      expect(`${grant.title} ${grant.description}`.toLowerCase()).toContain("soroban");
    }
  });
});

// ── useGrants consumption (the original crash) ────────────────────────────────

describe("useGrants parsing of the mock payload", () => {
  it("parses the mock list response without throwing", () => {
    const { body } = invokeGet("/", { query: { page: "1" } });

    let page: ReturnType<typeof normalizeGrantPage> | undefined;
    expect(() => {
      page = normalizeGrantPage(body);
    }).not.toThrow();

    expect(page).toBeDefined();
    expect(page!.grants).toHaveLength(12);
    expect(page!.total).toBe(47);
    expect(page!.nextPage).toBe(2);
    expect(page!.grants[0]!.title).toBeTruthy();
  });

  it("revives the serialized bigint fields back to bigint", () => {
    const { body } = invokeGet("/", { query: { page: "1" } });
    const page = normalizeGrantPage(body);
    const grant = page.grants[0] as Grant;

    expect(typeof grant.budget).toBe("bigint");
    expect(typeof grant.funded).toBe("bigint");
    expect(typeof grant.deadline).toBe("bigint");
    expect(typeof grant.created_at).toBe("bigint");
    expect(grant.budget).toBe(grants[0].budget);
    expect(grant.deadline).toBe(grants[0].deadline);
    expect(Array.isArray(grant.reviewers)).toBe(true);
  });

  it("still accepts the legacy { data, total, nextPage } envelope", () => {
    const { body } = invokeGet("/", { query: { page: "4", limit: "12" } });
    const legacy = { data: body.grants, total: body.total, nextPage: body.nextPage };

    const page = normalizeGrantPage(legacy);
    expect(page.grants).toHaveLength(11);
    expect(page.total).toBe(47);
    expect(page.nextPage).toBeNull();
  });

  it("throws a descriptive error instead of crashing on an unknown payload", () => {
    expect(() => normalizeGrantPage({ unexpected: true })).toThrow(/grants/i);
    expect(() => normalizeGrantPage(undefined)).toThrow(/grants/i);
  });
});

// ── Single-grant route → production detail mapper ─────────────────────────────

describe("GET /grants/:id (mock route)", () => {
  it("responds in the indexing-API envelope the Next.js detail route expects", () => {
    const { status, body } = invokeGet("/:id", { params: { id: "g2" } });

    expect(status).toBe(200);
    const detail = body.data as Record<string, unknown>;
    expect(detail.id).toBe("g2");
    expect(detail.status).toBe("active"); // numeric 1 → backend status label
    expect(detail.recipient).toBe(grants[1].recipient);
    expect(detail.totalAmount).toBe(grants[1].budget.toString());
    expect(detail.milestones).toHaveLength(3);
  });

  it("flows through the production mapApiGrantDetail mapper", () => {
    const { body } = invokeGet("/:id", { params: { id: "g3" } });
    const detail = body.data as Parameters<typeof mapApiGrantDetail>[0];

    const payload = mapApiGrantDetail(detail, 1000n);
    expect(payload.grant.id).toBe("g3");
    expect(payload.grant.budget).toBe(grants[2].budget);
    expect(payload.grant.recipient).toBe(grants[2].recipient);
    expect(payload.grant.status).toBe(grants[2].status);
    expect(payload.grant.funded).toBe(1000n);
    expect(payload.milestones).toHaveLength(3);
    expect(payload.completedMilestones).toBe(3); // g3 is Complete
  });

  it("404s for an unknown grant", () => {
    const { status, body } = invokeGet("/:id", { params: { id: "nope" } });
    expect(status).toBe(404);
    expect(body.error).toBe("Grant not found");
  });
});
