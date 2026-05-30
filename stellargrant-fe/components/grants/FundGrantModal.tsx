"use client";

import { useCallback, useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { motion, AnimatePresence } from "framer-motion";
import { X, ExternalLink, RefreshCw, Loader2 } from "lucide-react";
import { FundingProgress } from "@/components/grants/FundingProgress";
import { useWalletStore } from "@/lib/store/walletStore";
import { useFundGrant } from "@/hooks/useFundGrant";
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

function buildSchema(maxStroops: bigint) {
  return z.object({
    amount: z
      .string()
      .min(1, "Enter an amount")
      .refine((v) => !isNaN(parseFloat(v)) && parseFloat(v) > 0, {
        message: "Amount must be greater than 0",
      })
      .refine(
        (v) => xlmToStroops(v) <= maxStroops,
        { message: `Exceeds remaining (${stroopsToXlm(maxStroops)} XLM)` }
      )
      .refine(
        (v) => {
          const stroops = xlmToStroops(v);
          return stroops >= 1n;
        },
        { message: "Amount too small (min 0.0000001 XLM)" }
      ),
  });
}

type FundFormValues = { amount: string };

interface FundGrantModalProps {
  grant: Grant;
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

const AUTO_CLOSE_MS = 3000;

export function FundGrantModal({ grant, open, onClose, onSuccess }: FundGrantModalProps) {
  const { address } = useWalletStore();
  const {
    fund,
    txStatus,
    txHash,
    explorerUrl,
    error: fundError,
    reset: resetFund,
    walletBalance,
    isBalanceLoading,
  } = useFundGrant();

  const unfunded = grant.budget - grant.funded;
  const walletXlm = walletBalance?.xlm ?? 0n;
  const maxFundable = walletXlm < unfunded ? walletXlm : unfunded;

  const schema = buildSchema(unfunded);

  const {
    register,
    handleSubmit,
    setValue,
    watch,
    reset: resetForm,
    formState: { errors, isSubmitting },
  } = useForm<FundFormValues>({
    resolver: zodResolver(schema),
    defaultValues: { amount: "" },
  });

  const amountValue = watch("amount");
  const amountStroops = amountValue ? xlmToStroops(amountValue) : 0n;
  const contributionPct =
    grant.budget > 0n
      ? Math.min(100, Number((amountStroops * 100n) / grant.budget))
      : 0;

  // Reset everything when modal closes
  useEffect(() => {
    if (!open) {
      resetForm();
      resetFund();
    }
  }, [open, resetForm, resetFund]);

  // Auto-close after success
  useEffect(() => {
    if (txStatus !== "success") return;
    onSuccess?.();
    const id = setTimeout(onClose, AUTO_CLOSE_MS);
    return () => clearTimeout(id);
  }, [txStatus, onClose, onSuccess]);

  const onSubmit = useCallback(
    async (values: FundFormValues) => {
      await fund({
        grantId: grant.id,
        amount: xlmToStroops(values.amount),
        token: "native",
      });
    },
    [fund, grant.id]
  );

  const handleMaxClick = useCallback(() => {
    if (maxFundable > 0n) {
      setValue("amount", stroopsToXlm(maxFundable), { shouldValidate: true });
    }
  }, [maxFundable, setValue]);

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-40 bg-bg-primary/70"
            onClick={onClose}
            aria-hidden
          />

          {/* Dialog */}
          <motion.div
            role="dialog"
            aria-modal="true"
            aria-labelledby="fund-grant-title"
            initial={{ opacity: 0, scale: 0.96, y: 12 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 12 }}
            className="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 border border-border-color bg-surface p-6 ring-1 ring-border-color"
          >
            {/* Header */}
            <div className="flex items-start justify-between gap-4 mb-4">
              <h2
                id="fund-grant-title"
                className="font-orbitron text-lg text-text-primary"
              >
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

            {/* Funding progress bar */}
            <FundingProgress
              current={grant.funded}
              target={grant.budget}
              token={grant.token}
            />

            {/* ── States ── */}

            {/* Wallet not connected */}
            {!address ? (
              <div className="mt-6 border border-border-color bg-bg-secondary p-4 text-center">
                <p className="font-mono text-sm text-text-muted mb-3">
                  Connect your wallet to fund this grant.
                </p>
                <a
                  href="/profile"
                  className="font-orbitron text-xs uppercase tracking-wider text-accent-secondary hover:underline"
                >
                  Connect wallet →
                </a>
              </div>
            ) : txStatus === "success" ? (
              /* Success state */
              <div className="mt-6 border border-success/40 bg-success/10 p-5 text-center space-y-3">
                <p className="font-orbitron text-sm text-success">
                  Funding successful!
                </p>
                {explorerUrl && txHash && (
                  <a
                    href={explorerUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 font-mono text-xs text-accent-secondary underline"
                  >
                    View transaction <ExternalLink size={11} />
                  </a>
                )}
                <p className="font-mono text-[10px] text-text-muted">
                  Closing in {AUTO_CLOSE_MS / 1000}s…
                </p>
              </div>
            ) : txStatus === "error" ? (
              /* Retry state */
              <div className="mt-6 space-y-4">
                <div className="border border-danger/40 bg-danger/10 p-4">
                  <p className="font-mono text-xs text-danger">{fundError}</p>
                </div>
                <button
                  type="button"
                  onClick={resetFund}
                  className="flex w-full items-center justify-center gap-2 border border-border-color py-3 font-mono text-xs uppercase tracking-wider text-text-muted hover:text-text-primary transition-colors"
                >
                  <RefreshCw size={13} />
                  Try again
                </button>
              </div>
            ) : (
              /* Input form */
              <form
                onSubmit={handleSubmit(onSubmit)}
                className="mt-6 space-y-4"
              >
                {/* Wallet balance */}
                <div className="flex items-center justify-between font-mono text-[10px] text-text-muted">
                  <span>Wallet balance</span>
                  <span>
                    {isBalanceLoading
                      ? "Loading…"
                      : `${stroopsToXlm(walletXlm)} XLM`}
                  </span>
                </div>

                {/* Amount input + Max button */}
                <div>
                  <label
                    htmlFor="fund-amount"
                    className="block font-mono text-[10px] uppercase tracking-wider text-text-muted mb-2"
                  >
                    Amount (XLM)
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="fund-amount"
                      type="number"
                      min="0.0000001"
                      step="any"
                      {...register("amount")}
                      className="flex-1 border border-border-color bg-bg-secondary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent-primary"
                      placeholder="0.00"
                    />
                    <button
                      type="button"
                      onClick={handleMaxClick}
                      disabled={maxFundable === 0n || isBalanceLoading}
                      className="border border-border-color bg-bg-secondary px-3 font-mono text-[10px] uppercase tracking-wider text-text-muted hover:text-text-primary disabled:opacity-40 transition-colors"
                    >
                      Max
                    </button>
                  </div>
                  {errors.amount && (
                    <p className="mt-1 font-mono text-xs text-danger">
                      {errors.amount.message}
                    </p>
                  )}
                </div>

                {/* Contribution preview */}
                {amountStroops > 0n && (
                  <div className="border border-border-color bg-bg-secondary px-3 py-2">
                    <p className="font-mono text-[10px] text-text-muted">
                      Your contribution:{" "}
                      <span className="text-text-primary">
                        {stroopsToXlm(amountStroops)} XLM
                      </span>{" "}
                      ·{" "}
                      <span className="text-accent-secondary">
                        {contributionPct.toFixed(1)}% of total budget
                      </span>
                    </p>
                  </div>
                )}

                <button
                  type="submit"
                  disabled={isSubmitting || txStatus === "pending"}
                  className="flex w-full items-center justify-center gap-2 font-orbitron text-sm font-bold uppercase tracking-wider bg-accent-primary text-bg-primary py-3 disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90 transition-opacity"
                >
                  {isSubmitting || txStatus === "pending" ? (
                    <>
                      <Loader2 size={14} className="animate-spin" />
                      Processing…
                    </>
                  ) : (
                    "Fund Grant"
                  )}
                </button>
              </form>
            )}
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
