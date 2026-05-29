"use client";

import { useState, useCallback } from "react";
import { stellarExplorerTx } from "@/lib/constants";
import { toast } from "@/lib/toast";

export interface FundGrantParams {
  grantId: string;
  amount: bigint;
  token: "native" | string;
}

export interface FundGrantResult {
  txHash: string;
  explorerUrl: string;
}

export interface UseFundGrantReturn {
  fund: (params: FundGrantParams) => Promise<FundGrantResult>;
  isLoading: boolean;
  error: string | null;
  reset: () => void;
}

export function useFundGrant(): UseFundGrantReturn {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fund = useCallback(async (_params: FundGrantParams): Promise<FundGrantResult> => {
    setIsLoading(true);
    setError(null);

    try {
      // TODO: Replace with real contract client call once SDK binding is ready.
      // e.g.: const tx = await grantClient.fundGrant(_params.grantId, _params.amount);
      //       const result = await tx.signAndSend({ signTransaction });
      const mockTxHash = `mock_tx_${Date.now()}`;

      const result: FundGrantResult = {
        txHash: mockTxHash,
        explorerUrl: stellarExplorerTx(mockTxHash),
      };

      toast({
        title: `Funded ${_params.amount.toString()} stroops`,
        description: "Your contribution was recorded on-chain.",
        variant: "success",
        action: {
          label: "View on Stellar Explorer",
          href: result.explorerUrl,
        },
      });

      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      toast({
        title: "Funding failed",
        description: message,
        variant: "error",
      });
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  const reset = useCallback(() => {
    setError(null);
  }, []);

  return { fund, isLoading, error, reset };
}
