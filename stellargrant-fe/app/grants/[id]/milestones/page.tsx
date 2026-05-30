"use client";

import { use, useEffect, useState } from "react";
import { MilestoneList, MilestoneTimeline } from "@/components/milestones";
import type { Milestone } from "@/types";
import { ErrorCard, PageHeader } from "@/components/ui";

/**
 * Milestone List Page
 *
 * Shows all milestones for a grant with their status and progress.
 */

interface MilestonesPageProps {
  params: Promise<{
    id: string;
  }>;
}

/** Raw shape returned by the API (subset of the full Milestone type) */
type MilestoneResponse = {
  idx: number;
  title: string;
  description?: string | null;
  deadline?: string;
  submitted?: boolean;
  approved?: boolean;
  paid?: boolean;
  proof_hash?: string | null;
  submitted_at?: bigint | null;
  approved_at?: bigint | null;
  paid_at?: bigint | null;
  overdue?: boolean;
  daysUntilDeadline?: number;
  token?: string;
  amount?: bigint;
};

/** Normalise a raw API milestone into a full Milestone object */
function normaliseMilestone(raw: MilestoneResponse): Milestone {
  return {
    idx: raw.idx,
    title: raw.title,
    description: raw.description ?? "",
    proof_hash: raw.proof_hash ?? null,
    submitted: raw.submitted ?? false,
    approved: raw.approved ?? false,
    paid: raw.paid ?? false,
    submitted_at: raw.submitted_at ?? null,
    approved_at: raw.approved_at ?? null,
    paid_at: raw.paid_at ?? null,
    token: raw.token,
    amount: raw.amount,
  };
}

export default function MilestonesPage({ params }: MilestonesPageProps) {
  const { id } = use(params);
  const [milestones, setMilestones] = useState<Milestone[]>([]);
  const [title, setTitle] = useState(`Grant #${id}`);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    const loadGrant = async () => {
      try {
        setLoading(true);
        const baseUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:4000";
        const response = await fetch(`${baseUrl}/grants/${id}`, {
          signal: controller.signal,
          cache: "no-store",
        });

        if (!response.ok) {
          throw new Error("Failed to load milestones");
        }

        const payload = await response.json();
        const raw: MilestoneResponse[] = payload.data?.milestones ?? [];
        setMilestones(raw.map(normaliseMilestone));
        setTitle(payload.data?.title ?? `Grant #${id}`);
        setError(null);
      } catch (err) {
        if (controller.signal.aborted) return;
        setError(err instanceof Error ? err.message : "Failed to load milestones");
      } finally {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      }
    };

    void loadGrant();
    return () => controller.abort();
  }, [id, retryCount]);

  return (
    <div className="container mx-auto px-4 py-8">
      <PageHeader
        eyebrow="Creator Timeline"
        title={`Milestones — ${title}`}
        description="Upcoming deadlines, overdue work, and submitted proofs are grouped here so creators can see what needs attention first."
      />

      {loading && <div className="shimmer h-40 rounded-[4px]" />}
      {error && (
        <ErrorCard
          message={error}
          onRetry={() => setRetryCount((c) => c + 1)}
        />
      )}
      {!loading && !error && (
        <>
          <div className="mb-10">
            <MilestoneTimeline milestones={milestones} grantId={id} />
          </div>
          <MilestoneList milestones={milestones} grantId={id} />
        </>
      )}
    </div>
  );
}
