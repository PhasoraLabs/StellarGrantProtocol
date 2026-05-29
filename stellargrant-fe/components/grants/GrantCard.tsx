/**
 * GrantCard Component
 *
 * Compact card for grant listing pages. Shows title, status badge,
 * funding progress, deadline, and token. Includes a hover-lift animation
 * via framer-motion that respects the user's reduced-motion preference.
 */

"use client";

import { useEffect, useState, type MouseEvent } from "react";
import { motion, useReducedMotion } from "framer-motion";
import { formatTokenAmount, getTokenMetadata, TokenMetadata } from "@/lib/tokens";
import { GrantStatusBadge } from "./GrantStatusBadge";
import { FundingProgress } from "./FundingProgress";

interface GrantCardProps {
  grant: {
    id: number;
    title: string;
    status: number;
    funded: bigint | number;
    budget: bigint | number;
    deadline: bigint | number;
    token?: string;
    owner?: string;
  };
  onClick?: () => void;
  showOwner?: boolean;
  compact?: boolean;
  showWatchlistBadge?: boolean;
  watchlistGrantId?: string;
  onRemoveFromWatchlist?: () => void;
}

/** Stagger variant for parent list containers */
export const grantListVariants = {
  hidden: {},
  visible: {
    transition: { staggerChildren: 0.06 },
  },
};

/** Item variant consumed by each GrantCard in a staggered list */
export const grantCardVariants = {
  hidden: { opacity: 0, y: 16 },
  visible: { opacity: 1, y: 0, transition: { duration: 0.3, ease: "easeOut" as const } },
};

export function GrantCard({
  grant,
  onClick,
  showOwner = false,
  compact = false,
  showWatchlistBadge = false,
  watchlistGrantId,
  onRemoveFromWatchlist,
}: GrantCardProps) {
  const prefersReduced = useReducedMotion();
  const [tokenMetadata, setTokenMetadata] = useState<TokenMetadata | null>(null);

  useEffect(() => {
    async function fetchMetadata() {
      if (grant.token) {
        const metadata = await getTokenMetadata(grant.token);
        setTokenMetadata(metadata);
      }
    }
    fetchMetadata();
  }, [grant.token]);

  const decimals = tokenMetadata?.decimals ?? 7;
  const symbol = tokenMetadata?.symbol ?? (grant.token ? "UNKNOWN" : "XLM");

  const fundedFormatted = formatTokenAmount(grant.funded, decimals, { symbol, showSymbol: true });
  const budgetFormatted = formatTokenAmount(grant.budget, decimals, { symbol, showSymbol: true });

  const deadlineDate = typeof grant.deadline === "bigint"
    ? new Date(Number(grant.deadline) * 1000)
    : new Date(grant.deadline);

  const handleRemove = (event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    onRemoveFromWatchlist?.();
  };

  return (
    <motion.div
      className="group relative border rounded-lg p-4 cursor-pointer bg-white"
      onClick={onClick}
      whileHover={prefersReduced ? {} : { y: -3, boxShadow: "0 8px 24px rgba(0,0,0,0.12)" }}
      transition={{ duration: 0.18, ease: "easeOut" }}
      variants={grantCardVariants}
    >
      {showWatchlistBadge && (
        <div className="absolute right-3 top-3 z-10">
          <span
            className="block font-mono text-sm text-accent-primary transition-opacity group-hover:opacity-0"
            aria-hidden="true"
          >
            ★
          </span>
          <button
            type="button"
            onClick={handleRemove}
            className="absolute right-0 top-0 hidden max-w-[9rem] rounded-none border border-accent-primary/40 bg-bg-primary/95 px-2 py-1 text-left font-mono text-[10px] uppercase tracking-wider text-accent-primary group-hover:block"
            aria-label={`Remove grant ${watchlistGrantId ?? grant.id} from watchlist`}
          >
            Remove from watchlist
          </button>
        </div>
      )}

      <div className="flex justify-between items-start mb-3 gap-2">
        <h3 className="text-xl font-semibold flex-1 pr-6">{grant.title}</h3>
        <GrantStatusBadge status={grant.status} />
      </div>

      {!compact && (
        <>
          <FundingProgress
            current={grant.funded}
            target={grant.budget}
            token={grant.token}
            showBreakdown={false}
          />

          <div className="mt-4 flex justify-between text-sm text-gray-600">
            <span>
              Target: <span className="font-medium">{budgetFormatted}</span>
            </span>
            <span>
              Deadline:{" "}
              <span className="font-medium">
                {deadlineDate.toLocaleDateString()}
              </span>
            </span>
          </div>

          {showOwner && grant.owner && (
            <div className="mt-2 text-xs text-gray-500">
              Owner:{" "}
              <span className="font-mono">
                {grant.owner.slice(0, 8)}...{grant.owner.slice(-8)}
              </span>
            </div>
          )}
        </>
      )}

      {compact && (
        <div className="mt-2 text-sm text-gray-600">
          <span className="font-medium">{fundedFormatted}</span> raised
        </div>
      )}
    </motion.div>
  );
}
