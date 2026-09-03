"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Search, X } from "lucide-react";
import { GrantCard } from "@/components/grants/GrantCard";
import { ErrorCard, PageHeader, Pagination } from "@/components/ui";
import { useGrants } from "@/hooks/useGrants";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { GRANTS_PER_PAGE } from "@/lib/constants";

// ── Filter helpers ────────────────────────────────────────────────────────

const STATUS_OPTIONS = [
  { label: "All", value: "" },
  { label: "Pending", value: "open" },
  { label: "Active", value: "active" },
  { label: "In Progress", value: "active" },
  { label: "Completed", value: "completed" },
  { label: "Cancelled", value: "cancelled" },
] as const;

const TOKEN_OPTIONS = [
  { label: "All", value: "all" },
  { label: "XLM", value: "XLM" },
  { label: "USDC", value: "USDC" },
] as const;

const SORT_OPTIONS = [
  { label: "Newest", value: "newest" },
  { label: "Most Funded", value: "funded" },
  { label: "Deadline Soon", value: "deadline" },
] as const;

// ── Satellite SVG illustration for empty states ───────────────────────────

function SatelliteIllustration() {
  return (
    <svg
      aria-hidden="true"
      className="mx-auto mb-4 opacity-30"
      width="64"
      height="64"
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect x="28" y="20" width="8" height="24" rx="1" fill="currentColor" />
      <rect x="12" y="28" width="16" height="8" rx="1" fill="currentColor" />
      <rect x="36" y="28" width="16" height="8" rx="1" fill="currentColor" />
      <circle cx="32" cy="32" r="5" fill="currentColor" opacity="0.6" />
    </svg>
  );
}

// ── Page component ────────────────────────────────────────────────────────

export default function GrantsPage() {
  const router = useRouter();
  const searchParams = useSearchParams();

  // Read URL params
  const statusParam = searchParams.get("status") ?? "";
  const tokenParam = searchParams.get("token") ?? "all";
  const sortParam = searchParams.get("sort") ?? "newest";
  const pageParam = Number(searchParams.get("page") ?? "1");
  const qParam = searchParams.get("q") ?? "";

  // Local debounce buffer for search input
  const [searchInput, setSearchInput] = useState(qParam);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [focusedGrantIndex, setFocusedGrantIndex] = useState(-1);
  const gridRef = useRef<HTMLDivElement>(null);

  // Commit a URL param update; resets page to 1 unless we're explicitly
  // changing the page.
  const setParam = useCallback(
    (key: string, value: string) => {
      const params = new URLSearchParams(searchParams.toString());
      if (value) {
        params.set(key, value);
      } else {
        params.delete(key);
      }
      if (key !== "page") params.delete("page");
      router.push(`/grants?${params.toString()}`);
    },
    [router, searchParams]
  );

  // Debounce search → URL update
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      setParam("q", searchInput);
    }, 300);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [searchInput, setParam]);

  // Keep local input in sync if URL changes externally
  useEffect(() => {
    setSearchInput(qParam);
  }, [qParam]);

  // Fetch grants via hook
  const { data, isLoading, error, errorType, refetch } = useGrants({
    status: statusParam as "open" | "active" | "completed" | "cancelled" | undefined,
    token: tokenParam as "XLM" | "USDC" | "all",
    sort: sortParam as "newest" | "funded" | "deadline",
    page: pageParam,
    q: qParam,
  });

  const grants = data?.grants ?? [];
  const total = data?.total ?? 0;
  const totalPages = Math.ceil(total / GRANTS_PER_PAGE);
  const hasFilters = !!(statusParam || (tokenParam && tokenParam !== "all") || qParam);

  const handlePageChange = (p: number) => {
    setParam("page", String(p));
    gridRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  useKeyboardShortcuts([
    {
      key: "f",
      description: "Focus search",
      action: (e) => {
        e?.preventDefault();
        (document.querySelector('input[type="search"]') as HTMLInputElement | null)?.focus();
      },
    },
    {
      key: "j",
      description: "Next grant",
      condition: () => grants.length > 0,
      action: (e) => {
        e?.preventDefault();
        setFocusedGrantIndex((prev) => Math.min(prev + 1, grants.length - 1));
      },
    },
    {
      key: "k",
      description: "Previous grant",
      condition: () => grants.length > 0,
      action: (e) => {
        e?.preventDefault();
        setFocusedGrantIndex((prev) => Math.max(prev - 1, 0));
      },
    },
    {
      key: "Enter",
      description: "Open grant",
      condition: () => focusedGrantIndex >= 0 && focusedGrantIndex < grants.length,
      action: (e) => {
        e?.preventDefault();
        router.push(`/grants/${grants[focusedGrantIndex].id}`);
      },
    },
  ]);

  return (
    <div className="container mx-auto px-4 py-8">
      <PageHeader
        eyebrow="Live Delivery Board"
        title="Grants"
        description="Track open grants, see which milestone tracks are slipping, and jump straight into the creator work queue."
      />

      {/* ── Filter bar ─────────────────────────────────────────────────── */}
      <div className="mb-6 space-y-4">
        {/* Search input */}
        <div className="relative max-w-md">
          <Search
            size={16}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted pointer-events-none"
          />
          <input
            type="search"
            value={searchInput}
            onChange={(e) => {
              setSearchInput(e.target.value);
              setFocusedGrantIndex(-1);
            }}
            placeholder="Search grants…"
            className="w-full bg-surface border border-border-color text-text-primary placeholder:text-text-muted rounded-none pl-9 pr-8 py-2 text-sm focus:outline-none focus:border-accent-secondary"
          />
          {searchInput && (
            <button
              type="button"
              onClick={() => setSearchInput("")}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-text-muted hover:text-text-primary"
              aria-label="Clear search"
            >
              <X size={14} />
            </button>
          )}
        </div>

        {/* Status filter pills */}
        <div className="flex flex-wrap gap-2 items-center">
          {isLoading ? (
            <>
              {Array.from({ length: 6 }).map((_, i) => (
                <div key={i} className="shimmer rounded-none h-8 w-20" />
              ))}
            </>
          ) : (
            STATUS_OPTIONS.map((opt) => {
              const isActive = statusParam === opt.value;
              return (
                <button
                  key={opt.label}
                  type="button"
                  onClick={() => setParam("status", opt.value)}
                  className={[
                    "px-3 py-1 text-sm font-mono rounded-none transition-colors",
                    isActive
                      ? "bg-accent-primary text-bg-primary"
                      : "border border-border-color text-text-muted hover:border-accent-secondary",
                  ].join(" ")}
                >
                  {opt.label}
                </button>
              );
            })
          )}

          {/* Divider */}
          <span className="mx-2 text-border-color">|</span>

          {/* Token filter pills */}
          {TOKEN_OPTIONS.map((opt) => {
            const isActive = tokenParam === opt.value;
            return (
              <button
                key={opt.value}
                type="button"
                onClick={() => setParam("token", opt.value)}
                className={[
                  "px-3 py-1 text-sm font-mono rounded-none transition-colors",
                  isActive
                    ? "bg-accent-primary text-bg-primary"
                    : "border border-border-color text-text-muted hover:border-accent-secondary",
                ].join(" ")}
              >
                {opt.label}
              </button>
            );
          })}

          {/* Sort selector */}
          <select
            value={sortParam}
            onChange={(e) => setParam("sort", e.target.value)}
            className="ml-2 bg-surface border border-border-color text-text-primary rounded-none px-2 py-1 text-sm focus:outline-none focus:border-accent-secondary"
          >
            {SORT_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* ── Results count ───────────────────────────────────────────────── */}
      {!isLoading && !error && (
        <p className="text-text-muted font-mono text-sm mb-4">
          Showing {grants.length} of {total} grants
        </p>
      )}

      {/* ── Loading skeletons ────────────────────────────────────────────── */}
      {isLoading && (
        <div className="grid gap-4 md:grid-cols-2" ref={gridRef}>
          {Array.from({ length: 4 }).map((_, index) => (
            <div key={index} className="shimmer h-52 rounded-none" />
          ))}
        </div>
      )}

      {/* ── Error state ──────────────────────────────────────────────────── */}
      {error && (
        <ErrorCard
          type={errorType}
          message={error.message}
          onRetry={() => void refetch()}
        />
      )}

      {/* ── Empty state ──────────────────────────────────────────────────── */}
      {!isLoading && !error && grants.length === 0 && (
        <div className="text-center py-16">
          <SatelliteIllustration />
          {hasFilters ? (
            <>
              <p className="text-text-muted mb-3">No grants match your filters.</p>
              <button
                type="button"
                onClick={() => router.push("/grants")}
                className="text-accent-secondary underline text-sm"
              >
                Clear filters
              </button>
            </>
          ) : (
            <>
              <p className="text-text-muted mb-3">No grants yet.</p>
              <Link href="/grants/create" className="text-accent-secondary underline text-sm">
                Be the first to create one →
              </Link>
            </>
          )}
        </div>
      )}

      {/* ── Grant grid ───────────────────────────────────────────────────── */}
      {!isLoading && !error && grants.length > 0 && (
        <>
          <div className="grid gap-4 md:grid-cols-2" ref={gridRef}>
            {grants.map((grant, index) => (
              <div
                key={grant.id}
                className={`transition-all duration-200 ${
                  focusedGrantIndex === index
                    ? "ring-2 ring-accent-primary ring-offset-2 ring-offset-bg-primary scale-[1.02]"
                    : ""
                }`}
              >
                <GrantCard
                  grant={{
                    id: grant.id,
                    title: grant.title,
                    status: typeof grant.status === "number" ? grant.status : 0,
                    funded: grant.funded ?? 0,
                    budget: grant.budget ?? 0,
                    deadline: grant.deadline ?? 0,
                    token: grant.token,
                    owner: grant.owner,
                  }}
                />
              </div>
            ))}
          </div>

          {/* ── Pagination ─────────────────────────────────────────────── */}
          <Pagination
            page={pageParam}
            totalPages={totalPages}
            onPageChange={handlePageChange}
          />
        </>
      )}
    </div>
  );
}
