"use client";

/**
 * useGrants Hook
 *
 * Fetches a paginated, filterable grant list.
 * Returns grants array, pagination metadata, and loading/error state.
 */

import { useState, useEffect, useCallback } from "react";
import type { Grant } from "@/types";
import { logger } from "@/lib/logger";

interface UseGrantsOptions {
  status?: "open" | "active" | "completed" | "cancelled";
  token?: "XLM" | "USDC" | "all";
  sort?: "newest" | "funded" | "deadline";
  page?: number;
  q?: string;
}

interface GrantPage {
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

export function useGrants(options?: UseGrantsOptions): UseGrantsResult {
  const { status, token, sort, page = 1, q } = options ?? {};

  const [data, setData] = useState<GrantPage | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [errorType, setErrorType] = useState<"network" | "api" | "rpc" | "generic">("generic");
  const [currentPage, setCurrentPage] = useState(page);

  const buildUrl = useCallback((p: number) => {
    const params = new URLSearchParams();
    if (status) params.set("status", status);
    if (token && token !== "all") params.set("token", token);
    if (sort) params.set("sort", sort);
    if (q) params.set("q", q);
    params.set("page", String(p));
    return `/api/grants?${params.toString()}`;
  }, [status, token, sort, q]);

  const fetchPage = useCallback(async (p: number) => {
    setIsLoading(true);
    setError(null);
    hookLogger.debug("Fetching grants page", { page: p, status, sort });

    try {
      const res = await fetch(buildUrl(p));
      if (!res.ok) throw new Error(`Failed to fetch grants: ${res.status}`);
      const json = await res.json() as GrantPage;
      hookLogger.debug("Grants fetched", { count: json.grants.length, total: json.total });
      setData(json);
      setCurrentPage(p);
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      hookLogger.error("Error fetching grants", { error: error.message });
      setError(error);
      
      // Detect error type
      if (error.message.includes("Failed to fetch") || !navigator.onLine) {
        setErrorType("network");
      } else if (error.message.includes("503") || error.message.includes("504")) {
        setErrorType("api");
      } else {
        setErrorType("generic");
      }
    } finally {
      setIsLoading(false);
    }
  }, [buildUrl, status, sort]);

  useEffect(() => {
    // defer to microtask to avoid setState-in-effect warning
    Promise.resolve().then(() => void fetchPage(1));
  }, [fetchPage]);

  const fetchNextPage = useCallback(async () => {
    if (!data?.nextPage) return;
    await fetchPage(data.nextPage);
  }, [data, fetchPage]);

  return {
    data,
    isLoading,
    error,
    errorType,
    fetchNextPage,
    hasNextPage: !!data?.nextPage,
    refetch: () => fetchPage(currentPage),
  };
}
