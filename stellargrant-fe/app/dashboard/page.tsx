"use client";

/**
 * Dashboard Page
 *
 * Wallet-guarded overview for the connected user. Displays:
 *   - WalletInfo card (address, XLM balance, reputation, network)
 *   - Four tabs controlled by ?tab= URL param:
 *       my-grants     — grants the user created
 *       funding       — grants the user has funded
 *       reviewing     — grants the user is reviewing
 *       watching      — bookmarked grants (local watchlist)
 */

import { useEffect, useState, useCallback, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import Link from "next/link";
import { motion, AnimatePresence, useReducedMotion } from "framer-motion";
import { useWalletStore } from "@/lib/store/walletStore";
import { WalletInfo } from "@/components/wallet/WalletInfo";
import { WalletConnect } from "@/components/wallet/WalletConnect";
import { GrantCard, grantListVariants, grantCardVariants } from "@/components/grants/GrantCard";
import { WatchedGrantsPanel } from "@/components/grants/WatchedGrantsPanel";
import type { Grant } from "@/types";

// ── Types ─────────────────────────────────────────────────────────────────────

type Tab = "my-grants" | "funding" | "reviewing" | "watching";

const TABS: { id: Tab; label: string }[] = [
  { id: "my-grants", label: "My Grants" },
  { id: "funding", label: "Grants I Fund" },
  { id: "reviewing", label: "Grants I Review" },
  { id: "watching", label: "Watching" },
];

// ── Helpers ───────────────────────────────────────────────────────────────────

function isValidTab(value: string | null): value is Tab {
  return (
    value === "my-grants" ||
    value === "funding" ||
    value === "reviewing" ||
    value === "watching"
  );
}

// ── Dashboard inner (needs useSearchParams so wrapped in Suspense) ─────────────

function DashboardContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const rawTab = searchParams.get("tab");
  const activeTab: Tab = isValidTab(rawTab) ? rawTab : "my-grants";

  const address = useWalletStore((s) => s.address);
  const network = useWalletStore((s) => s.network);

  const prefersReduced = useReducedMotion();
  const [balance, setBalance] = useState<bigint | null>(null);
  const [reputation, setReputation] = useState<number | null>(null);
  const [grants, setGrants] = useState<Grant[]>([]);
  const [loading, setLoading] = useState(false);

  // Load wallet info (balance + reputation) when address changes
  useEffect(() => {
    if (!address) return;

    setBalance(null);
    setReputation(null);

    const controller = new AbortController();

    async function loadWalletInfo() {
      try {
        const baseUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";
        const res = await fetch(`${baseUrl}/wallet/${address}/info`, {
          signal: controller.signal,
          cache: "no-store",
        });
        if (res.ok) {
          const data = await res.json() as { balance?: number; reputation?: number };
          if (typeof data.balance === "number") setBalance(BigInt(Math.round(data.balance)));
          if (typeof data.reputation === "number") setReputation(data.reputation);
        }
      } catch {
        // API unavailable — leave as null (shimmer stays visible)
      }
    }

    void loadWalletInfo();
    return () => controller.abort();
  }, [address]);

  // Load grants for the active tab
  const fetchGrants = useCallback(async (tab: Tab, addr: string) => {
    setLoading(true);
    setGrants([]);
    try {
      const baseUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";
      const endpoint =
        tab === "my-grants"
          ? `/grants?owner=${addr}`
          : tab === "funding"
          ? `/grants?funder=${addr}`
          : `/grants?reviewer=${addr}`;
      const res = await fetch(`${baseUrl}${endpoint}`, { cache: "no-store" });
      if (res.ok) {
        const data = await res.json() as { grants?: Grant[] };
        setGrants(data.grants ?? []);
      }
    } catch {
      // API unavailable — show empty state
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!address || activeTab === "watching") return;
    void fetchGrants(activeTab, address);
  }, [activeTab, address, fetchGrants]);

  function setTab(tab: Tab) {
    const params = new URLSearchParams(searchParams.toString());
    params.set("tab", tab);
    router.replace(`/dashboard?${params.toString()}`, { scroll: false });
  }

  // ── Not connected (wallet tabs only) ───────────────────────────────────────
  if (!address && activeTab !== "watching") {
    return (
      <div className="min-h-screen flex flex-col items-center justify-center gap-6 px-4">
        <h1 className="text-2xl font-bold">Connect your wallet</h1>
        <p className="text-text-muted text-sm max-w-xs text-center">
          Your dashboard is personalised. Connect a wallet to view your grants, funding activity,
          and review assignments — or open the Watching tab for saved grants.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Link
            href="/dashboard?tab=watching"
            className="font-mono text-xs uppercase tracking-wider text-accent-secondary hover:underline"
          >
            View watchlist →
          </Link>
          <WalletConnect />
        </div>
      </div>
    );
  }

  // ── Connected / watchlist ──────────────────────────────────────────────────
  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl space-y-6">
      {/* Header */}
      <div>
        <p className="font-mono text-xs uppercase tracking-[0.32em] text-accent-secondary">
          Dashboard
        </p>
        <h1 className="mt-2 text-3xl font-bold">My Account</h1>
      </div>

      {/* Wallet info card */}
      {address && (
        <WalletInfo
          address={address}
          network={network}
          balance={balance}
          reputation={reputation}
        />
      )}

      {/* Tab bar */}
      <div className="flex gap-1 border-b border-border-color" role="tablist">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            onClick={() => setTab(tab.id)}
            className={[
              "px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px",
              activeTab === tab.id
                ? "border-accent-primary text-accent-primary"
                : "border-transparent text-text-muted hover:text-text-primary",
            ].join(" ")}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content — AnimatePresence cross-fade on tab switch */}
      <AnimatePresence mode="wait">
        <motion.div
          key={activeTab}
          role="tabpanel"
          aria-label={TABS.find((t) => t.id === activeTab)?.label}
          initial={prefersReduced ? { opacity: 0 } : { opacity: 0, y: 8 }}
          animate={prefersReduced ? { opacity: 1 } : { opacity: 1, y: 0 }}
          exit={{ opacity: 0 }}
          transition={{ duration: prefersReduced ? 0.1 : 0.2, ease: "easeOut" as const }}
        >
          <AnimatePresence mode="wait">
            {activeTab === "watching" ? (
              <motion.div
                key="watching"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
              >
                <WatchedGrantsPanel />
              </motion.div>
            ) : loading ? (
              <motion.div
                key="loading"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="space-y-3"
              >
                {[1, 2, 3].map((n) => (
                  <div
                    key={n}
                    className="h-28 rounded-[4px] border border-border-color animate-pulse bg-surface/40"
                  />
                ))}
              </motion.div>
            ) : grants.length === 0 ? (
              <motion.div
                key="empty"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="rounded-[4px] border border-border-color bg-surface/60 p-10 text-center space-y-3"
              >
                <p className="text-text-muted text-sm">
                  {activeTab === "my-grants" && "You haven't created any grants yet."}
                  {activeTab === "funding" && "You haven't funded any grants yet."}
                  {activeTab === "reviewing" && "You have no grants assigned for review."}
                </p>
                {activeTab === "my-grants" && (
                  <Link
                    href="/grants/new"
                    className="inline-block text-sm text-accent-primary hover:underline"
                  >
                    Create your first grant →
                  </Link>
                )}
              </motion.div>
            ) : (
              <motion.div
                key="list"
                className="space-y-3"
                variants={grantListVariants}
                initial={prefersReduced ? {} : "hidden"}
                animate="visible"
              >
                {grants.map((grant) => (
                  <motion.div
                    key={grant.id}
                    variants={prefersReduced ? {} : grantCardVariants}
                  >
                    <GrantCard
                      grant={{
                        id: Number(grant.id),
                        title: grant.title,
                        status: grant.status,
                        funded: grant.funded,
                        budget: grant.budget,
                        deadline: grant.deadline,
                        token: grant.token,
                        owner: grant.owner,
                      }}
                    />
                  </motion.div>
                ))}
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      </AnimatePresence>
    </div>
  );
}

// ── Page export ───────────────────────────────────────────────────────────────

export default function DashboardPage() {
  return (
    <Suspense>
      <DashboardContent />
    </Suspense>
  );
}
