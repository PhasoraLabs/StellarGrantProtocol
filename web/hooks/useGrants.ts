"use client";

/**
 * useGrants Hook
 *
 * Fetches a paginated, filterable grant list.
 * Backed by TanStack Query for automatic caching and background refetch.
 * Return shape is unchanged for backward compatibility.
 */

import { useQuery } from "@tanstack/react-query";
import type { Grant } from "@/types";
import { logger } from "@/lib/logger";

export interface UseGrantsOptions {
  status?: "open" | "active" | "completed" | "cancelled";
  token?: "XLM" | "USDC" | "all";
  sort?: "newest" | "funded" | "deadline";
  page?: number;
  q?: string;
}

export interface GrantPage {
  grants: Grant[];
  nextPage: number | null;
  total: number;
}

interface UseGrantsResult {
  data: GrantPage | null;
  isLoading: boolean;
  error: Error | null;
  errorType: "network" | "api" | "rpc" | "generic";
  fetchNextPage: () => Promise<void>;
  hasNextPage: boolean;
  refetch: () => Promise<void>;
}

const hookLogger = logger.child("useGrants");

/**
 * `Grant` stores amounts/timestamps as `bigint`, but JSON cannot — responses
 * arrive with those fields as numbers or digit strings. Convert them back so
 * consumers (e.g. `GrantCard`) can do real `bigint` maths and date maths.
 * Anything that is not an integer (e.g. an ISO date string) is left untouched.
 */
function toChainUint(value: unknown): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "number" && Number.isSafeInteger(value)) return BigInt(value);
  if (typeof value === "string" && /^-?\d+$/.test(value.trim())) return BigInt(value.trim());
  return 0n;
}

function reviveGrant(grant: Grant): Grant {
  return {
    ...grant,
    budget: toChainUint(grant.budget),
    funded: toChainUint(grant.funded),
    deadline: toChainUint(grant.deadline),
    created_at: toChainUint(grant.created_at),
    reviewers: Array.isArray(grant.reviewers) ? grant.reviewers : [],
  };
}

/**
 * Parse a grants-list payload into the canonical `GrantPage`.
 *
 * The mock server (`mock-server/routes/grants.ts`) responds with the
 * `GrantPage` envelope; the legacy `{ data, total, nextPage }` and backend
 * `{ data, meta }` shapes are accepted too, so a differently-shaped upstream
 * degrades gracefully instead of throwing on `payload.grants`.
 */
export function normalizeGrantPage(payload: unknown): GrantPage {
  const body = (payload ?? {}) as {
    grants?: unknown;
    data?: unknown;
    total?: unknown;
    nextPage?: unknown;
    meta?: unknown;
  };

  const rawGrants = Array.isArray(body.grants)
    ? body.grants
    : Array.isArray(body.data)
      ? body.data
      : null;

  if (!rawGrants) {
    throw new Error("Unexpected grants response: payload has no `grants` array");
  }

  const meta = (body.meta ?? {}) as { total?: unknown };
  const total =
    typeof body.total === "number"
      ? body.total
      : typeof meta.total === "number"
        ? meta.total
        : rawGrants.length;

  const grants = rawGrants
    .filter((grant): grant is Grant => Boolean(grant) && typeof grant === "object")
    .map(reviveGrant);

  return {
    grants,
    total,
    nextPage: typeof body.nextPage === "number" ? body.nextPage : null,
  };
}

function buildGrantsUrl(options: UseGrantsOptions, page: number): string {
  const params = new URLSearchParams();
  if (options.status) params.set("status", options.status);
  if (options.token && options.token !== "all") params.set("token", options.token);
  if (options.sort) params.set("sort", options.sort);
  if (options.q) params.set("q", options.q);
  params.set("page", String(page));
  return `/api/grants?${params.toString()}`;
}

async function fetchGrantsPage(options: UseGrantsOptions, page: number): Promise<GrantPage> {
  const url = buildGrantsUrl(options, page);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch grants: ${res.status}`);
  const grantPage = normalizeGrantPage(await res.json());
  hookLogger.debug("Grants fetched", {
    count: grantPage.grants.length,
    total: grantPage.total,
  });
  return grantPage;
}

function classifyError(err: Error): "network" | "api" | "rpc" | "generic" {
  if (err.message.includes("Failed to fetch") || (typeof navigator !== "undefined" && !navigator.onLine)) {
    return "network";
  }
  if (err.message.includes("503") || err.message.includes("504")) return "api";
  return "generic";
}

export function useGrants(options?: UseGrantsOptions): UseGrantsResult {
  const { status, token, sort, page = 1, q } = options ?? {};
  const opts: UseGrantsOptions = { status, token, sort, q };

  hookLogger.debug("useGrants", { page, ...opts });

  const { data, isLoading, error, refetch } = useQuery<GrantPage, Error>({
    queryKey: ["grants", opts, page],
    queryFn: () => fetchGrantsPage(opts, page),
    staleTime: 30_000,
    placeholderData: (prev) => prev,
  });

  const fetchNextPage = async () => {
    if (!data?.nextPage) return;
    // Prefetch next page — the caller navigates via URL update which
    // triggers a fresh query; this warms the cache.
    await refetch();
  };

  return {
    data: data ?? null,
    isLoading,
    error: error ?? null,
    errorType: error ? classifyError(error) : "generic",
    fetchNextPage,
    hasNextPage: !!data?.nextPage,
    refetch: async () => {
      await refetch();
    },
  };
}
