/**
 * Grant Search and Filtering Utilities — Tests
 * Issue #253
 *
 * Covers:
 *   - grantListAll()          pagination edge-cases
 *   - grantFilterByCategory() title + description matching
 *   - grantSearchByTitle()    single-word, multi-word AND, empty query
 *   - sortGrants()            createdAt / budget / milestoneProgress, asc & desc
 *   - queryGrants()           compound pipe
 *   - StellarGrantsSDK        class-level wrapper
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  grantListAll,
  grantFilterByCategory,
  grantSearchByTitle,
  sortGrants,
  queryGrants,
  StellarGrantsSDK,
} from "../lib/stellar/sdk";
import type { Grant, Milestone } from "../types";

// ── Fixtures ────────────────────────────────────────────────────────────────

function makeGrant(overrides: Partial<Grant> & { id: string }): Grant {
  return {
    owner: "GABC",
    title: "Unnamed Grant",
    description: "A generic grant description.",
    budget: 1_000_000n,
    funded: 0n,
    deadline: 9_999_999n,
    status: 1,
    milestones: 3,
    reviewers: [],
    created_at: 1_000_000n,
    ...overrides,
  };
}

function makeMilestone(overrides: Partial<Milestone> & { idx: number }): Milestone {
  return {
    title: `Milestone ${overrides.idx}`,
    description: "A milestone",
    proof_hash: null,
    submitted: false,
    approved: false,
    paid: false,
    submitted_at: null,
    approved_at: null,
    paid_at: null,
    ...overrides,
  };
}

const grants: Grant[] = [
  makeGrant({
    id: "1",
    title: "Ocean Cleanup Initiative",
    description: "Category: Environment | Removing plastic from oceans.",
    budget: 500_000n,
    created_at: 1_000n,
    milestones: 2,
  }),
  makeGrant({
    id: "2",
    title: "DeFi Liquidity Protocol",
    description: "Category: DeFi | Build an AMM on Stellar.",
    budget: 2_000_000n,
    created_at: 3_000n,
    milestones: 4,
  }),
  makeGrant({
    id: "3",
    title: "Open Source Tooling Suite",
    description: "Category: Developer Tools | CLI and SDK improvements.",
    budget: 750_000n,
    created_at: 2_000n,
    milestones: 3,
  }),
  makeGrant({
    id: "4",
    title: "NFT Marketplace v2",
    description: "Category: NFT | Next-gen marketplace for creators.",
    budget: 1_200_000n,
    created_at: 4_000n,
    milestones: 2,
  }),
  makeGrant({
    id: "5",
    title: "DAO Governance Framework",
    description: "Category: DAO | On-chain voting and treasury management.",
    budget: 900_000n,
    created_at: 5_000n,
    milestones: 5,
  }),
];

// Milestone map: grant 2 has 2/4 approved, grant 5 has 5/5 approved
const milestoneMap = new Map<string, Milestone[]>([
  [
    "2",
    [
      makeMilestone({ idx: 0, approved: true }),
      makeMilestone({ idx: 1, approved: true }),
      makeMilestone({ idx: 2 }),
      makeMilestone({ idx: 3 }),
    ],
  ],
  [
    "5",
    [
      makeMilestone({ idx: 0, approved: true }),
      makeMilestone({ idx: 1, approved: true }),
      makeMilestone({ idx: 2, approved: true }),
      makeMilestone({ idx: 3, approved: true }),
      makeMilestone({ idx: 4, approved: true }),
    ],
  ],
]);

// ── grantListAll ────────────────────────────────────────────────────────────

describe("grantListAll", () => {
  it("returns first page with default page size", () => {
    const result = grantListAll(grants);
    expect(result.ids).toHaveLength(5); // all fit in default page of 20
    expect(result.total).toBe(5);
    expect(result.page).toBe(1);
    expect(result.pageSize).toBe(20);
    expect(result.hasMore).toBe(false);
  });

  it("paginates correctly with pageSize=2, page=1", () => {
    const result = grantListAll(grants, { pageSize: 2, page: 1 });
    expect(result.ids).toEqual(["1", "2"]);
    expect(result.hasMore).toBe(true);
    expect(result.total).toBe(5);
  });

  it("paginates correctly with pageSize=2, page=2", () => {
    const result = grantListAll(grants, { pageSize: 2, page: 2 });
    expect(result.ids).toEqual(["3", "4"]);
    expect(result.hasMore).toBe(true);
  });

  it("last page — hasMore is false", () => {
    const result = grantListAll(grants, { pageSize: 2, page: 3 });
    expect(result.ids).toEqual(["5"]);
    expect(result.hasMore).toBe(false);
  });

  it("page beyond range returns empty ids", () => {
    const result = grantListAll(grants, { pageSize: 10, page: 99 });
    expect(result.ids).toHaveLength(0);
    expect(result.hasMore).toBe(false);
  });

  it("clamps pageSize to maximum of 100", () => {
    const big = Array.from({ length: 200 }, (_, i) =>
      makeGrant({ id: String(i) })
    );
    const result = grantListAll(big, { pageSize: 999 });
    expect(result.ids).toHaveLength(100);
    expect(result.pageSize).toBe(100);
  });

  it("clamps page below 1 to page 1", () => {
    const result = grantListAll(grants, { page: -5 });
    expect(result.page).toBe(1);
    expect(result.ids[0]).toBe("1");
  });

  it("empty grant list returns zero total", () => {
    const result = grantListAll([]);
    expect(result.total).toBe(0);
    expect(result.ids).toHaveLength(0);
    expect(result.hasMore).toBe(false);
  });
});

// ── grantFilterByCategory ───────────────────────────────────────────────────

describe("grantFilterByCategory", () => {
  it("filters by category in description (case-insensitive)", () => {
    const result = grantFilterByCategory(grants, "DeFi");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("2");
  });

  it("matches category in title", () => {
    const result = grantFilterByCategory(grants, "DAO");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("5");
  });

  it("is case-insensitive", () => {
    expect(grantFilterByCategory(grants, "ENVIRONMENT")).toHaveLength(1);
    expect(grantFilterByCategory(grants, "environment")).toHaveLength(1);
  });

  it("returns all grants when category is empty string", () => {
    expect(grantFilterByCategory(grants, "")).toHaveLength(grants.length);
  });

  it("returns all grants when category is whitespace only", () => {
    expect(grantFilterByCategory(grants, "   ")).toHaveLength(grants.length);
  });

  it("returns empty array when no match", () => {
    expect(grantFilterByCategory(grants, "Unicorn")).toHaveLength(0);
  });
});

// ── grantSearchByTitle ──────────────────────────────────────────────────────

describe("grantSearchByTitle", () => {
  it("matches a single word", () => {
    const result = grantSearchByTitle(grants, "Ocean");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("multi-word query uses AND logic", () => {
    const result = grantSearchByTitle(grants, "Open Source");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("3");
  });

  it("does not match when one word is missing", () => {
    const result = grantSearchByTitle(grants, "Open Missing");
    expect(result).toHaveLength(0);
  });

  it("is case-insensitive", () => {
    expect(grantSearchByTitle(grants, "OCEAN CLEANUP")).toHaveLength(1);
    expect(grantSearchByTitle(grants, "ocean cleanup")).toHaveLength(1);
  });

  it("returns all grants on empty query", () => {
    expect(grantSearchByTitle(grants, "")).toHaveLength(grants.length);
  });

  it("returns all grants on whitespace-only query", () => {
    expect(grantSearchByTitle(grants, "   ")).toHaveLength(grants.length);
  });

  it("returns empty array when no title matches", () => {
    expect(grantSearchByTitle(grants, "ZZZ NonExistent")).toHaveLength(0);
  });

  it("matches partial word within title", () => {
    // "Liquidity" is in the DeFi grant title
    const result = grantSearchByTitle(grants, "Liquid");
    expect(result[0].id).toBe("2");
  });
});

// ── sortGrants ──────────────────────────────────────────────────────────────

describe("sortGrants", () => {
  it("defaults to createdAt descending (newest first)", () => {
    const sorted = sortGrants(grants);
    const ids = sorted.map((g) => g.id);
    expect(ids).toEqual(["5", "4", "2", "3", "1"]);
  });

  it("createdAt ascending (oldest first)", () => {
    const sorted = sortGrants(grants, { by: "createdAt", direction: "asc" });
    expect(sorted[0].id).toBe("1");
    expect(sorted[sorted.length - 1].id).toBe("5");
  });

  it("budget descending (largest first)", () => {
    const sorted = sortGrants(grants, { by: "budget", direction: "desc" });
    expect(sorted[0].id).toBe("2"); // 2_000_000n
    expect(sorted[sorted.length - 1].id).toBe("1"); // 500_000n
  });

  it("budget ascending (cheapest first)", () => {
    const sorted = sortGrants(grants, { by: "budget", direction: "asc" });
    expect(sorted[0].id).toBe("1"); // 500_000n
    expect(sorted[sorted.length - 1].id).toBe("2"); // 2_000_000n
  });

  it("milestoneProgress descending — grant 5 (5/5) first, grant 2 (2/4) second", () => {
    const sorted = sortGrants(
      grants,
      { by: "milestoneProgress", direction: "desc" },
      milestoneMap
    );
    expect(sorted[0].id).toBe("5"); // 100 %
    expect(sorted[1].id).toBe("2"); // 50 %
  });

  it("milestoneProgress ascending — grants without milestones sort first", () => {
    const sorted = sortGrants(
      grants,
      { by: "milestoneProgress", direction: "asc" },
      milestoneMap
    );
    // Grants 1, 3, 4 have 0 milestone data → 0 % progress
    const topIds = sorted.slice(0, 3).map((g) => g.id).sort();
    expect(topIds).toEqual(["1", "3", "4"]);
  });

  it("does not mutate the original array", () => {
    const original = [...grants];
    sortGrants(grants, { by: "budget", direction: "asc" });
    expect(grants.map((g) => g.id)).toEqual(original.map((g) => g.id));
  });

  it("handles single-element array without error", () => {
    const single = [makeGrant({ id: "solo" })];
    expect(sortGrants(single)).toHaveLength(1);
  });

  it("handles empty array without error", () => {
    expect(sortGrants([])).toHaveLength(0);
  });
});

// ── queryGrants ─────────────────────────────────────────────────────────────

describe("queryGrants", () => {
  it("returns all grants when no options given", () => {
    expect(queryGrants(grants)).toHaveLength(grants.length);
  });

  it("applies titleQuery", () => {
    const result = queryGrants(grants, { titleQuery: "NFT" });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("4");
  });

  it("applies category filter", () => {
    const result = queryGrants(grants, { category: "DAO" });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("5");
  });

  it("applies sort", () => {
    const result = queryGrants(grants, { sort: { by: "budget", direction: "asc" } });
    expect(result[0].id).toBe("1");
  });

  it("chains titleQuery + category (AND) — no results when incompatible", () => {
    // DeFi grant doesn't have "Ocean" in title → expect 0 results
    const result = queryGrants(grants, { titleQuery: "Ocean", category: "DeFi" });
    expect(result).toHaveLength(0);
  });

  it("chains titleQuery + category (AND) — matching result", () => {
    // Grant 3 has "Tooling" in title and "Developer Tools" in description
    const result = queryGrants(grants, {
      titleQuery: "Tooling",
      category: "Developer",
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("3");
  });

  it("chains all three options", () => {
    const result = queryGrants(grants, {
      category: "DeFi",
      titleQuery: "Liquidity",
      sort: { by: "createdAt", direction: "desc" },
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("2");
  });
});

// ── StellarGrantsSDK class ──────────────────────────────────────────────────

describe("StellarGrantsSDK", () => {
  let sdk: StellarGrantsSDK;

  beforeEach(() => {
    sdk = new StellarGrantsSDK();
    sdk.hydrate(grants, milestoneMap);
  });

  it("grantListAll() returns paginated IDs", () => {
    const { ids, total } = sdk.grantListAll({ page: 1, pageSize: 3 });
    expect(ids).toHaveLength(3);
    expect(total).toBe(5);
  });

  it("grantFilterByCategory() filters correctly", () => {
    const result = sdk.grantFilterByCategory("NFT");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("4");
  });

  it("grantSearchByTitle() searches correctly", () => {
    const result = sdk.grantSearchByTitle("Governance");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("5");
  });

  it("sortGrants() sorts by budget desc", () => {
    const sorted = sdk.sortGrants({ by: "budget", direction: "desc" });
    expect(sorted[0].id).toBe("2");
  });

  it("sortGrants() uses internally hydrated milestone map for progress sort", () => {
    const sorted = sdk.sortGrants({ by: "milestoneProgress", direction: "desc" });
    expect(sorted[0].id).toBe("5"); // 100 %
  });

  it("queryGrants() compound pipeline works", () => {
    const result = sdk.queryGrants({
      titleQuery: "ocean",
      sort: { by: "createdAt", direction: "desc" },
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("hydrate() with no milestone map keeps existing empty map", () => {
    const fresh = new StellarGrantsSDK();
    fresh.hydrate(grants); // no milestones
    // Should not throw; milestone-based sort just uses 0 % for all
    expect(() =>
      fresh.sortGrants({ by: "milestoneProgress" })
    ).not.toThrow();
  });
});
