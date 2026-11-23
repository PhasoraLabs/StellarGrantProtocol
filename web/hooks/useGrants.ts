"use client";

/**
 * useGrants Hook
 *
 * Fetches a paginated, filterable grant list.
 * Backed by TanStack Query for automatic caching and background refetch.
 * Exposes controls synced to URL query params for status filtering and amount/date sorting.
 */

import { useCallback, useMemo, useState, useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import type { Grant } from "@/types";
import { logger } from "@/lib/logger";

export type GrantSort =
  | "newest"
  | "funded"
  | "deadline"
  | "amount"
  | "amount_desc"
  | "amount_asc";

export type GrantStatus = "open" | "active" | "completed" | "cancelled" | "pending";

export interface UseGrantsOptions {
  status?: GrantStatus | "";
  token?: "XLM" | "USDC" | "all";
  sort?: GrantSort;
  page?: number;
  q?: string;
  syncWithUrl?: boolean;
  pathname?: string;
  router?: { push: (url: string) => void };
  searchParams?: URLSearchParams | { get: (key: string) => string | null; toString: () => string };
}

export interface GrantPage {
  grants: Grant[];
  nextPage: number | null;
  total: number;
}

export interface UseGrantsResult {
  data: GrantPage | null;
  isLoading: boolean;
  error: Error | null;
  errorType: "network" | "api" | "rpc" | "generic";
  fetchNextPage: () => Promise<void>;
  hasNextPage: boolean;
  refetch: () => Promise<void>;

  // Controls synced to URL query params
  status: string;
  setStatus: (status: GrantStatus | "") => void;
  sort: GrantSort | undefined;
  setSort: (sort: GrantSort) => void;
  token: "XLM" | "USDC" | "all";
  setToken: (token: "XLM" | "USDC" | "all") => void;
  page: number;
  setPage: (page: number) => void;
  q: string;
  setQ: (q: string) => void;
  setParam: (key: string, value: string) => void;
  resetFilters: () => void;
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

export function buildGrantsUrl(options: UseGrantsOptions, page: number): string {
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

function getWindowSearchParams(): URLSearchParams {
  if (typeof window === "undefined") return new URLSearchParams();
  return new URLSearchParams(window.location.search);
}

export function useGrants(options?: UseGrantsOptions): UseGrantsResult {
  const {
    status: initialStatus,
    token: initialToken,
    sort: initialSort,
    page: initialPage,
    q: initialQ,
    syncWithUrl = false,
    pathname: customPathname,
    router,
    searchParams: customSearchParams,
  } = options ?? {};

  const resolvedPathname = customPathname ?? (typeof window !== "undefined" && window.location.pathname !== "/" ? window.location.pathname : "/grants");

  // Read URL search params when synced or provided
  const searchParams = customSearchParams ?? (syncWithUrl ? getWindowSearchParams() : null);

  const [localStatus, setLocalStatus] = useState<string>(
    initialStatus ?? (searchParams?.get("status") || ""),
  );
  const [localSort, setLocalSort] = useState<GrantSort | undefined>(
    initialSort ?? ((searchParams?.get("sort") as GrantSort) || undefined),
  );
  const [localToken, setLocalToken] = useState<"XLM" | "USDC" | "all">(
    initialToken ?? ((searchParams?.get("token") as "XLM" | "USDC" | "all") || "all"),
  );
  const [localPage, setLocalPage] = useState<number>(
    initialPage ?? (searchParams?.get("page") ? Number(searchParams.get("page")) : 1),
  );
  const [localQ, setLocalQ] = useState<string>(
    initialQ ?? (searchParams?.get("q") || ""),
  );

  // Sync state if options change
  useEffect(() => {
    if (initialStatus !== undefined) setLocalStatus(initialStatus);
  }, [initialStatus]);

  useEffect(() => {
    if (initialSort !== undefined) setLocalSort(initialSort);
  }, [initialSort]);

  useEffect(() => {
    if (initialToken !== undefined) setLocalToken(initialToken);
  }, [initialToken]);

  useEffect(() => {
    if (initialPage !== undefined) setLocalPage(initialPage);
  }, [initialPage]);

  useEffect(() => {
    if (initialQ !== undefined) setLocalQ(initialQ);
  }, [initialQ]);

  // Track initial param string so we only sync on genuine external URL changes
  const prevUrlStringRef = useRef<string>(searchParams ? searchParams.toString() : "");

  useEffect(() => {
    if (!syncWithUrl) return;
    const currentParams = customSearchParams ?? getWindowSearchParams();
    const currentStr = currentParams.toString();
    if (currentStr !== prevUrlStringRef.current) {
      prevUrlStringRef.current = currentStr;
      const urlStatus = currentParams.get("status") || "";
      const urlSort = (currentParams.get("sort") as GrantSort) || undefined;
      const urlToken = (currentParams.get("token") as "XLM" | "USDC" | "all") || "all";
      const urlPage = currentParams.get("page") ? Number(currentParams.get("page")) : 1;
      const urlQ = currentParams.get("q") || "";

      setLocalStatus(urlStatus);
      setLocalSort(urlSort);
      setLocalToken(urlToken);
      setLocalPage(urlPage);
      setLocalQ(urlQ);
    }
  }, [syncWithUrl, customSearchParams]);

  // Commit a URL param update and keep local state synchronized
  const setParam = useCallback(
    (key: string, value: string) => {
      // Update local state immediately
      if (key === "status") setLocalStatus(value);
      if (key === "sort") setLocalSort((value as GrantSort) || undefined);
      if (key === "token") setLocalToken(value as "XLM" | "USDC" | "all");
      if (key === "page") setLocalPage(Number(value) || 1);
      if (key === "q") setLocalQ(value);

      // If updating a filter, reset page to 1
      if (key !== "page") {
        setLocalPage(1);
      }

      // Sync to URL if enabled
      if (syncWithUrl || router || customSearchParams) {
        const nextStatus = key === "status" ? value : localStatus;
        const nextSort = key === "sort" ? (value as GrantSort) || undefined : localSort;
        const nextToken = key === "token" ? (value as "XLM" | "USDC" | "all") : localToken;
        const nextPage = key === "page" ? Number(value) || 1 : 1;
        const nextQ = key === "q" ? value : localQ;

        const params = new URLSearchParams();
        if (nextStatus) params.set("status", nextStatus);
        if (nextToken && nextToken !== "all") params.set("token", nextToken);
        if (nextSort) params.set("sort", nextSort);
        if (nextQ) params.set("q", nextQ);
        if (nextPage > 1) params.set("page", String(nextPage));

        const newUrl = params.toString() ? `${resolvedPathname}?${params.toString()}` : resolvedPathname;

        if (router?.push) {
          router.push(newUrl);
        } else if (typeof window !== "undefined") {
          window.history.pushState({}, "", newUrl);
          window.dispatchEvent(new Event("popstate"));
        }
      }
    },
    [localStatus, localSort, localToken, localQ, resolvedPathname, syncWithUrl, router, customSearchParams],
  );

  const setStatus = useCallback(
    (newStatus: GrantStatus | "") => setParam("status", newStatus),
    [setParam],
  );

  const setSort = useCallback(
    (newSort: GrantSort) => setParam("sort", newSort),
    [setParam],
  );

  const setToken = useCallback(
    (newToken: "XLM" | "USDC" | "all") => setParam("token", newToken),
    [setParam],
  );

  const setPage = useCallback(
    (newPage: number) => setParam("page", String(newPage)),
    [setParam],
  );

  const setQ = useCallback((newQ: string) => setParam("q", newQ), [setParam]);

  const resetFilters = useCallback(() => {
    setLocalStatus("");
    setLocalSort("newest");
    setLocalToken("all");
    setLocalPage(1);
    setLocalQ("");

    if (syncWithUrl || router || customSearchParams) {
      if (router?.push) {
        router.push(resolvedPathname);
      } else if (typeof window !== "undefined") {
        window.history.pushState({}, "", resolvedPathname);
        window.dispatchEvent(new Event("popstate"));
      }
    }
  }, [resolvedPathname, syncWithUrl, router, customSearchParams]);

  const queryOpts = useMemo<UseGrantsOptions>(
    () => ({
      status: (localStatus as GrantStatus) || undefined,
      token: localToken,
      sort: localSort,
      q: localQ || undefined,
    }),
    [localStatus, localToken, localSort, localQ],
  );

  hookLogger.debug("useGrants", { page: localPage, ...queryOpts });

  const { data, isLoading, error, refetch } = useQuery<GrantPage, Error>({
    queryKey: ["grants", queryOpts, localPage],
    queryFn: () => fetchGrantsPage(queryOpts, localPage),
    staleTime: 30_000,
    placeholderData: (prev) => prev,
  });

  const fetchNextPage = async () => {
    if (!data?.nextPage) return;
    setPage(data.nextPage);
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
    status: localStatus,
    setStatus,
    sort: localSort,
    setSort,
    token: localToken,
    setToken,
    page: localPage,
    setPage,
    q: localQ,
    setQ,
    setParam,
    resetFilters,
  };
}
