"use client";

/**
 * WalletInfo Component
 *
 * Displays a summary card for the connected wallet:
 *   - Truncated address with clipboard copy (via WalletAddress)
 *   - XLM balance tile with shimmer while loading
 *   - Reputation score tile (green ≥ 75 / amber ≥ 40 / red < 40)
 *   - Network badge (testnet / mainnet / futurenet)
 *
 * Props:
 *   address     — connected Stellar address
 *   network     — active network name
 *   balance     — XLM balance (stroops as bigint, or null while loading)
 *   reputation  — numeric score 0–100 (or null while loading)
 */

import { WalletAddress } from "./WalletAddress";

interface WalletInfoProps {
  address: string;
  network: "testnet" | "mainnet" | "futurenet";
  /** XLM balance in stroops. Pass null to show a loading shimmer. */
  balance: bigint | null;
  /** Reputation score 0–100. Pass null to show a loading shimmer. */
  reputation: number | null;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatXLM(stroops: bigint): string {
  const xlm = Number(stroops) / 1e7;
  return xlm.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function reputationColor(score: number): string {
  if (score >= 75) return "text-green-400 border-green-700/40 bg-green-900/20";
  if (score >= 40) return "text-amber-400 border-amber-700/40 bg-amber-900/20";
  return "text-red-400 border-red-700/40 bg-red-900/20";
}

function reputationLabel(score: number): string {
  if (score >= 75) return "Excellent";
  if (score >= 40) return "Moderate";
  return "Low";
}

const NETWORK_BADGE: Record<WalletInfoProps["network"], string> = {
  testnet: "border-accent-secondary/50 text-accent-secondary",
  mainnet: "border-green-600/50 text-green-400",
  futurenet: "border-purple-600/50 text-purple-400",
};

// ── Sub-components ────────────────────────────────────────────────────────────

function Shimmer({ className = "" }: { className?: string }) {
  return (
    <span
      className={`block animate-pulse rounded bg-surface/60 ${className}`}
      aria-hidden="true"
    />
  );
}

// ── Component ─────────────────────────────────────────────────────────────────

export function WalletInfo({ address, network, balance, reputation }: WalletInfoProps) {
  return (
    <div className="rounded-[4px] border border-border-color bg-surface/80 p-5 space-y-4">
      {/* Address row */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <WalletAddress address={address} showCopyIcon showAvatar />

        {/* Network badge */}
        <span
          className={`font-mono text-[11px] uppercase tracking-[0.24em] border px-2 py-0.5 rounded-sm ${NETWORK_BADGE[network]}`}
        >
          {network}
        </span>
      </div>

      {/* Tiles row */}
      <div className="grid grid-cols-2 gap-3">
        {/* XLM balance tile */}
        <div className="rounded-[4px] border border-border-color bg-surface/40 p-4">
          <p className="font-mono text-[11px] uppercase tracking-[0.24em] text-text-muted mb-2">
            XLM Balance
          </p>
          {balance === null ? (
            <Shimmer className="h-6 w-24" />
          ) : (
            <p className="text-xl font-bold text-text-primary">
              {formatXLM(balance)}{" "}
              <span className="text-sm font-normal text-text-muted">XLM</span>
            </p>
          )}
        </div>

        {/* Reputation tile */}
        <div className="rounded-[4px] border border-border-color bg-surface/40 p-4">
          <p className="font-mono text-[11px] uppercase tracking-[0.24em] text-text-muted mb-2">
            Reputation
          </p>
          {reputation === null ? (
            <Shimmer className="h-6 w-20" />
          ) : (
            <div className={`inline-flex items-center gap-2 px-2.5 py-1 rounded-sm border text-sm font-bold ${reputationColor(reputation)}`}>
              <span>{reputation}</span>
              <span className="font-normal text-xs">{reputationLabel(reputation)}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
