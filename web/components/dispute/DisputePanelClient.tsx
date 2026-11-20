"use client";

import { useEffect, useState } from "react";
import { WalletGuard } from "@/components/wallet/WalletGuard";
import { useWalletStore } from "@/lib/store/walletStore";
import { contractClient } from "@/lib/stellar/contract";
import { DisputePanel } from "./DisputePanel";
import { DisputeHistory, ResolvedDispute } from "./DisputeHistory";
import { apiGet } from "@/lib/api";

interface DisputeItem {
  grantId: string;
  grantTitle: string;
  milestoneIdx: number;
  milestoneTitle: string;
  proofHash: string;
  contributorArgument: string | null;
  funderArgument: string | null;
  fundedAmount: bigint;
  token: string;
  priorVotes: { approved: number; rejected: number };
}

interface RawDisputeResponse {
  grantId: string | number;
  grantTitle?: string;
  milestoneIdx: number;
  milestoneTitle?: string;
  proofHash?: string;
  contributorArgument?: string | null;
  funderArgument?: string | null;
  fundedAmount?: string | number;
  token?: string;
  priorVotes?: { approved: number; rejected: number };
}

interface RawGrantResponse {
  id: string | number;
  status: string | number;
  title: string;
  token?: string;
}

interface RawMilestoneResponse {
  idx: number;
  title?: string;
  proof_hash?: string;
  amount?: string | number;
  token?: string;
  approvals?: number;
  rejections?: number;
  state?: string;
  approved?: boolean;
  submitted?: boolean;
}

interface RawMetaResponse {
  contributorArgument?: string | null;
  funderArgument?: string | null;
  priorVotes?: { approved: number; rejected: number };
}

interface RawHistoryResponse {
  id?: string;
  grantId: string | number;
  grantTitle?: string;
  milestoneIdx: number;
  milestoneTitle?: string;
  resolution?: string;
  resolvedAt?: string;
  fundedAmount?: string | number;
  token?: string;
}

export default function DisputePanelClient() {
  const { address } = useWalletStore();
  const [isCouncil, setIsCouncil] = useState<boolean | null>(null);
  const [checkingCouncil, setCheckingCouncil] = useState(false);
  const [disputes, setDisputes] = useState<DisputeItem[]>([]);
  const [history, setHistory] = useState<ResolvedDispute[]>([]);
  const [loadingData, setLoadingData] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [refreshTrigger, setRefreshTrigger] = useState(0);

  // 1. Check council membership on mount/wallet change
  useEffect(() => {
    if (!address) {
      queueMicrotask(() => setIsCouncil(null));
      return;
    }

    const checkMember = async () => {
      queueMicrotask(() => setCheckingCouncil(true));
      try {
        const member = await contractClient.isCouncilMember({ address });
        setIsCouncil(member);
      } catch (err) {
        console.error("Error checking council membership:", err);
        setIsCouncil(false);
      } finally {
        setCheckingCouncil(false);
      }
    };

    void checkMember();
  }, [address]);

  // 2. Fetch disputes and history when council status is verified
  useEffect(() => {
    if (!isCouncil) return;

    let cancelled = false;

    const fetchData = async () => {
      queueMicrotask(() => {
        setLoadingData(true);
        setErrorMsg(null);
      });
      try {
        // Fetch Open Disputes
        let openDisputes: DisputeItem[] = [];
        try {
          const rawOpen = await apiGet<RawDisputeResponse[]>("/disputes?status=open");
          if (Array.isArray(rawOpen)) {
            openDisputes = rawOpen.map((item) => ({
              grantId: String(item.grantId),
              grantTitle: item.grantTitle || `Grant #${item.grantId}`,
              milestoneIdx: Number(item.milestoneIdx),
              milestoneTitle: item.milestoneTitle || `Milestone #${item.milestoneIdx + 1}`,
              proofHash: item.proofHash || "",
              contributorArgument: item.contributorArgument ?? null,
              funderArgument: item.funderArgument ?? null,
              fundedAmount: BigInt(item.fundedAmount || 0),
              token: item.token || "native",
              priorVotes: item.priorVotes || { approved: 0, rejected: 0 },
            }));
          }
        } catch (err) {
          console.warn("Direct disputes endpoint failed, attempting fallback query...", err);
          // Fallback: Fetch all grants and filter by disputed status/milestones
          const allGrants = await apiGet<RawGrantResponse[]>("/grants");
          if (Array.isArray(allGrants)) {
            const disputedGrants = allGrants.filter(
              (g) => g.status === "Disputed" || Number(g.status) === 5
            );
            
            for (const g of disputedGrants) {
              try {
                const milestones = await apiGet<RawMilestoneResponse[]>(`/grants/${g.id}/milestones`);
                if (Array.isArray(milestones)) {
                  const disputedMilestones = milestones.filter(
                    (m) => m.state === "Disputed" || m.approved === false && m.submitted === true // Or status state check
                  );
                  for (const m of disputedMilestones) {
                    openDisputes.push({
                      grantId: String(g.id),
                      grantTitle: g.title,
                      milestoneIdx: Number(m.idx),
                      milestoneTitle: m.title || `Milestone #${m.idx + 1}`,
                      proofHash: m.proof_hash || "",
                      contributorArgument: null,
                      funderArgument: null,
                      fundedAmount: BigInt(m.amount || 0),
                      token: m.token || g.token || "native",
                      priorVotes: { approved: m.approvals || 0, rejected: m.rejections || 0 },
                    });
                  }
                }
              } catch (milestoneErr) {
                console.error(`Failed to fetch milestones for fallback grant #${g.id}:`, milestoneErr);
              }
            }
          }
        }

        // Fetch detailed arguments for open disputes
        for (const dispute of openDisputes) {
          if (!dispute.contributorArgument || !dispute.funderArgument) {
            try {
              const meta = await apiGet<RawMetaResponse>(
                `/grants/${dispute.grantId}/milestones/${dispute.milestoneIdx}/dispute`
              );
              if (meta) {
                dispute.contributorArgument = meta.contributorArgument ?? dispute.contributorArgument;
                dispute.funderArgument = meta.funderArgument ?? dispute.funderArgument;
                if (meta.priorVotes) {
                  dispute.priorVotes = meta.priorVotes;
                }
              }
            } catch (metaErr) {
              console.warn(
                `Failed to fetch argument metadata for Grant #${dispute.grantId} Milestone #${dispute.milestoneIdx}`,
                metaErr
              );
            }
          }
        }

        // Fetch Resolved Disputes
        let resolvedDisputes: ResolvedDispute[] = [];
        try {
          const rawHistory = await apiGet<RawHistoryResponse[]>("/disputes?status=resolved");
          if (Array.isArray(rawHistory)) {
            resolvedDisputes = rawHistory.map((item) => ({
              id: item.id || `${item.grantId}-${item.milestoneIdx}`,
              grantTitle: item.grantTitle || `Grant #${item.grantId}`,
              milestoneIdx: Number(item.milestoneIdx),
              milestoneTitle: item.milestoneTitle || `Milestone #${item.milestoneIdx + 1}`,
              resolution: item.resolution === "payout" ? "payout" : "refund",
              resolvedAt: item.resolvedAt || new Date().toISOString(),
              fundedAmount: BigInt(item.fundedAmount || 0),
              token: item.token || "native",
            }));
          }
        } catch (historyErr) {
          console.warn("Direct resolved disputes endpoint failed.", historyErr);
        }

        if (!cancelled) {
          setDisputes(openDisputes);
          setHistory(resolvedDisputes);
        }
      } catch (err: unknown) {
        if (!cancelled) {
          setErrorMsg((err as Error)?.message || "Failed to load dispute panel data.");
        }
      } finally {
        if (!cancelled) {
          setLoadingData(false);
        }
      }
    };

    void fetchData();

    return () => {
      cancelled = true;
    };
  }, [isCouncil, refreshTrigger]);

  if (checkingCouncil) {
    return (
      <div className="flex flex-col items-center justify-center py-20 space-y-4">
        <div className="w-8 h-8 border-4 border-accent-secondary border-t-transparent rounded-full animate-spin" />
        <p className="font-mono text-sm text-text-muted">Verifying Council authorization...</p>
      </div>
    );
  }

  if (isCouncil === false) {
    return (
      <div className="max-w-md mx-auto my-12">
        <div className="bg-surface/60 backdrop-blur-md border border-warning/20 p-8 text-center space-y-6 shadow-xl relative overflow-hidden group">
          {/* Animated glow background */}
          <div className="absolute inset-0 bg-warning/5 opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
          
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-warning/10 border border-warning/30 text-warning animate-pulse">
            <svg
              className="w-8 h-8"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
          </div>

          <div className="space-y-2">
            <h2 className="font-orbitron text-lg font-bold text-text-primary uppercase tracking-wider">
              Restricted Access
            </h2>
            <p className="font-mono text-xs text-text-muted leading-relaxed">
              This panel is only accessible to StellarGrant Council members.
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <WalletGuard>
      <div className="space-y-8">
        {/* Dashboard Header */}
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-border-color/10 pb-6">
          <div>
            <span className="font-mono text-xs text-accent-secondary uppercase tracking-widest block mb-1">
              [ Administrative Dashboard ]
            </span>
            <h1 className="font-orbitron text-2xl font-bold uppercase tracking-wider text-text-primary">
              Council Dispute Panel
            </h1>
          </div>
          
          <div className="flex gap-4 font-mono text-xs">
            <div className="bg-surface/20 border border-border-color/20 px-4 py-2">
              <span className="text-text-muted block text-[10px] uppercase tracking-wider">Open Disputes</span>
              <span className="text-base font-bold text-danger">{disputes.length} active</span>
            </div>
            <div className="bg-surface/20 border border-border-color/20 px-4 py-2">
              <span className="text-text-muted block text-[10px] uppercase tracking-wider">Resolved History</span>
              <span className="text-base font-bold text-success">{history.length} cases</span>
            </div>
          </div>
        </div>

        {/* Loading Spinner */}
        {loadingData && (
          <div className="grid gap-6">
            <div className="shimmer h-64 rounded-none border border-border-color/20" />
            <div className="shimmer h-64 rounded-none border border-border-color/20" />
          </div>
        )}

        {/* Error State */}
        {errorMsg && !loadingData && (
          <div className="bg-danger/10 border border-danger/20 p-4 font-mono text-xs text-danger text-center flex flex-col items-center gap-3">
            <span>{errorMsg}</span>
            <button
              onClick={() => setRefreshTrigger((c) => c + 1)}
              className="px-4 py-1.5 bg-danger text-black font-bold uppercase tracking-widest text-[10px]"
            >
              Retry
            </button>
          </div>
        )}

        {/* Content list */}
        {!loadingData && !errorMsg && (
          <div className="space-y-8">
            {disputes.length === 0 ? (
              <div className="bg-surface/10 border border-border-color/20 p-12 text-center">
                <p className="font-mono text-xs text-text-muted uppercase tracking-widest">
                  ✓ All quiet. No active disputes require attention.
                </p>
              </div>
            ) : (
              <div className="space-y-6">
                {disputes.map((dispute) => (
                  <DisputePanel
                    key={`${dispute.grantId}-${dispute.milestoneIdx}`}
                    {...dispute}
                    onResolved={() => setRefreshTrigger((c) => c + 1)}
                  />
                ))}
              </div>
            )}

            {/* Historical disputes */}
            <DisputeHistory history={history} />
          </div>
        )}
      </div>
    </WalletGuard>
  );
}
