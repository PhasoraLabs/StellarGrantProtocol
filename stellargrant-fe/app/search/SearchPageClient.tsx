"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { GrantCard } from "@/components/grants/GrantCard";
import { SearchInput } from "@/components/ui/SearchInput";
import { WalletAddress } from "@/components/wallet/WalletAddress";
import { queryGrants } from "@/lib/stellar/sdk";
import { apiGet } from "@/lib/api";
import { addressToColor, mapListItemToGrant, type ApiGrantListItem } from "@/lib/search/map";
import type { Grant } from "@/types";
import type { ContributorSearchResult } from "@/app/api/contributors/route";
import type { MilestoneSearchResult } from "@/app/api/milestones/route";

type SearchTab = "all" | "grants" | "contributors" | "milestones";

const GRANT_PREVIEW_LIMIT = 6;

function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(id);
  }, [value, delayMs]);
  return debounced;
}

function ContributorRow({ contributor }: { contributor: ContributorSearchResult }) {
  return (
    <Link
      href={`/contributors/${contributor.address}`}
      className="flex items-center justify-between gap-4 border-b border-border-color/40 px-4 py-3 transition-colors hover:bg-surface/50 last:border-b-0"
    >
      <div className="flex min-w-0 items-center gap-3">
        <div
          className="flex h-8 w-8 shrink-0 items-center justify-center font-mono text-[10px] font-bold text-white"
          style={{ backgroundColor: addressToColor(contributor.address) }}
          aria-hidden="true"
        >
          {contributor.address.slice(0, 2)}
        </div>
        <WalletAddress address={contributor.address} showCopyIcon={false} />
      </div>
      <div className="shrink-0 text-right font-mono text-xs text-text-muted">
        <p>
          Score: <span className="text-text-primary">{contributor.reputation_score}</span>
        </p>
        <p>
          {contributor.grants_completed} grant
          {contributor.grants_completed === 1 ? "" : "s"} completed
        </p>
      </div>
      <span className="hidden shrink-0 font-mono text-xs text-accent-secondary sm:inline">
        View Profile →
      </span>
    </Link>
  );
}

function MilestoneRow({ milestone }: { milestone: MilestoneSearchResult }) {
  const label = milestone.grantTitle
    ? `Phase ${milestone.milestoneIdx + 1}: ${milestone.title} — ${milestone.grantTitle}`
    : milestone.title;

  const href =
    milestone.grantId !== ""
      ? `/grants/${milestone.grantId}/milestones/${milestone.milestoneIdx}`
      : undefined;

  const content = (
    <>
      <p className="min-w-0 flex-1 font-mono text-sm text-text-primary">{label}</p>
      <span className="shrink-0 border border-border-color px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider text-text-muted">
        {milestone.status}
      </span>
      {href && (
        <span className="shrink-0 font-mono text-xs text-accent-secondary">View →</span>
      )}
    </>
  );

  if (!href) {
    return (
      <div className="flex flex-wrap items-center gap-3 border-b border-border-color/40 px-4 py-3 last:border-b-0">
        {content}
      </div>
    );
  }

  return (
    <Link
      href={href}
      className="flex flex-wrap items-center gap-3 border-b border-border-color/40 px-4 py-3 transition-colors hover:bg-surface/50 last:border-b-0"
    >
      {content}
    </Link>
  );
}

export function SearchPageClient() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const qParam = searchParams.get("q") ?? "";
  const typeParam = searchParams.get("type");
  const activeTab: SearchTab =
    typeParam === "grants" ||
    typeParam === "contributors" ||
    typeParam === "milestones"
      ? typeParam
      : "all";

  const [inputValue, setInputValue] = useState(qParam);
  const debouncedQuery = useDebouncedValue(inputValue, 300);

  const [allGrants, setAllGrants] = useState<Grant[]>([]);
  const [contributors, setContributors] = useState<ContributorSearchResult[]>([]);
  const [milestones, setMilestones] = useState<MilestoneSearchResult[]>([]);
  const [recentGrants, setRecentGrants] = useState<Grant[]>([]);
  const [topContributors, setTopContributors] = useState<ContributorSearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setInputValue(qParam);
  }, [qParam]);

  const updateUrl = useCallback(
    (nextQ: string, tab: SearchTab = activeTab) => {
      const params = new URLSearchParams();
      if (nextQ.trim()) params.set("q", nextQ.trim());
      if (tab !== "all") params.set("type", tab);
      const query = params.toString();
      router.replace(query ? `/search?${query}` : "/search");
    },
    [router, activeTab],
  );

  useEffect(() => {
    if (debouncedQuery === qParam) return;
    updateUrl(debouncedQuery);
  }, [debouncedQuery, qParam, updateUrl]);

  const grantResults = useMemo(() => {
    if (!qParam.trim()) return [];
    return queryGrants(allGrants, {
      titleQuery: qParam,
      sort: { by: "createdAt", direction: "desc" },
    });
  }, [allGrants, qParam]);

  const grantPreview = grantResults.slice(0, GRANT_PREVIEW_LIMIT);

  const loadGrantsCatalog = useCallback(async () => {
    const raw = await apiGet<ApiGrantListItem[]>("/grants?limit=100&order=DESC&sortBy=updatedAt");
    return (Array.isArray(raw) ? raw : []).map((item) => mapListItemToGrant(item));
  }, []);

  const runSearch = useCallback(
    async (query: string) => {
      if (!query.trim()) {
        setContributors([]);
        setMilestones([]);
        setError(null);
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        const [contributorRes, milestoneRes] = await Promise.all([
          fetch(`/api/contributors?q=${encodeURIComponent(query)}`),
          fetch(`/api/milestones?q=${encodeURIComponent(query)}`),
        ]);

        const contributorJson = (await contributorRes.json()) as {
          contributors?: ContributorSearchResult[];
        };
        const milestoneJson = (await milestoneRes.json()) as {
          milestones?: MilestoneSearchResult[];
        };

        setContributors(contributorJson.contributors ?? []);
        setMilestones(milestoneJson.milestones ?? []);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Search failed");
        setContributors([]);
        setMilestones([]);
      } finally {
        setIsLoading(false);
      }
    },
    [],
  );

  const loadDiscovery = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [grants, contributorRes] = await Promise.all([
        loadGrantsCatalog(),
        fetch("/api/contributors"),
      ]);
      const contributorJson = (await contributorRes.json()) as {
        contributors?: ContributorSearchResult[];
      };
      setAllGrants(grants);
      setRecentGrants(grants.slice(0, 6));
      setTopContributors(contributorJson.contributors ?? []);
      setContributors([]);
      setMilestones([]);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load discovery content");
    } finally {
      setIsLoading(false);
    }
  }, [loadGrantsCatalog]);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      if (!qParam.trim()) {
        await loadDiscovery();
        return;
      }

      setIsLoading(true);
      setError(null);
      try {
        const grants = await loadGrantsCatalog();
        if (cancelled) return;
        setAllGrants(grants);
        await runSearch(qParam);
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Search failed");
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };

    void load();
    return () => {
      cancelled = true;
    };
  }, [qParam, loadDiscovery, loadGrantsCatalog, runSearch]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setInputValue("");
      updateUrl("");
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [updateUrl]);

  const tabs: { id: SearchTab; label: string; count: number }[] = [
    { id: "all", label: "All", count: grantResults.length + contributors.length + milestones.length },
    { id: "grants", label: "Grants", count: grantResults.length },
    { id: "contributors", label: "Contributors", count: contributors.length },
    { id: "milestones", label: "Milestones", count: milestones.length },
  ];

  const setTab = (tab: SearchTab) => {
    updateUrl(qParam, tab);
  };

  const showGrants = activeTab === "all" || activeTab === "grants";
  const showContributors = activeTab === "all" || activeTab === "contributors";
  const showMilestones = activeTab === "all" || activeTab === "milestones";
  const hasQuery = qParam.trim().length > 0;
  const hasResults =
    grantResults.length > 0 || contributors.length > 0 || milestones.length > 0;

  return (
    <div className="container mx-auto max-w-6xl px-4 py-8">
      <p className="font-mono text-xs uppercase tracking-[0.32em] text-accent-secondary mb-3">
        Search
      </p>

      <SearchInput
        value={inputValue}
        onChange={setInputValue}
        autoFocus
        className="mb-6"
      />

      {hasQuery && (
        <div className="mb-8 flex flex-wrap gap-2">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setTab(tab.id)}
              className={[
                "font-mono text-xs uppercase tracking-wider border px-3 py-1.5 transition-colors",
                activeTab === tab.id
                  ? "border-accent-primary bg-accent-primary/10 text-accent-primary"
                  : "border-border-color text-text-muted hover:border-accent-secondary hover:text-accent-secondary",
              ].join(" ")}
            >
              {tab.label} ({tab.count})
            </button>
          ))}
        </div>
      )}

      {error && (
        <div className="mb-6 rounded-none border border-danger/40 bg-danger/10 p-4 font-mono text-sm text-danger">
          {error}
        </div>
      )}

      {!hasQuery && (
        <div className="space-y-10">
          <section>
            <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-muted mb-4">
              Recent Grants
            </h2>
            {isLoading ? (
              <div className="grid gap-4 md:grid-cols-2">
                {Array.from({ length: 4 }).map((_, i) => (
                  <div key={i} className="shimmer h-40 rounded-none" />
                ))}
              </div>
            ) : (
              <div className="grid gap-4 md:grid-cols-2">
                {recentGrants.map((grant) => (
                  <Link key={grant.id} href={`/grants/${grant.id}`}>
                    <GrantCard grant={{ ...grant, id: Number(grant.id) }} compact />
                  </Link>
                ))}
              </div>
            )}
          </section>

          <section>
            <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-muted mb-4">
              Top Contributors
            </h2>
            <div className="border border-border-color bg-surface ring-1 ring-border-color">
              {topContributors.length === 0 ? (
                <p className="p-6 font-mono text-sm text-text-muted">No contributors yet.</p>
              ) : (
                topContributors.map((contributor) => (
                  <ContributorRow key={contributor.address} contributor={contributor} />
                ))
              )}
            </div>
          </section>
        </div>
      )}

      {hasQuery && isLoading && (
        <div className="space-y-4">
          <div className="shimmer h-32 rounded-none" />
          <div className="shimmer h-32 rounded-none" />
        </div>
      )}

      {hasQuery && !isLoading && !hasResults && (
        <p className="font-mono text-sm text-text-muted">
          No results for &apos;{qParam}&apos;. Try a different keyword or{" "}
          <Link href="/grants" className="text-accent-secondary hover:underline">
            browse all grants
          </Link>
          .
        </p>
      )}

      {hasQuery && !isLoading && hasResults && (
        <div className="space-y-10">
          {showGrants && grantResults.length > 0 && (
            <section>
              <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-muted mb-4">
                Grants
              </h2>
              <div className="grid gap-4 md:grid-cols-2">
                {(activeTab === "all" ? grantPreview : grantResults).map((grant) => (
                  <Link key={grant.id} href={`/grants/${grant.id}`}>
                    <GrantCard grant={{ ...grant, id: Number(grant.id) }} compact />
                  </Link>
                ))}
              </div>
              {activeTab === "all" && grantResults.length > GRANT_PREVIEW_LIMIT && (
                <p className="mt-4">
                  <Link
                    href={`/grants?q=${encodeURIComponent(qParam)}`}
                    className="font-mono text-xs text-accent-secondary hover:underline"
                  >
                    See all {grantResults.length} grants matching &quot;{qParam}&quot; →
                  </Link>
                </p>
              )}
            </section>
          )}

          {showContributors && contributors.length > 0 && (
            <section>
              <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-muted mb-4">
                Contributors
              </h2>
              <div className="border border-border-color bg-surface ring-1 ring-border-color">
                {contributors.map((contributor) => (
                  <ContributorRow key={contributor.address} contributor={contributor} />
                ))}
              </div>
            </section>
          )}

          {showMilestones && milestones.length > 0 && (
            <section>
              <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-muted mb-4">
                Milestones
              </h2>
              <div className="border border-border-color bg-surface ring-1 ring-border-color">
                {milestones.map((milestone) => (
                  <MilestoneRow key={milestone.id} milestone={milestone} />
                ))}
              </div>
            </section>
          )}
        </div>
      )}
    </div>
  );
}
