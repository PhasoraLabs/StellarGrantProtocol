"use client";

import { use, Suspense, useEffect, useMemo, useState, useCallback, useRef } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { ExternalLink, Download, ChevronDown } from "lucide-react";
import { FundingProgress } from "@/components/grants/FundingProgress";
import { GrantStatusBadge } from "@/components/grants/GrantStatusBadge";
import { WalletAddress } from "@/components/wallet/WalletAddress";
import { GrantStats } from "@/components/grants/GrantStats";
import { MilestoneTimeline } from "@/components/milestones/MilestoneTimeline";
import { FundGrantModal } from "@/components/grants/FundGrantModal";
import { WatchButton } from "@/components/grants/WatchButton";
import { ShareButton } from "@/components/grants/ShareButton";
import RichTextRenderer from "@/components/ui/RichTextRenderer";
import { formatDate } from "@/lib/utils";
import { formatTokenAmount, getTokenMetadata } from "@/lib/tokens";
import { stellarExplorerAccount } from "@/lib/constants";
import { exportGrantAsJSON, exportGrantAsCSV, exportFundersAsCSV } from "@/lib/utils/export";
import type { FunderRecord } from "@/lib/utils/export";
import { useGrant } from "@/hooks/useGrant";
import { useFunders } from "@/hooks/useFunders";
import { useGrantBalances } from "@/hooks/useGrantBalances";
import { useRelativeTime } from "@/hooks/useRelativeTime";
import { useContractEvents } from "@/hooks/useContractEvents";
import { useOptimisticGrant } from "@/hooks/useOptimisticGrant";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { toast } from "@/lib/toast";
import { ErrorCard } from "@/components/ui/ErrorCard";
import type { TokenMetadata, Grant, Milestone } from "@/types";
import type { GrantBalances } from "@/lib/stellar/balances";

function ExportDropdown({
  grant,
  milestones,
  funders,
}: {
  grant: Grant;
  milestones: Milestone[];
  funders: FunderRecord[];
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    if (open) document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const items: { label: string; action: () => void }[] = [
    {
      label: "Export as JSON",
      action: () => {
        exportGrantAsJSON(grant, milestones);
        setOpen(false);
      },
    },
    {
      label: "Export milestones as CSV",
      action: () => {
        exportGrantAsCSV(grant, milestones);
        setOpen(false);
      },
    },
    {
      label: "Export funders as CSV",
      action: () => {
        exportFundersAsCSV(funders);
        setOpen(false);
      },
    },
  ];

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="inline-flex items-center gap-1.5 font-mono text-xs uppercase tracking-wider border border-border-color text-text-muted px-3 py-1.5 hover:border-accent-primary hover:text-accent-primary transition-colors"
        aria-expanded={open}
        aria-haspopup="menu"
      >
        <Download size={12} />
        Export
        <ChevronDown size={12} className={open ? "rotate-180 transition-transform" : "transition-transform"} />
      </button>

      {open && (
        <div
          role="menu"
          className="absolute right-0 top-full mt-1 z-50 min-w-[200px] border border-border-color bg-surface shadow-lg"
        >
          {items.map((item) => (
            <button
              key={item.label}
              type="button"
              role="menuitem"
              onClick={item.action}
              className="block w-full text-left font-mono text-xs text-text-primary px-4 py-2.5 hover:bg-bg-secondary transition-colors"
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function daysUntilDeadline(deadlineTs: bigint): number {
  const ms = Number(deadlineTs) * 1000 - Date.now();
  return Math.ceil(ms / (1000 * 60 * 60 * 24));
}

function GrantDetailSkeleton() {
  return (
    <div className="container mx-auto max-w-6xl px-4 py-8">
      <div className="shimmer h-4 w-48 rounded-none mb-4" />
      <div className="flex flex-wrap gap-4 mb-6">
        <div className="shimmer h-10 flex-1 min-w-[200px] rounded-none" />
        <div className="shimmer h-8 w-24 rounded-none" />
      </div>
      <div className="shimmer h-4 w-64 rounded-none mb-8" />
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3 mb-8">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="shimmer h-16 rounded-none" />
        ))}
      </div>
      <div className="flex flex-col lg:flex-row gap-8">
        <div className="flex-1 space-y-6">
          <div className="shimmer h-40 rounded-none" />
          <div className="shimmer h-6 w-32 rounded-none" />
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="shimmer h-20 rounded-none" />
          ))}
        </div>
        <div className="w-full lg:w-80 space-y-4 shrink-0">
          <div className="shimmer h-48 rounded-none" />
          <div className="shimmer h-32 rounded-none" />
        </div>
      </div>
    </div>
  );
}


function GrantDetailContent({ grantId }: { grantId: string }) {
  const router = useRouter();
  const { data, isLoading, error, errorType, refetch } = useGrant(grantId);
  const { 
    events, 
    connectionStatus 
  } = useContractEvents({ grantId });
  
  const { 
    grant: optimisticGrant, 
    applyMutation
  } = useOptimisticGrant(data?.grant ?? {} as Grant, data?.milestones ?? []);

  const { funders, isLoading: fundersLoading, refetch: refetchFunders } = useFunders(grantId);
  const [fundModalOpen, setFundModalOpen] = useState(false);
  const [tokenMetadata, setTokenMetadata] = useState<TokenMetadata | null>(null);

  useKeyboardShortcuts([
    {
      key: "F",
      description: "Fund Grant",
      action: (e) => {
        e?.preventDefault();
        if (!optimisticGrant) return;
        const statusLabel = ["Pending", "Active", "In Progress", "Completed", "Cancelled"][optimisticGrant.status] ?? "Pending";
        if (statusLabel !== "Completed" && statusLabel !== "Cancelled") {
          setFundModalOpen(true);
        }
      },
    },
    {
      key: "h",
      description: "Grant History",
      action: (e) => {
        e?.preventDefault();
        router.push(`/grants/${grantId}/history`);
      },
    },
  ]);

  // Handle incoming GrantFunded events
  useEffect(() => {
    const latestEvent = events[events.length - 1];
    if (latestEvent?.type === "GrantFunded") {
      // 1. apply optimistic update
      if (latestEvent.data.amount) {
        applyMutation({ 
          type: "fund", 
          amount: BigInt(latestEvent.data.amount as string)
        });
      }
      // 2. refetch confirmed data
      void refetch();
      void refetchFunders();
    }
  }, [events, applyMutation, refetch, refetchFunders]);

  const grant = optimisticGrant;
  const milestones = data?.milestones ?? [];

  const handleBalanceChange = useCallback(
    (_current: GrantBalances, previous: GrantBalances | null) => {
      if (!grant || grant.budget === 0n) return;
      const prevFunded = previous
        ? previous.balances
            .filter((b) => b.isNative)
            .reduce((sum, b) => sum + b.balanceStroops, 0n)
        : grant.funded;
      if (prevFunded < grant.budget) {
        const newPct = Math.min(
          100,
          Number((prevFunded * 100n) / grant.budget)
        );
        toast({
          title: "New funding received!",
          description: `Grant is now ${newPct.toFixed(1)}% funded.`,
          variant: "success",
        });
      }
    },
    [grant]
  );

  const contractAddress = grant?.contractAddress ?? "";

  const {
    balances,
    isLoading: balancesLoading,
    lastUpdated,
  } = useGrantBalances({
    contractAddress,
    pollInterval: 10_000,
    enabled: !!grant && !!contractAddress,
    onChange: handleBalanceChange,
  });

  const liveFunded = useMemo(() => {
    if (!balancesLoading && balances?.balances) {
      const nativeBalance = balances.balances.find((b) => b.isNative);
      if (nativeBalance) return nativeBalance.balanceStroops;
    }
    return grant?.funded ?? 0n;
  }, [balances, balancesLoading, grant?.funded]);

  const liveTokens = useMemo(() => {
    if (!balances?.balances) return undefined;
    return balances.balances.map((b) => ({
      token: b.assetCode === "XLM" ? "native" : b.assetCode,
      amount: b.balanceStroops,
    }));
  }, [balances]);

  const freshnessLabel = useRelativeTime(lastUpdated ?? new Date());

  useEffect(() => {
    if (!grant?.token) return;
    getTokenMetadata(grant.token)
      .then(setTokenMetadata)
      .catch(() => setTokenMetadata(null));
  }, [grant?.token]);

  const fundedPercent = useMemo(() => {
    if (!grant || grant.budget === 0n) return 0;
    return Math.min(100, Number((liveFunded * 100n) / grant.budget));
  }, [grant, liveFunded]);

  if (isLoading) return <GrantDetailSkeleton />;

  if (error || !grant || !data) {
    return (
      <div className="container mx-auto max-w-6xl px-4 py-8">
        <ErrorCard
          type={errorType}
          message={error?.message}
          onRetry={() => void refetch()}
          title={!grant && !error ? "Grant Not Found" : undefined}
        />
      </div>
    );
  }

  const decimals = tokenMetadata?.decimals ?? 7;
  const symbol = tokenMetadata?.symbol ?? "XLM";
  const statusLabel = ["Pending", "Active", "In Progress", "Completed", "Cancelled"][grant.status] ?? "Pending";
  const canFund = statusLabel !== "Completed" && statusLabel !== "Cancelled";
  const daysLeft = grant.deadline > 0n ? daysUntilDeadline(grant.deadline) : null;
  const quorumRequired =
    grant.reviewers.length > 0
      ? Math.floor(grant.reviewers.length / 2) + 1
      : 0;

  return (
    <div className="container mx-auto max-w-6xl px-4 py-8">
      {/* Header */}
      <nav className="mb-4 font-mono text-xs text-text-muted">
        <Link href="/grants" className="hover:text-accent-secondary transition-colors">
          Grants
        </Link>
        <span className="mx-2">/</span>
        <span className="text-text-primary">{grant.title}</span>
      </nav>

      <div className="flex flex-wrap items-start justify-between gap-4 mb-2">
        <h1 className="font-orbitron text-3xl text-text-primary flex-1 min-w-0 wrap-break-word">
          {grant.title}
        </h1>
        <div className="flex flex-wrap items-center gap-2 shrink-0">
          <GrantStatusBadge status={grant.status} />
          <WatchButton grantId={grant.id} />
          <ShareButton
            grantId={grant.id}
            grantTitle={grant.title}
            fundedPercent={fundedPercent}
          />
          <ExportDropdown
            grant={grant}
            milestones={milestones}
            funders={funders.map((f) => ({
              address: f.address,
              amount: f.amount,
              token: grant.token ?? "native",
              timestamp: new Date().toISOString(),
            }))}
          />
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-4 mb-6 text-sm">
        <span className="font-mono text-xs uppercase tracking-wider text-text-muted">
          Owner:{" "}
          <WalletAddress address={grant.owner} />
        </span>
        <a
          href={stellarExplorerAccount(grant.owner)}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1 font-mono text-xs text-accent-secondary hover:underline"
        >
          Explorer <ExternalLink size={12} />
        </a>
        <span className="font-mono text-xs text-text-muted">
          Created: {formatDate(Number(grant.created_at) * 1000)}
        </span>
        {daysLeft !== null && daysLeft > 0 && daysLeft < 7 && (
          <span className="font-mono text-xs uppercase tracking-wider px-2 py-0.5 border border-warning/50 text-warning bg-warning/10">
            Deadline in {daysLeft} day{daysLeft === 1 ? "" : "s"}
          </span>
        )}
      </div>

      <div className="mb-8">
        <GrantStats
          totalBudget={grant.budget}
          fundedAmount={grant.funded}
          milestoneCount={milestones.length || grant.milestones}
          completedMilestones={data.completedMilestones}
          reviewerCount={grant.reviewers.length}
          token={symbol}
          deadline={grant.deadline}
        />
      </div>

      <div className="flex flex-col lg:flex-row gap-8">
        {/* Left column */}
        <div className="flex-1 min-w-0 space-y-8 order-2 lg:order-1">
          <section className="border border-border-color bg-surface p-6 ring-1 ring-border-color">
            <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-muted mb-4">
              Description
            </h2>
            <RichTextRenderer content={grant.description} />
          </section>

          <section>
            <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-muted mb-4">
              Milestones
            </h2>
            <MilestoneTimeline milestones={milestones} grantId={grant.id} />
          </section>

          <section className="lg:hidden">
            <h2 className="font-mono text-[10px] uppercase tracking-[0.24em] text-text-muted mb-3">
              Reviewers
            </h2>
            {grant.reviewers.length === 0 ? (
              <p className="font-mono text-sm text-text-muted">No reviewers assigned.</p>
            ) : (
              <>
                <div className="flex flex-wrap gap-2">
                  {grant.reviewers.map((addr) => (
                    <span
                      key={addr}
                      className="inline-flex border border-border-color bg-surface px-2 py-1"
                    >
                      <WalletAddress address={addr} />
                    </span>
                  ))}
                </div>
                {quorumRequired > 0 && (
                  <p className="mt-3 font-mono text-xs text-text-muted">
                    Requires {quorumRequired} of {grant.reviewers.length} approvals
                  </p>
                )}
              </>
            )}
          </section>
        </div>

        {/* Right sidebar */}
        <aside className="w-full lg:w-80 shrink-0 space-y-6 order-1 lg:order-2">
          <section className="border border-border-color bg-surface p-5 ring-1 ring-border-color">
            <FundingProgress
              current={liveFunded}
              target={grant.budget}
              token={grant.token}
              tokens={liveTokens}
              showBreakdown
            />
            {!!contractAddress && (
              <p className="font-mono text-[10px] text-text-muted mt-1">
                On-chain balance · Updated {freshnessLabel}
              </p>
            )}
            {canFund && (
              <button
                type="button"
                onClick={() => setFundModalOpen(true)}
                className="mt-4 w-full font-orbitron text-sm font-bold uppercase tracking-wider bg-accent-primary text-bg-primary py-3 hover:opacity-90 transition-opacity"
              >
                Fund This Grant
              </button>
            )}
            
            <div className="mt-4 flex items-center justify-center gap-2">
              <div className={`h-2 w-2 rounded-full ${connectionStatus === 'connected' ? 'bg-success animate-pulse' : 'bg-text-muted'}`} />
              <span className={`font-mono text-[10px] uppercase tracking-widest ${connectionStatus === 'connected' ? 'text-success' : 'text-text-muted'}`}>
                {connectionStatus === 'connected' ? 'Live' : 'Offline'}
              </span>
            </div>
          </section>

          <section className="border border-border-color bg-surface p-5 ring-1 ring-border-color">
            <h3 className="font-mono text-[10px] uppercase tracking-[0.24em] text-text-muted mb-3">
              Top Funders
            </h3>
            {fundersLoading ? (
              <div className="space-y-2">
                {Array.from({ length: 3 }).map((_, i) => (
                  <div key={i} className="shimmer h-6 rounded-none" />
                ))}
              </div>
            ) : funders.length === 0 ? (
              <p className="font-mono text-xs text-text-muted">
                Be the first to fund this grant
              </p>
            ) : (
              <ul className="space-y-2">
                {funders.map((f) => (
                  <li
                    key={f.address}
                    className="flex items-center justify-between gap-2 font-mono text-xs"
                  >
                    <WalletAddress address={f.address} showCopyIcon={false} />
                    <span className="text-text-primary shrink-0">
                      {formatTokenAmount(f.amount, decimals, {
                        symbol,
                        showSymbol: true,
                      })}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="border border-border-color bg-surface p-5 ring-1 ring-border-color">
            <Link
              href={`/grants/${grant.id}/history`}
              className="font-mono text-xs text-accent-secondary hover:underline"
            >
              History →
            </Link>
          </section>

          <section className="hidden lg:block border border-border-color bg-surface p-5 ring-1 ring-border-color">
            <h3 className="font-mono text-[10px] uppercase tracking-[0.24em] text-text-muted mb-3">
              Reviewers
            </h3>
            {grant.reviewers.length === 0 ? (
              <p className="font-mono text-xs text-text-muted">No reviewers assigned.</p>
            ) : (
              <>
                <div className="flex flex-wrap gap-2">
                  {grant.reviewers.map((addr) => (
                    <span
                      key={addr}
                      className="inline-flex border border-border-color bg-bg-secondary px-2 py-1"
                    >
                      <WalletAddress address={addr} />
                    </span>
                  ))}
                </div>
                {quorumRequired > 0 && (
                  <p className="mt-3 font-mono text-xs text-text-muted">
                    Requires {quorumRequired} of {grant.reviewers.length} approvals
                  </p>
                )}
              </>
            )}
          </section>
        </aside>
      </div>

      <FundGrantModal
        grant={grant}
        open={fundModalOpen}
        onClose={() => setFundModalOpen(false)}
        onSuccess={() => {
          void refetch();
          void refetchFunders();
        }}
      />
    </div>
  );
}

export default function GrantDetailClient({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  return (
    <Suspense fallback={<GrantDetailSkeleton />}>
      <GrantDetailContent grantId={id} />
    </Suspense>
  );
}
