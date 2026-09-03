/**
 * Grant Search and Filtering Utilities — Issue #253
 *
 * Provides utility methods on StellarGrantsSDK to:
 *   - List all/paginated grant IDs              → grantListAll()
 *   - Filter grants by category tag             → grantFilterByCategory()
 *   - Full-text search grants by title          → grantSearchByTitle()
 *   - Sort grants by date, budget, or progress  → sortGrants()
 *
 * These functions operate over an in-memory grant list (typically fetched
 * once from the contract or an indexing service) and are intentionally
 * kept pure so they can be unit-tested without a live Stellar RPC node.
 */

import type { Grant, Milestone } from "@/types";

// ── Supporting types ────────────────────────────────────────────────────────

/** A grant enriched with its milestone list (used for progress sorting). */
export interface GrantWithMilestones {
  grant: Grant;
  milestones: Milestone[];
}

/** Pagination options for grantListAll(). */
export interface PaginationOptions {
  /** 1-indexed page number (default: 1). */
  page?: number;
  /** Number of grants per page (default: 20, max: 100). */
  pageSize?: number;
}

/** Result returned by grantListAll(). */
export interface GrantListResult {
  /** Grant IDs on the requested page. */
  ids: string[];
  /** Total number of grants across all pages. */
  total: number;
  /** Current page (1-indexed). */
  page: number;
  /** Size of each page. */
  pageSize: number;
  /** Whether there are more pages after this one. */
  hasMore: boolean;
}

/** Supported sort fields. */
export type GrantSortField = "createdAt" | "budget" | "milestoneProgress";

/** Sort direction. */
export type SortDirection = "asc" | "desc";

/** Options for sortGrants(). */
export interface SortOptions {
  /** Field to sort by (default: "createdAt"). */
  by?: GrantSortField;
  /** Sort direction (default: "desc"). */
  direction?: SortDirection;
}

// ── Pagination helper ───────────────────────────────────────────────────────

/**
 * Return a paginated slice of grant IDs from the full list.
 *
 * @example
 * ```ts
 * const result = grantListAll(allGrants, { page: 2, pageSize: 10 });
 * // result.ids → grant IDs for page 2
 * ```
 */
export function grantListAll(
  grants: Grant[],
  options: PaginationOptions = {}
): GrantListResult {
  const page = Math.max(1, options.page ?? 1);
  const pageSize = Math.min(100, Math.max(1, options.pageSize ?? 20));

  const total = grants.length;
  const start = (page - 1) * pageSize;
  const end = start + pageSize;

  const ids = grants.slice(start, end).map((g) => g.id);

  return {
    ids,
    total,
    page,
    pageSize,
    hasMore: end < total,
  };
}

// ── Category filter ─────────────────────────────────────────────────────────

/**
 * Filter grants whose description contains the given category tag
 * (case-insensitive). The convention used in the contract is to embed
 * category keywords directly in the description field, e.g.:
 *   "Category: DeFi | Build a liquidity protocol..."
 *
 * If no grants match, an empty array is returned — never throws.
 *
 * @param grants   - Full list of grants to search within
 * @param category - Category string to match (e.g. "DeFi", "NFT", "DAO")
 *
 * @example
 * ```ts
 * const defiGrants = grantFilterByCategory(allGrants, "DeFi");
 * ```
 */
export function grantFilterByCategory(
  grants: Grant[],
  category: string
): Grant[] {
  if (!category.trim()) return grants;

  const needle = category.trim().toLowerCase();

  return grants.filter(
    (g) =>
      g.description.toLowerCase().includes(needle) ||
      g.title.toLowerCase().includes(needle)
  );
}

// ── Title search ────────────────────────────────────────────────────────────

/**
 * Search grants by title using a fuzzy substring match (case-insensitive).
 * Every word in the query must appear somewhere in the title for a grant
 * to be included (AND logic), allowing natural multi-word searches like
 * "liquidity protocol" without needing exact ordering.
 *
 * @param grants - Full list of grants to search within
 * @param query  - Search string, e.g. "staking rewards"
 *
 * @example
 * ```ts
 * const results = grantSearchByTitle(allGrants, "open source tooling");
 * ```
 */
export function grantSearchByTitle(grants: Grant[], query: string): Grant[] {
  const trimmed = query.trim();
  if (!trimmed) return grants;

  const words = trimmed.toLowerCase().split(/\s+/);

  return grants.filter((g) => {
    const haystack = g.title.toLowerCase();
    return words.every((word) => haystack.includes(word));
  });
}

// ── Progress helper (internal) ──────────────────────────────────────────────

/**
 * Compute 0–1 milestone completion ratio for a grant.
 * Uses the `milestones` count from the Grant and the number of approved
 * Milestone objects passed in.  Falls back to funded/budget ratio if no
 * milestone data is available.
 */
function milestoneProgress(grant: Grant, milestones: Milestone[]): number {
  if (grant.milestones === 0) return 0;
  if (milestones.length === 0) {
    // Fallback: use funded/budget if we have no milestone detail
    return grant.budget > 0n
      ? Number(grant.funded * 10000n / grant.budget) / 10000
      : 0;
  }
  const approved = milestones.filter((m) => m.approved).length;
  return approved / grant.milestones;
}

// ── Sorting ─────────────────────────────────────────────────────────────────

/**
 * Sort an array of grants (with optional milestone data) by the specified
 * field and direction.  The original array is never mutated.
 *
 * @param grants    - Grants to sort (plain Grant[] or GrantWithMilestones[])
 * @param options   - Sort field and direction
 * @param milestones - Optional map of grantId → Milestone[] for progress sorting
 *
 * @example
 * ```ts
 * // Sort newest first (default)
 * const sorted = sortGrants(grants);
 *
 * // Sort by budget ascending
 * const cheapest = sortGrants(grants, { by: "budget", direction: "asc" });
 *
 * // Sort by milestone progress descending, with milestone detail
 * const byProgress = sortGrants(grants, { by: "milestoneProgress" }, milestoneMap);
 * ```
 */
export function sortGrants(
  grants: Grant[],
  options: SortOptions = {},
  milestones: Map<string, Milestone[]> = new Map()
): Grant[] {
  const { by = "createdAt", direction = "desc" } = options;
  const multiplier = direction === "asc" ? 1 : -1;

  return [...grants].sort((a, b) => {
    let delta = 0;

    switch (by) {
      case "createdAt":
        delta = Number(a.created_at - b.created_at);
        break;

      case "budget":
        delta = Number(a.budget - b.budget);
        break;

      case "milestoneProgress": {
        const pa = milestoneProgress(a, milestones.get(a.id) ?? []);
        const pb = milestoneProgress(b, milestones.get(b.id) ?? []);
        delta = pa - pb;
        break;
      }
    }

    return delta * multiplier;
  });
}

// ── Compound helper ─────────────────────────────────────────────────────────

/**
 * Convenience function that chains search → filter → sort in one call.
 *
 * @example
 * ```ts
 * const results = queryGrants(allGrants, {
 *   titleQuery: "ocean cleanup",
 *   category:   "environment",
 *   sort: { by: "budget", direction: "desc" },
 * });
 * ```
 */
export interface QueryGrantsOptions {
  titleQuery?: string;
  category?: string;
  sort?: SortOptions;
  milestones?: Map<string, Milestone[]>;
}

export function queryGrants(
  grants: Grant[],
  options: QueryGrantsOptions = {}
): Grant[] {
  let result = grants;

  if (options.titleQuery) {
    result = grantSearchByTitle(result, options.titleQuery);
  }
  if (options.category) {
    result = grantFilterByCategory(result, options.category);
  }
  result = sortGrants(result, options.sort, options.milestones);

  return result;
}
