/**
 * FundingProgress Component
 *
 * Animated progress bar showing escrow amount vs target.
 * Displays per-token breakdown when multiple tokens deposited.
 * Uses design-system tokens — no generic Tailwind colours.
 */

"use client";

import { useEffect, useState } from "react";
import { formatTokenAmount, getTokenMetadata, TokenMetadata } from "@/lib/tokens";

interface TokenBreakdown {
  token: string;
  amount: bigint;
  metadata?: TokenMetadata;
}

interface FundingProgressProps {
  current: bigint | number;
  target: bigint | number;
  token?: string;
  tokens?: TokenBreakdown[];
  showBreakdown?: boolean;
}

export function FundingProgress({
  current,
  target,
  token,
  tokens,
  showBreakdown = true,
}: FundingProgressProps) {
  const [tokenMetadataMap, setTokenMetadataMap] = useState<Map<string, TokenMetadata>>(new Map());
  const [isLoading, setIsLoading] = useState(true);

  const currentBigInt = typeof current === "bigint" ? current : BigInt(Math.round(current));
  const targetBigInt = typeof target === "bigint" ? target : BigInt(Math.round(target));

  const percentage = targetBigInt > 0n
    ? Number((currentBigInt * 100n) / targetBigInt)
    : 0;

  const safePercentage = Math.min(percentage, 100);

  useEffect(() => {
    async function fetchMetadata() {
      setIsLoading(true);
      const metadataMap = new Map<string, TokenMetadata>();

      if (token) {
        const metadata = await getTokenMetadata(token);
        metadataMap.set(token, metadata);
      }

      if (tokens) {
        for (const t of tokens) {
          if (!metadataMap.has(t.token)) {
            const metadata = await getTokenMetadata(t.token);
            metadataMap.set(t.token, metadata);
          }
        }
      }

      setTokenMetadataMap(metadataMap);
      setIsLoading(false);
    }

    fetchMetadata();
  }, [token, tokens]);

  const formatDisplayAmount = (amount: bigint, tokenAddress?: string) => {
    if (!tokenAddress) return amount.toString();
    const metadata = tokenMetadataMap.get(tokenAddress);
    const decimals = metadata?.decimals ?? 7;
    const symbol = metadata?.symbol;
    return formatTokenAmount(amount, decimals, { symbol, showSymbol: true });
  };

  return (
    <div className="w-full">
      <div className="flex justify-between text-sm mb-2">
        <span className="font-medium">
          {isLoading ? (
            <span className="animate-pulse">Loading...</span>
          ) : (
            formatDisplayAmount(currentBigInt, token)
          )}{" "}
          /{" "}
          {formatDisplayAmount(targetBigInt, token)}
        </span>
        <span className="text-text-muted">{safePercentage.toFixed(1)}%</span>
      </div>

      {/* Progress track — dark bg, amber fill, CSS transition */}
      <div className="w-full bg-bg-secondary h-1.5 overflow-hidden">
        <div
          className="bg-accent-primary h-1.5 transition-[width] duration-700 ease-out"
          style={{ width: `${safePercentage}%` }}
        />
      </div>

      {/* Multi-token breakdown */}
      {showBreakdown && tokens && tokens.length > 0 && (
        <div className="mt-3 space-y-2">
          <p className="text-xs text-text-muted">Funding breakdown:</p>
          {tokens.map((t, index) => {
            const metadata = tokenMetadataMap.get(t.token);
            const decimals = metadata?.decimals ?? 7;
            const symbol = metadata?.symbol ?? "UNKNOWN";
            const formatted = formatTokenAmount(t.amount, decimals, { symbol, showSymbol: true });

            return (
              <div key={`${t.token}-${index}`} className="flex justify-between text-xs">
                <span className="text-text-muted">{symbol}</span>
                <span className="font-medium">{formatted}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
