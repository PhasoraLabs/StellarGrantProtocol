"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { GrantCard } from "@/components/grants/GrantCard";
import { apiGet } from "@/lib/api";
import { EmptyState, ErrorCard, PageHeader, SearchInput } from "@/components/ui";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

/** Raw shape returned by the API */
type GrantListItem = {
  id: number;
  title: string;
  status: string | number;
  totalAmount?: string;
  funded?: bigint | number;
  budget?: bigint | number;
  deadline?: bigint | number;
  token?: string;
  owner?: string;
  hasOverdueMilestones?: boolean;
  milestoneSummary?: {
    total: number;
    submitted: number;
    overdue: number;
    upcoming: number;
    nextDeadline: string | null;
  };
};

/** Shape expected by GrantCard */
type GrantCardInput = {
  id: number;
  title: string;
  status: number;
  funded: bigint | number;
  budget: bigint | number;
  deadline: bigint | number;
  token?: string;
  owner?: string;
};

function normaliseGrant(raw: GrantListItem): GrantCardInput {
  return {
    id: raw.id,
    title: raw.title,
    status: typeof raw.status === "number" ? raw.status : 0,
    funded: raw.funded ?? 0,
    budget: raw.budget ?? 0,
    deadline: raw.deadline ?? 0,
    token: raw.token,
    owner: raw.owner,
  };
}

export default function GrantsPage() {
  const router = useRouter();
  const [grants, setGrants] = useState<GrantCardInput[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [errorType, setErrorType] = useState<"network" | "api" | "rpc" | "generic">("generic");
  const [retryCount, setRetryCount] = useState(0);

  const [filterQuery, setFilterQuery] = useState("");
  const [focusedGrantIndex, setFocusedGrantIndex] = useState(-1);

  const filteredGrants = grants.filter(
    (g) => g.title.toLowerCase().includes(filterQuery.toLowerCase())
  );

  useKeyboardShortcuts([
    {
      key: "f",
      description: "Focus Filters",
      action: (e) => {
        e?.preventDefault();
        const searchInput = document.querySelector('input[type="search"]') as HTMLInputElement | null;
        searchInput?.focus();
      },
    },
    {
      key: "j",
      description: "Next Grant",
      condition: () => filteredGrants.length > 0,
      action: (e) => {
        e?.preventDefault();
        setFocusedGrantIndex((prev) => Math.min(prev + 1, filteredGrants.length - 1));
      },
    },
    {
      key: "k",
      description: "Previous Grant",
      condition: () => filteredGrants.length > 0,
      action: (e) => {
        e?.preventDefault();
        setFocusedGrantIndex((prev) => Math.max(prev - 1, 0));
      },
    },
    {
      key: "Enter",
      description: "Open Grant",
      condition: () => focusedGrantIndex >= 0 && focusedGrantIndex < filteredGrants.length,
      action: (e) => {
        e?.preventDefault();
        const grantId = filteredGrants[focusedGrantIndex].id;
        router.push(`/grants/${grantId}`);
      },
    },
  ]);

  useEffect(() => {
    let cancelled = false;

    const loadGrants = async () => {
      try {
        setLoading(true);
        const raw = await apiGet<GrantListItem[]>("/grants");
        if (!cancelled) {
          setGrants((Array.isArray(raw) ? raw : []).map(normaliseGrant));
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          const error = err instanceof Error ? err : new Error(String(err));
          setError(error.message);
          // @ts-expect-error - type is a custom property on StellarGrantsError
          setErrorType(err.type ?? "generic");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void loadGrants();
    return () => {
      cancelled = true;
    };
  }, [retryCount]);

  return (
    <div className="container mx-auto px-4 py-8">
      <PageHeader
        eyebrow="Live Delivery Board"
        title="Grants"
        description="Track open grants, see which milestone tracks are slipping, and jump straight into the creator work queue."
      />

      <div className="mb-6 max-w-md">
        <SearchInput
          value={filterQuery}
          onChange={(v) => {
            setFilterQuery(v);
            setFocusedGrantIndex(-1); // reset focus on search
          }}
          placeholder="Filter grants by title…"
        />
      </div>

      {loading && (
        <div className="grid gap-4 md:grid-cols-2">
          {Array.from({ length: 4 }).map((_, index) => (
            <div key={index} className="shimmer h-52 rounded-[4px]" />
          ))}
        </div>
      )}

      {error && (
        <ErrorCard
          type={errorType}
          message={error}
          onRetry={() => setRetryCount((c) => c + 1)}
        />
      )}

      {!loading && !error && filteredGrants.length === 0 && grants.length > 0 && (
        <EmptyState
          title="No matches found"
          description={`No grants match "${filterQuery}".`}
          action={{ label: "Clear filter", onClick: () => setFilterQuery("") }}
        />
      )}

      {!loading && !error && filteredGrants.length > 0 && (
        <div className="grid gap-4 md:grid-cols-2">
          {filteredGrants.map((grant, index) => (
            <div
              key={grant.id}
              className={`transition-all duration-200 ${
                focusedGrantIndex === index
                  ? "ring-2 ring-accent-primary ring-offset-2 ring-offset-bg-primary shadow-lg scale-[1.02]"
                  : ""
              }`}
            >
              <GrantCard grant={grant} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
