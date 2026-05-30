"use client";

import { useState } from "react";
import Link from "next/link";
import { ChevronDown, ChevronUp, FileCheck } from "lucide-react";
import { VotePanel } from "@/components/milestones/VotePanel";
import {
  getMilestoneStatus,
  getMilestoneStatusClass,
  getMilestoneNodeClass,
} from "@/lib/utils/milestoneStatus";
import type { Milestone } from "@/types";

interface MilestoneTimelineProps {
  milestones: Milestone[];
  grantId: string;
  reviewers?: string[];
  quorum?: number;
}

interface MilestoneCardProps {
  milestone: Milestone;
  grantId: string;
  reviewers: string[];
  quorum: number;
  isLast: boolean;
}

function MiniVoteProgress({
  approved,
  quorum,
  total,
}: {
  approved: number;
  quorum: number;
  total: number;
}) {
  if (total === 0) return null;
  const pct = Math.min(100, Math.round((approved / quorum) * 100));
  return (
    <div className="mt-2">
      <div className="flex items-center justify-between font-mono text-[10px] text-text-muted mb-1">
        <span>Votes</span>
        <span>{approved}/{quorum} approvals</span>
      </div>
      <div className="h-1 w-full bg-surface overflow-hidden">
        <div
          className="h-full bg-success transition-all duration-500"
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function MilestoneCard({
  milestone: m,
  grantId,
  reviewers,
  quorum,
  isLast,
}: MilestoneCardProps) {
  const [votePanelOpen, setVotePanelOpen] = useState(false);
  const status = getMilestoneStatus(m);
  const statusClass = getMilestoneStatusClass(status);
  const nodeClass = getMilestoneNodeClass(status);
  const showVoteToggle = (m.submitted && !m.paid) && reviewers.length > 0;

  return (
    <li className="relative flex gap-4">
      {/* Connector line + node — hidden on mobile, shown md+ */}
      <div className="hidden md:flex flex-col items-center">
        <span
          className={`h-3 w-3 rounded-full border-2 shrink-0 mt-1.5 ${nodeClass}`}
          aria-hidden
        />
        {!isLast && (
          <div className="w-px flex-1 bg-border-color mt-1" aria-hidden />
        )}
      </div>

      {/* Card */}
      <div className="flex-1 mb-6">
        <div className="border border-border-color bg-surface ring-1 ring-border-color">
          <Link
            href={`/grants/${grantId}/milestones/${m.idx}`}
            className="block p-4 hover:border-accent-secondary/50 transition-colors"
          >
            <div className="flex flex-wrap items-center justify-between gap-2 mb-1">
              <span className="font-orbitron text-sm text-text-primary">
                M{m.idx + 1}: {m.title}
              </span>
              <span
                className={`font-mono text-[10px] uppercase tracking-wider px-2 py-0.5 border ${statusClass}`}
              >
                {status}
              </span>
            </div>
            {m.description && (
              <p className="font-mono text-xs text-text-muted line-clamp-2">
                {m.description}
              </p>
            )}

            {/* Mini vote progress on submitted milestones */}
            {m.submitted && quorum > 0 && (
              <MiniVoteProgress
                approved={0}
                quorum={quorum}
                total={reviewers.length}
              />
            )}

            {/* Proof indicator */}
            {m.proof_hash && (
              <div className="mt-2 flex items-center gap-1 font-mono text-[10px] text-accent-secondary">
                <FileCheck size={11} />
                <span>Proof submitted</span>
              </div>
            )}
          </Link>

          {/* Vote panel toggle */}
          {showVoteToggle && (
            <>
              <div className="border-t border-border-color">
                <button
                  type="button"
                  onClick={() => setVotePanelOpen((v) => !v)}
                  className="flex w-full items-center justify-between px-4 py-2 font-mono text-[10px] uppercase tracking-wider text-text-muted hover:text-text-primary transition-colors"
                >
                  <span>Reviewer Votes</span>
                  {votePanelOpen ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                </button>
              </div>
              {votePanelOpen && (
                <div className="border-t border-border-color p-4">
                  <VotePanel
                    grantId={grantId}
                    milestoneIdx={m.idx}
                    reviewers={reviewers}
                    quorum={quorum}
                  />
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </li>
  );
}

export function MilestoneTimeline({
  milestones,
  grantId,
  reviewers = [],
  quorum = 0,
}: MilestoneTimelineProps) {
  if (milestones.length === 0) {
    return (
      <p className="font-mono text-sm text-text-muted">
        No milestones defined yet.
      </p>
    );
  }

  return (
    <>
      {/* Desktop: timeline with connector line via flex column children */}
      <ol className="hidden md:block ml-1">
        {milestones.map((m, idx) => (
          <MilestoneCard
            key={m.idx}
            milestone={m}
            grantId={grantId}
            reviewers={reviewers}
            quorum={quorum}
            isLast={idx === milestones.length - 1}
          />
        ))}
      </ol>

      {/* Mobile: stacked list without connector chrome */}
      <ol className="md:hidden space-y-4">
        {milestones.map((m) => (
          <li key={m.idx}>
            <MilestoneCard
              milestone={m}
              grantId={grantId}
              reviewers={reviewers}
              quorum={quorum}
              isLast={false}
            />
          </li>
        ))}
      </ol>
    </>
  );
}
