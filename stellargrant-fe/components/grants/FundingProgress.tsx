/**
 * FundingProgress Component
 *
 * Animated progress bar showing escrow amount vs target.
 * Displays per-token breakdown when multiple tokens deposited.
 */

"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { formatTokenAmount, getTokenMetadata, TokenMetadata } from "@/lib/tokens";

interface TokenBreakdown {
  token: string;
  amount: bigint;
  metadata?: TokenMetadata;
}

interface FundingProgressProps {
  current: bigint | number;
  target: bigint | number;
  token?: string; // Primary token address
  tokens?: TokenBreakdown[]; // Multiple token breakdown
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

  // Convert to bigint for consistent handling
  const currentBigInt = typeof current === "bigint" ? current : BigInt(Math.round(current));
  const targetBigInt = typeof target === "bigint" ? target : BigInt(Math.round(target));

  // Calculate percentage safely
  const percentage = targetBigInt > 0n
    ? Number((currentBigInt * 100n) / targetBigInt)
    : 0;

  // Fetch metadata for all tokens on mount
  useEffect(() => {
    async function fetchMetadata() {
      setIsLoading(true);
      const metadataMap = new Map<string, TokenMetadata>();

      // Fetch primary token metadata
      if (token) {
        const metadata = await getTokenMetadata(token);
        metadataMap.set(token, metadata);
      }

      // Fetch metadata for all tokens in breakdown
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

  // Format the primary display amount
  const formatDisplayAmount = (amount: bigint, tokenAddress?: string) => {
    if (!tokenAddress) {
      return amount.toString();
    }
    const metadata = tokenMetadataMap.get(tokenAddress);
    const decimals = metadata?.decimals ?? 7;
    const symbol = metadata?.symbol;
    return formatTokenAmount(amount, decimals, { symbol, showSymbol: true });
  };

  // Calculate total target percentage
  const safePercentage = Math.min(percentage, 100);

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
        <span>{safePercentage.toFixed(1)}%</span>
      </div>
      <div className="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
        <motion.div
          className="bg-stellar-cyan h-full rounded-full"
          initial={false}
          animate={{ width: `${safePercentage}%` }}
          transition={{ duration: 0.6, ease: "easeOut" }}
        />
      </div>

      {/* Multi-token breakdown */}
      {showBreakdown && tokens && tokens.length > 0 && (
        <div className="mt-3 space-y-2">
          <p className="text-xs text-gray-500">Funding breakdown:</p>
          {tokens.map((t, index) => {
            const metadata = tokenMetadataMap.get(t.token);
            const decimals = metadata?.decimals ?? 7;
            const symbol = metadata?.symbol ?? "UNKNOWN";
            const formatted = formatTokenAmount(t.amount, decimals, { symbol, showSymbol: true });

            return (
              <div key={`${t.token}-${index}`} className="flex justify-between text-xs">
                <span className="text-gray-600">{symbol}</span>
                <span className="font-medium">{formatted}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
