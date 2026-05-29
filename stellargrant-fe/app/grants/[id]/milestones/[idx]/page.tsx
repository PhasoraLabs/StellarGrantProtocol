"use client";

/**
 * Milestone Detail Page
 *
 * Shows a single milestone's submitted proof, reviewer vote tally,
 * and role-based action buttons:
 *   • Recipient wallet  → Submit Proof form (when milestone is pending)
 *   • Reviewer wallet   → Approve / Reject buttons (via VotePanel / useVoting)
 *   • Any other wallet  → Read-only proof + vote tally
 *
 * Route: /grants/[id]/milestones/[idx]
 *
 * Breadcrumb: Grants → Grant #id → Milestones → Milestone idx
 */

import { use, useEffect, useState } from "react";
import Link from "next/link";
import { useGrant } from "@/hooks/useGrant";
import { useMilestone } from "@/hooks/useMilestone";
import { useWalletStore } from "@/lib/store/walletStore";
import { formatTokenAmount, getTokenMetadata } from "@/lib/tokens";
import { Badge } from "@/components/ui/Badge";
import { VotePanel } from "@/components/milestones/VotePanel";
import { ProofViewer } from "@/components/milestones/ProofViewer";
import { MilestoneSubmitForm } from "@/components/milestones/MilestoneSubmitForm";
import type { BadgeVariant } from "@/components/ui/Badge";
import type { Milestone } from "@/types";

// Note: generateMetadata is a server-only API and cannot be exported from a
// "use client" page. Page title is set via document.title in MilestoneDetailContent
// instead, matching the pattern used by other client pages in this codebase.

// ─── Status → Badge variant mapping ──────────────────────────────────────────

function milestoneVariant(m: Milestone): BadgeVariant {
  if (m.paid)     return "success";
  if (m.approved) return "info";
  if (m.submitted) return "warning";
  return "muted";
}

function milestoneStatusLabel(m: Milestone): string {
  if (m.paid)      return "Paid";
  if (m.approved)  return "Approved";
  if (m.submitted) return "Submitted";
  if (m.overdue)   return "Overdue";
  return "Pending";
}

// ─── Loading skeleton ─────────────────────────────────────────────────────────

function MilestoneDetailSkeleton() {
  return (
    <div className="container mx-auto max-w-3xl px-4 py-8 space-y-8">
      {/* breadcrumb */}
      <div className="shimmer h-3 w-64 rounded-none" />
      {/* header */}
      <div className="flex items-center justify-between gap-4">
        <div className="shimmer h-8 w-72 rounded-none" />
        <div className="shimmer h-6 w-20 rounded-none" />
      </div>
      {/* proof area */}
      <div className="shimmer h-32 w-full rounded-none" />
      {/* vote panel */}
      <div className="shimmer h-40 w-full rounded-none" />
      {/* milestone rows */}
      <div className="space-y-3">
        {[0, 1, 2].map((i) => (
          <div key={i} className="shimmer h-16 w-full rounded-none" />
        ))}
      </div>
    </div>
  );
}

// ─── Error card ───────────────────────────────────────────────────────────────

function ErrorCard({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="container mx-auto max-w-3xl px-4 py-8">
      <div className="rounded-none border border-danger/40 bg-danger/10 p-6 space-y-4">
        <p className="font-mono text-sm text-danger">{message}</p>
        <button
          onClick={onRetry}
          className="rounded-none border border-danger/40 px-4 py-2 font-mono text-xs uppercase tracking-widest text-danger hover:bg-danger/10 transition-colors"
        >
          Retry
        </button>
      </div>
    </div>
  );
}

// ─── Breadcrumb ───────────────────────────────────────────────────────────────

function Breadcrumb({ grantId, milestoneIdx }: { grantId: string; milestoneIdx: string }) {
  const crumbs = [
    { label: "Grants",          href: "/grants" },
    { label: `Grant #${grantId}`, href: `/grants/${grantId}` },
    { label: "Milestones",      href: `/grants/${grantId}/milestones` },
    { label: `Milestone ${milestoneIdx}`, href: null },
  ];

  return (
    <nav className="flex items-center gap-1.5 font-mono text-xs text-text-muted" aria-label="Breadcrumb">
      {crumbs.map((c, i) => (
        <span key={i} className="flex items-center gap-1.5">
          {i > 0 && <span className="text-border-color">›</span>}
          {c.href ? (
            <Link href={c.href} className="hover:text-text-primary transition-colors">
              {c.label}
            </Link>
          ) : (
            <span className="text-text-primary">{c.label}</span>
          )}
        </span>
      ))}
    </nav>
  );
}

// ─── Main page content ────────────────────────────────────────────────────────

function MilestoneDetailContent({
  grantId,
  milestoneIdx,
}: {
  grantId: string;
  milestoneIdx: number;
}) {
  const { data: grantDetail, isLoading: grantLoading, error: grantError, refetch } = useGrant(grantId);
  const grant = grantDetail?.grant ?? null;
  const {
    milestone,
    isLoading: milestoneLoading,
    error: milestoneError,
  } = useMilestone(grantId, milestoneIdx);
  const { address: walletAddress } = useWalletStore();
  const [amountFormatted, setAmountFormatted] = useState<string>("");

  // Set page title dynamically (generateMetadata cannot be used in "use client" pages)
  useEffect(() => {
    document.title = `Milestone ${milestoneIdx} — Grant #${grantId} | StellarGrants`;
  }, [grantId, milestoneIdx]);

  // Format reward amount when milestone + token metadata are available
  useEffect(() => {
    if (!milestone?.amount || !milestone.token) return;
    getTokenMetadata(milestone.token).then((meta) => {
      setAmountFormatted(
        formatTokenAmount(milestone.amount!, meta.decimals, {
          symbol: meta.symbol,
          showSymbol: true,
        })
      );
    });
  }, [milestone?.amount, milestone?.token]);

  if (grantLoading || milestoneLoading) return <MilestoneDetailSkeleton />;

  if (grantError) {
    return <ErrorCard message={grantError.message} onRetry={() => void refetch()} />;
  }
  if (milestoneError) {
    return <ErrorCard message={milestoneError.message} onRetry={() => void refetch()} />;
  }

  // No grant data (e.g. 404)
  if (!grant) {
    return (
      <ErrorCard
        message={`Grant #${grantId} could not be found.`}
        onRetry={() => void refetch()}
      />
    );
  }

  // Roles
  const isRecipient = !!walletAddress && walletAddress === grant.owner;
  const isReviewer  = !!walletAddress && grant.reviewers.includes(walletAddress);

  // Milestone may be null if the grant came from the mock and doesn't carry
  // individual milestone objects — show a placeholder in that case.
  const statusLabel   = milestone ? milestoneStatusLabel(milestone) : "Unknown";
  const badgeVariant  = milestone ? milestoneVariant(milestone) : "muted";
  const milestoneTitle = milestone?.title ?? `Milestone ${milestoneIdx}`;

  const showSubmitForm =
    isRecipient && milestone && !milestone.submitted && !milestone.approved;
  const showProof      = milestone?.proof_hash;

  return (
    <div className="container mx-auto max-w-3xl px-4 py-8 space-y-8">
      {/* ── Breadcrumb ────────────────────────────────────────────────── */}
      <Breadcrumb grantId={grantId} milestoneIdx={String(milestoneIdx)} />

      {/* ── Header ───────────────────────────────────────────────────── */}
      <div>
        <div className="flex flex-wrap items-center justify-between gap-4 mb-2">
          <h1 className="text-2xl font-bold">{milestoneTitle}</h1>
          <Badge variant={badgeVariant}>{statusLabel}</Badge>
        </div>

        {milestone?.description && (
          <p className="text-sm leading-6 text-text-muted">{milestone.description}</p>
        )}

        {/* Reward amount */}
        {amountFormatted && (
          <p className="mt-2 font-mono text-xs text-text-muted">
            Reward:{" "}
            <span className="text-text-primary font-semibold">{amountFormatted}</span>
          </p>
        )}
      </div>

      {/* ── Proof section ─────────────────────────────────────────────── */}
      <section className="space-y-4">
        <h2 className="font-orbitron text-sm uppercase tracking-widest text-text-muted">
          Proof of Work
        </h2>

        {showProof ? (
          <ProofViewer proofHash={milestone!.proof_hash!} />
        ) : showSubmitForm ? (
          <div className="rounded-none border border-border-color/40 p-6">
            <p className="mb-4 font-mono text-xs uppercase tracking-widest text-text-muted">
              Submit milestone proof
            </p>
            <MilestoneSubmitForm grantId={grantId} milestoneIdx={milestoneIdx} />
          </div>
        ) : (
          <div className="rounded-none border border-border-color/40 p-4">
            <p className="font-mono text-xs text-text-muted">
              {milestone?.submitted
                ? "Proof submitted — awaiting reviewer votes."
                : "No proof submitted yet."}
            </p>
          </div>
        )}
      </section>

      {/* ── Vote panel ────────────────────────────────────────────────── */}
      <section className="space-y-4">
        <h2 className="font-orbitron text-sm uppercase tracking-widest text-text-muted">
          Reviewer Votes
        </h2>
        <VotePanel
          grantId={grantId}
          milestoneIdx={milestoneIdx}
          reviewers={grant.reviewers}
          quorum={Math.ceil(grant.reviewers.length * 0.67)}
          threshold={0.67}
        />
        {!isReviewer && !isRecipient && walletAddress && (
          <p className="font-mono text-xs text-text-muted">
            Your wallet is not a reviewer or the grant recipient — viewing in read-only mode.
          </p>
        )}
        {!walletAddress && (
          <p className="font-mono text-xs text-text-muted">
            Connect your wallet to participate.
          </p>
        )}
      </section>
    </div>
  );
}

// ─── Page entry-point ─────────────────────────────────────────────────────────

export default function MilestoneDetailPage({
  params,
}: {
  params: Promise<{ id: string; idx: string }>;
}) {
  const { id, idx } = use(params);
  const milestoneIdx = parseInt(idx, 10);

  if (isNaN(milestoneIdx)) {
    return (
      <div className="container mx-auto max-w-3xl px-4 py-8">
        <p className="font-mono text-sm text-danger">
          Invalid milestone index: &ldquo;{idx}&rdquo;
        </p>
      </div>
    );
  }

  return <MilestoneDetailContent grantId={id} milestoneIdx={milestoneIdx} />;
}
