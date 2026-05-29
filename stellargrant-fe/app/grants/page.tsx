"use client";

import { useEffect, useState } from "react";
import { GrantCard } from "@/components/grants/GrantCard";
import { apiGet } from "@/lib/api";
import { EmptyState, ErrorCard, PageHeader } from "@/components/ui";

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
  const [grants, setGrants] = useState<GrantCardInput[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

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
          setError(err instanceof Error ? err.message : "Failed to load grants");
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

      {loading && (
        <div className="grid gap-4 md:grid-cols-2">
          {Array.from({ length: 4 }).map((_, index) => (
            <div key={index} className="shimmer h-52 rounded-[4px]" />
          ))}
        </div>
      )}

      {error && (
        <ErrorCard
          message={error}
          onRetry={() => setRetryCount((c) => c + 1)}
        />
      )}

      {!loading && !error && grants.length === 0 && (
        <EmptyState
          title="No grants yet"
          description="Be the first to create a grant and kick off a milestone track."
          action={{ label: "Create a grant", href: "/grants/create" }}
        />
      )}

      {!loading && !error && grants.length > 0 && (
        <div className="grid gap-4 md:grid-cols-2">
          {grants.map((grant) => (
            <GrantCard key={grant.id} grant={grant} />
          ))}
        </div>
      )}
    </div>
  );
}
