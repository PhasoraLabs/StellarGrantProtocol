"use client";

/**
 * useMultiSig Hook
 *
 * React hook that wraps MultiSigTracker to manage signature collection
 * and submission state for multi-sig grant transactions.
 *
 * @example
 * ```tsx
 * const { status, addSignature, submitWhenReady, isSubmitting, txHash, error } =
 *   useMultiSig({ unsignedXdr, requiredSignatures: 2, signerPublicKeys });
 *
 * // When a signer's signed XDR arrives:
 * addSignature("GAAA...", signedXdr);
 * ```
 */

import { useState, useEffect, useCallback } from "react";
import {
  MultiSigTracker,
  submitSignedXdr,
  type MultiSigStatus,
  type TransactionXdr,
} from "@/lib/stellar/multisig";

export interface UseMultiSigOptions {
  unsignedXdr: TransactionXdr;
  requiredSignatures: number;
  signerPublicKeys: string[];
}

export interface UseMultiSigResult {
  /** Current signature collection status */
  status: MultiSigStatus;
  /** True when enough signatures are collected to submit */
  isReady: boolean;
  /** True while the combined transaction is being submitted */
  isSubmitting: boolean;
  /** Tx hash after successful submission */
  txHash: string | null;
  /** Error from submission or signature validation */
  error: string | null;
  /** Record a signer's signed XDR */
  addSignature: (publicKey: string, signedXdr: TransactionXdr) => void;
  /** Mark a signer as declined */
  markDeclined: (publicKey: string) => void;
  /** Combine all signatures and submit to the network */
  submitWhenReady: () => Promise<void>;
}

export function useMultiSig(options: UseMultiSigOptions): UseMultiSigResult {
  const [tracker] = useState(() => new MultiSigTracker(options));
  const [status, setStatus] = useState<MultiSigStatus>(
    () => tracker.snapshot
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [txHash, setTxHash] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Subscribe to tracker updates
  useEffect(() => {
    return tracker.onChange((s) => setStatus(s));
  }, [tracker]);

  const addSignature = useCallback((publicKey: string, signedXdr: TransactionXdr) => {
    try {
      tracker.addSignature(publicKey, signedXdr);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [tracker]);

  const markDeclined = useCallback((publicKey: string) => {
    try {
      tracker.markDeclined(publicKey);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [tracker]);

  const submitWhenReady = useCallback(async () => {
    if (!tracker.isReady) {
      setError(`Not enough signatures yet (${status.collectedCount}/${status.requiredSignatures})`);
      return;
    }

    setIsSubmitting(true);
    setError(null);
    try {
      const combinedXdr = tracker.buildCombinedXdr();
      const result = await submitSignedXdr(combinedXdr);
      setTxHash(result.txHash);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSubmitting(false);
    }
  }, [status.collectedCount, status.requiredSignatures, tracker]);

  return {
    status,
    isReady: status.isReady,
    isSubmitting,
    txHash,
    error,
    addSignature,
    markDeclined,
    submitWhenReady,
  };
}
