"use client";

import { useCallback, useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X } from "lucide-react";
import { FundingProgress } from "@/components/grants/FundingProgress";
import { useWalletStore } from "@/lib/store/walletStore";
import { useFundGrant } from "@/hooks/useFundGrant";
import { stellarExplorerTx } from "@/lib/constants";
import type { Grant } from "@/types";

const STROOP = 10_000_000n;

function xlmToStroops(xlm: string): bigint {
  const n = parseFloat(xlm);
  if (isNaN(n) || n <= 0) return 0n;
  return BigInt(Math.round(n * Number(STROOP)));
}

function stroopsToXlm(stroops: bigint): string {
  return (Number(stroops) / Number(STROOP)).toLocaleString("en-US", {
    minimumFractionDigits: 0,
    maximumFractionDigits: 7,
  });
}

interface FundGrantModalProps {
  grant: Grant;
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export function FundGrantModal({ grant, open, onClose, onSuccess }: FundGrantModalProps) {
  const { address } = useWalletStore();
  const { fund, isLoading, error: fundError, reset: resetFund } = useFundGrant();
  const [amountInput, setAmountInput] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [successTx, setSuccessTx] = useState<string | null>(null);

  const unfunded = grant.budget - grant.funded;

  useEffect(() => {
    if (!open) {
      setAmountInput("");
      setValidationError(null);
      setSuccessTx(null);
      resetFund();
    }
  }, [open, resetFund]);

  const validate = useCallback((): boolean => {
    const stroops = xlmToStroops(amountInput);
    if (stroops <= 0n) {
      setValidationError("Enter a valid amount.");
      return false;
    }
    if (stroops > unfunded) {
      setValidationError(`Exceeds remaining balance (${stroopsToXlm(unfunded)} XLM).`);
      return false;
    }
    setValidationError(null);
    return true;
  }, [amountInput, unfunded]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;
    try {
      const result = await fund({
        grantId: grant.id,
        amount: xlmToStroops(amountInput),
        token: "native",
      });
      setSuccessTx(result.txHash);
      onSuccess?.();
    } catch {
      /* fundError set in hook */
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-40 bg-bg-primary/70"
            onClick={onClose}
            aria-hidden
          />
          <motion.div
            role="dialog"
            aria-modal="true"
            aria-labelledby="fund-grant-title"
            initial={{ opacity: 0, scale: 0.96, y: 12 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 12 }}
            className="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 border border-border-color bg-surface p-6 ring-1 ring-border-color"
          >
            <div className="flex items-start justify-between gap-4 mb-4">
              <h2 id="fund-grant-title" className="font-orbitron text-lg text-text-primary">
                Fund This Grant
              </h2>
              <button
                type="button"
                onClick={onClose}
                className="text-text-muted hover:text-text-primary"
                aria-label="Close"
              >
                <X size={18} />
              </button>
            </div>

            <FundingProgress
              current={grant.funded}
              target={grant.budget}
              token={grant.token}
            />

            {successTx ? (
              <div className="mt-6 border border-success/40 bg-success/10 p-4 text-center">
                <p className="font-orbitron text-sm text-success mb-2">Funding successful</p>
                <a
                  href={stellarExplorerTx(successTx)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-mono text-xs text-accent-secondary underline"
                >
                  View transaction
                </a>
              </div>
            ) : !address ? (
              <p className="mt-6 font-mono text-sm text-text-muted text-center">
                Connect your wallet to fund this grant.
              </p>
            ) : (
              <form onSubmit={handleSubmit} className="mt-6 space-y-4">
                <div>
                  <label
                    htmlFor="fund-amount"
                    className="block font-mono text-[10px] uppercase tracking-wider text-text-muted mb-2"
                  >
                    Amount (XLM)
                  </label>
                  <input
                    id="fund-amount"
                    type="number"
                    min="0.0000001"
                    step="any"
                    value={amountInput}
                    onChange={(e) => {
                      setAmountInput(e.target.value);
                      setValidationError(null);
                      resetFund();
                    }}
                    className="w-full border border-border-color bg-bg-secondary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent-primary"
                    placeholder="0.00"
                  />
                  {(validationError || fundError) && (
                    <p className="mt-1 font-mono text-xs text-danger">
                      {validationError ?? fundError}
                    </p>
                  )}
                </div>
                <button
                  type="submit"
                  disabled={isLoading || !amountInput}
                  className="w-full font-orbitron text-sm font-bold uppercase tracking-wider bg-accent-primary text-bg-primary py-3 disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90 transition-opacity"
                >
                  {isLoading ? "Processing…" : "Fund Grant"}
                </button>
              </form>
            )}
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
