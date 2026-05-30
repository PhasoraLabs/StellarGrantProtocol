"use client";

import { useState, useCallback, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { stellarExplorerTx } from "@/lib/constants";
import { toast } from "@/lib/toast";
import { useWalletStore } from "@/lib/store/walletStore";
import { getHorizonClient } from "@/lib/stellar/client";

export type TxStatus = "idle" | "pending" | "success" | "error";

export interface FundGrantParams {
  grantId: string;
  amount: bigint;
  token: "native" | string;
}

export interface FundGrantResult {
  txHash: string;
  explorerUrl: string;
}

export interface WalletBalance {
  xlm: bigint;
  tokens: Record<string, bigint>;
}

export interface UseFundGrantReturn {
  fund: (params: FundGrantParams) => Promise<FundGrantResult>;
  txStatus: TxStatus;
  txHash: string | null;
  explorerUrl: string | null;
  error: string | null;
  reset: () => void;
  walletBalance: WalletBalance | null;
  isBalanceLoading: boolean;
}

const STROOP = 10_000_000n;

function parseBalance(raw: string): bigint {
  const [whole = "0", frac = ""] = raw.split(".");
  const paddedFrac = frac.padEnd(7, "0").slice(0, 7);
  return BigInt(whole) * STROOP + BigInt(paddedFrac);
}

export function useFundGrant(): UseFundGrantReturn {
  const { address } = useWalletStore();
  const queryClient = useQueryClient();

  const [txStatus, setTxStatus] = useState<TxStatus>("idle");
  const [txHash, setTxHash] = useState<string | null>(null);
  const [explorerUrl, setExplorerUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [walletBalance, setWalletBalance] = useState<WalletBalance | null>(null);
  const [isBalanceLoading, setIsBalanceLoading] = useState(false);

  // Load the connected wallet's own balance
  useEffect(() => {
    if (!address) {
      setWalletBalance(null);
      return;
    }
    const controller = new AbortController();
    const load = async () => {
      setIsBalanceLoading(true);
      try {
        const horizon = getHorizonClient();
        const account = await horizon.loadAccount(address);
        const xlmEntry = account.balances.find((b) => b.asset_type === "native");
        const xlm = xlmEntry ? parseBalance(xlmEntry.balance) : 0n;
        const tokens: Record<string, bigint> = {};
        for (const b of account.balances) {
          if (b.asset_type !== "native") {
            const code = (b as { asset_code: string }).asset_code;
            tokens[code] = parseBalance(b.balance);
          }
        }
        if (!controller.signal.aborted) setWalletBalance({ xlm, tokens });
      } catch {
        if (!controller.signal.aborted) setWalletBalance(null);
      } finally {
        if (!controller.signal.aborted) setIsBalanceLoading(false);
      }
    };
    void load();
    return () => controller.abort();
  }, [address]);

  const fund = useCallback(
    async (params: FundGrantParams): Promise<FundGrantResult> => {
      setTxStatus("pending");
      setError(null);
      setTxHash(null);
      setExplorerUrl(null);

      try {
        // TODO: Replace with real Soroban contract call when SDK binding is ready.
        // e.g.: const tx = await grantClient.fundGrant(params.grantId, params.amount);
        //       const result = await tx.signAndSend({ signTransaction });
        const mockHash = `mock_tx_${Date.now()}`;
        const url = stellarExplorerTx(mockHash);

        setTxHash(mockHash);
        setExplorerUrl(url);
        setTxStatus("success");

        // Invalidate grant queries so balances/funded amount refresh
        await queryClient.invalidateQueries({ queryKey: ["grant", params.grantId] });
        await queryClient.invalidateQueries({ queryKey: ["funders", params.grantId] });

        toast({
          title: "Grant funded",
          description: `Contributed ${params.amount.toString()} stroops.`,
          variant: "success",
          action: { label: "View on Explorer", href: url },
        });

        return { txHash: mockHash, explorerUrl: url };
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setTxStatus("error");
        toast({ title: "Funding failed", description: message, variant: "error" });
        throw err;
      }
    },
    [queryClient]
  );

  const reset = useCallback(() => {
    setTxStatus("idle");
    setTxHash(null);
    setExplorerUrl(null);
    setError(null);
  }, []);

  return {
    fund,
    txStatus,
    txHash,
    explorerUrl,
    error,
    reset,
    walletBalance,
    isBalanceLoading,
  };
}