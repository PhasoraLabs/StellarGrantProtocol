"use client";

import { useState, useCallback, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
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

/** Returns an error message when the amount cannot be funded, otherwise null. */
export function validateFundAmount(
  amount: bigint,
  token: string,
  balance: WalletBalance | null,
): string | null {
  if (amount <= 0n) return "Amount must be greater than zero";
  if (balance) {
    const available = token === "native" ? balance.xlm : (balance.tokens[token] ?? 0n);
    if (amount > available) return "Amount exceeds your wallet balance";
  }
  return null;
}

export function useFundGrant(): UseFundGrantReturn {
  const { address } = useWalletStore();
  const queryClient = useQueryClient();

  const [txStatus, setTxStatus] = useState<TxStatus>("idle");
  const [txHash, setTxHash] = useState<string | null>(null);
  const [explorerUrl, setExplorerUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [walletBalance, setWalletBalance] = useState<WalletBalance | null>(
    null,
  );
  const [isBalanceLoading, setIsBalanceLoading] = useState(false);

  // Load the connected wallet's own balance
  useEffect(() => {
    const controller = new AbortController();
    const load = async () => {
      if (!address) {
        setWalletBalance(null);
        return;
      }
      setIsBalanceLoading(true);
      try {
        const horizon = getHorizonClient();
        const account = await horizon.loadAccount(address);
        const xlmEntry = account.balances.find(
          (b) => b.asset_type === "native",
        );
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
    async (_params: FundGrantParams): Promise<FundGrantResult> => {
      if (!address) {
        const errMsg = "Wallet not connected";
        setError(errMsg);
        setTxStatus("error");
        toast({
          title: "Connection Required",
          description: errMsg,
          variant: "error",
        });
        throw new Error(errMsg);
      }

      const validationError = validateFundAmount(_params.amount, _params.token, walletBalance);
      if (validationError) {
        setError(validationError);
        setTxStatus("error");
        toast({ title: "Invalid amount", description: validationError, variant: "error" });
        throw new Error(validationError);
      }

      setTxStatus("pending");
      setError(null);
      setTxHash(null);
      setExplorerUrl(null);

      try {
        // TODO: Implement the full Soroban contract call using contractClient when SDK binding is available.
        // This is a placeholder that simulates the proper flow:
        // 1. Build transaction with contractClient.fundGrant(grantId, amount, token)
        // 2. Simulate transaction
        // 3. Assemble with auth
        // 4. Sign via wallet adapter
        // 5. Submit to network
        // 6. Poll for confirmation
        //
        // For now, throw an error to prevent fabricated success states until
        // the real implementation is complete.
        throw new Error(
          "Fund grant transaction not yet implemented. Real Soroban integration required.",
        );

        // Example of what the real implementation will look like:
        // const txParams = await contractClient.fundGrant({
        //   grantId: params.grantId,
        //   amount: params.amount,
        //   token: params.token,
        //   funder: address,
        // });
        //
        // const signedXdr = await walletSign(txParams);
        // const result = await rpcClient.sendTransaction(signedXdr);
        // const hash = result.hash;
        //
        // // Poll for confirmation
        // await pollTransactionStatus(hash);
        //
        // const url = stellarExplorerTx(hash);
        // setTxHash(hash);
        // setExplorerUrl(url);
        // setTxStatus("success");
        //
        // await queryClient.invalidateQueries({ queryKey: ["grant", params.grantId] });
        // await queryClient.invalidateQueries({ queryKey: ["funders", params.grantId] });
        //
        // toast({
        //   title: "Grant funded",
        //   description: `Transaction confirmed: ${hash.slice(0, 10)}...`,
        //   variant: "success",
        //   action: { label: "View on Explorer", href: url },
        // });
        //
        // return { txHash: hash, explorerUrl: url };
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setTxStatus("error");
        toast({
          title: "Funding failed",
          description: message,
          variant: "error",
        });
        throw err;
      }
    },
    [address, queryClient, walletBalance],
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
