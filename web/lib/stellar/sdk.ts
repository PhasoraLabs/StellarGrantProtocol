/**
 * StellarGrantsSDK
 *
 * High-level SDK class that wraps all contract interactions and search/filter
 * utilities, exposing a single coherent API for frontend consumers.
 *
 * Issue #253 adds:
 *   - grantListAll()          → paginated grant ID listing
 *   - grantFilterByCategory() → filter grants by category keyword
 *   - grantSearchByTitle()    → fuzzy title search
 *   - sortGrants()            → sort by date / budget / milestone progress
 *   - queryGrants()           → compound search + filter + sort helper
 */

import type { Grant, Milestone } from "@/types";
import {
  grantListAll,
  grantFilterByCategory,
  grantSearchByTitle,
  sortGrants,
  queryGrants,
} from "./search";
import type {
  PaginationOptions,
  GrantListResult,
  SortOptions,
  QueryGrantsOptions,
} from "./search";

export class StellarGrantsSDK {
  // ── Internal grant cache ────────────────────────────────────────────────
  //
  // In a production integration this would be populated by a call to the
  // contract's grant_count / grant_get methods (or an indexer REST endpoint).
  // For now callers inject the list via `hydrate()` so the class remains
  // testable without a live RPC node.
  //
  private _grants: Grant[] = [];
  private _milestones: Map<string, Milestone[]> = new Map();

  // ── Data hydration ──────────────────────────────────────────────────────

  /**
   * Load (or refresh) the in-memory grant list.
   * Call this after fetching grants from the chain / indexer.
   */
  hydrate(grants: Grant[], milestones?: Map<string, Milestone[]>): void {
    this._grants = grants;
    if (milestones) this._milestones = milestones;
  }

  // ── Issue #253 — Search & Filter utilities ──────────────────────────────

  /**
   * Return a paginated list of grant IDs from the current in-memory cache.
   *
   * If your contract exposes a `grant_count` / `grant_get` iterator this is
   * the right place to call it and populate the cache on-demand.
   *
   * @example
   * ```ts
   * const sdk = new StellarGrantsSDK();
   * sdk.hydrate(myGrants);
   * const { ids, total, hasMore } = sdk.grantListAll({ page: 1, pageSize: 10 });
   * ```
   */
  grantListAll(options?: PaginationOptions): GrantListResult {
    return grantListAll(this._grants, options);
  }

  /**
   * Filter the cached grants by a category keyword.
   * Matches against both the title and description (case-insensitive).
   *
   * @example
   * ```ts
   * const defi = sdk.grantFilterByCategory("DeFi");
   * ```
   */
  grantFilterByCategory(category: string): Grant[] {
    return grantFilterByCategory(this._grants, category);
  }

  /**
   * Search cached grants by title using multi-word AND matching.
   *
   * @example
   * ```ts
   * const results = sdk.grantSearchByTitle("open source tooling");
   * ```
   */
  grantSearchByTitle(query: string): Grant[] {
    return grantSearchByTitle(this._grants, query);
  }

  /**
   * Sort the cached grants by the specified field and direction.
   *
   * @example
   * ```ts
   * const newest = sdk.sortGrants({ by: "createdAt", direction: "desc" });
   * const cheapest = sdk.sortGrants({ by: "budget", direction: "asc" });
   * ```
   */
  sortGrants(options?: SortOptions): Grant[] {
    return sortGrants(this._grants, options, this._milestones);
  }

  /**
   * Compound helper: search + filter + sort in one call.
   *
   * @example
   * ```ts
   * const results = sdk.queryGrants({
   *   titleQuery: "liquidity",
   *   category:   "DeFi",
   *   sort: { by: "budget", direction: "desc" },
   * });
   * ```
   */
  queryGrants(options?: QueryGrantsOptions): Grant[] {
    return queryGrants(this._grants, {
      ...options,
      milestones: options?.milestones ?? this._milestones,
    });
  }
}

// Singleton instance for convenience
export const stellarGrantsSDK = new StellarGrantsSDK();

// Re-export pure helpers so callers can use them without the class
export {
  grantListAll,
  grantFilterByCategory,
  grantSearchByTitle,
  sortGrants,
  queryGrants,
};
export type {
  PaginationOptions,
  GrantListResult,
  SortOptions,
  SortDirection,
  GrantSortField,
  QueryGrantsOptions,
  GrantWithMilestones,
} from "./search";
