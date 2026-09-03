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
  const json = (await res.json()) as GrantPage;
  hookLogger.debug("Grants fetched", { count: json.grants.length, total: json.total });
  return json;
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
