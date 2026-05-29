/**
 * useContractTransaction Hook
 * 
 * Generic hook for building, simulating, signing, and submitting
 * Soroban contract transactions. Manages loading, error, and success
 * state for the full transaction lifecycle.
 */

"use client";

import { useState } from "react";
import {
  TransactionBuilder,
  Contract,
  Account,
  BASE_FEE,
  xdr,
} from "@stellar/stellar-sdk";
import { rpc as SorobanRpc } from "@stellar/stellar-sdk";
import { rpcClient, networkPassphraseConfig } from "@/lib/stellar/client";
import { CONTRACT_ID, stellarExplorerTx } from "@/lib/constants";
import { toast } from "@/lib/toast";
import { useWallet } from "./useWallet";

interface ExecuteOptions {
  method: string;
  args: xdr.ScVal[];
  onSuccess?: (txHash: string) => void;
  onError?: (error: Error) => void;
}

interface UseContractTransactionResult {
  execute: (options: ExecuteOptions) => Promise<string | null>;
  isPending: boolean;
  isSimulating: boolean;
  isSuccess: boolean;
  txHash: string | null;
  error: string | null;
  reset: () => void;
}

const MAX_RETRIES = 3;
const RETRY_DELAY_MS = 5000;
const POLL_INTERVAL_MS = 2000;
const POLL_TIMEOUT_MS = 60000;

export function useContractTransaction(): UseContractTransactionResult {
  const { address, signTransaction: walletSign } = useWallet();
  const [isPending, setIsPending] = useState(false);
  const [isSimulating, setIsSimulating] = useState(false);
  const [isSuccess, setIsSuccess] = useState(false);
  const [txHash, setTxHash] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reset = () => {
    setIsPending(false);
    setIsSimulating(false);
    setIsSuccess(false);
    setTxHash(null);
    setError(null);
  };

  const execute = async (options: ExecuteOptions): Promise<string | null> => {
    const { method, args, onSuccess, onError } = options;

    if (!address) {
      const err = new Error("Wallet not connected");
      setError(err.message);
      onError?.(err);
      return null;
    }

    reset();
    setIsSimulating(true);

    try {
      // Step 1: Build the operation XDR
      const contract = new Contract(CONTRACT_ID);
      const account = await rpcClient.getAccount(address);
      const sourceAccount = new Account(account.accountId(), account.sequenceNumber());

      const operation = contract.call(method, ...args);

      let transaction = new TransactionBuilder(sourceAccount, {
        fee: BASE_FEE,
        networkPassphrase: networkPassphraseConfig,
      })
        .addOperation(operation)
        .setTimeout(180)
        .build();

      // Step 2: Simulate
      const simulationResult = await rpcClient.simulateTransaction(transaction);

      if (SorobanRpc.Api.isSimulationError(simulationResult)) {
        throw new Error(`Simulation failed: ${simulationResult.error}`);
      }

      setIsSimulating(false);

      // Step 3: Prepare (assemble with footprint and auth)
      transaction = SorobanRpc.assembleTransaction(transaction, simulationResult).build();

      // Step 4: Sign
      const preparedXdr = transaction.toXDR();
      let signedXdr: string;
      try {
        signedXdr = await walletSign(preparedXdr);
      } catch (_err) {
        toast({
          title: "Transaction rejected",
          description: "You rejected the signing request.",
          variant: "warning",
        });
        throw new Error("Transaction rejected by user");
      }

      setIsPending(true);

      // Step 5: Submit with retry logic
      let sendResult: SorobanRpc.Api.SendTransactionResponse | null = null;
      for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
        sendResult = await rpcClient.sendTransaction(
          TransactionBuilder.fromXDR(signedXdr, networkPassphraseConfig)
        );

        if (sendResult.status !== "TRY_AGAIN_LATER") {
          break;
        }

        if (attempt < MAX_RETRIES - 1) {
          await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY_MS));
        }
      }

      if (!sendResult || sendResult.status === "ERROR") {
        throw new Error(`Transaction submission failed: ${sendResult?.errorResult?.toXDR("base64")}`);
      }

      const hash = sendResult.hash;
      setTxHash(hash);

      // Step 6: Confirm (poll)
      const startTime = Date.now();
      while (Date.now() - startTime < POLL_TIMEOUT_MS) {
        const txResult = await rpcClient.getTransaction(hash);

        if (txResult.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
          setIsSuccess(true);
          setIsPending(false);
          onSuccess?.(hash);

          toast({
            title: "Transaction confirmed",
            variant: "success",
            action: {
              label: "View on Stellar Explorer",
              href: stellarExplorerTx(hash),
            },
          });

          return hash;
        }

        if (txResult.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
          throw new Error("Transaction failed on-chain");
        }

        await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
      }

      throw new Error("Transaction confirmation timeout");
    } catch (_err) {
      const message = _err instanceof Error ? _err.message : "Transaction failed";
      setError(message);
      setIsPending(false);
      setIsSimulating(false);
      onError?.(_err instanceof Error ? _err : new Error(message));

      toast({
        title: "Transaction failed",
        description: message,
        variant: "error",
      });

      return null;
    }
  };

  return {
    execute,
    isPending,
    isSimulating,
    isSuccess,
    txHash,
    error,
    reset,
  };
}
