/**
 * TransactionPendingOverlay
 *
 * Full-screen overlay that guides the user through the complete Soroban
 * transaction lifecycle. Renders stage-specific feedback and a 4-step
 * progress stepper (Sign → Broadcast → Confirm → Done).
 */

"use client";

import React, { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";

// ── Types ─────────────────────────────────────────────────────────────────

export type TransactionStage =
  | "AWAITING_SIGNATURE"
  | "SIGNING"
  | "SUBMITTING"
  | "CONFIRMING"
  | "CONFIRMED"
  | "FAILED";

export interface TransactionPendingOverlayProps {
  isOpen: boolean;
  stage: TransactionStage | null;
  txHash?: string | null;
  error?: string | null;
  onClose?: () => void;
  onRetry?: () => void;
  title?: string;
}

// ── Stepper helpers ───────────────────────────────────────────────────────

const STEPS = ["Sign", "Broadcast", "Confirm", "Done"] as const;

function stageToStepIndex(stage: TransactionStage | null): number {
  switch (stage) {
    case "AWAITING_SIGNATURE":
    case "SIGNING":
      return 0;
    case "SUBMITTING":
      return 1;
    case "CONFIRMING":
      return 2;
    case "CONFIRMED":
    case "FAILED":
      return 3;
    default:
      return 0;
  }
}

// ── Stage icons / content ─────────────────────────────────────────────────

function FreighterLogo() {
  return (
    <svg
      viewBox="0 0 32 32"
      width="40"
      height="40"
      aria-hidden="true"
      className="text-accent-primary"
      fill="currentColor"
    >
      <text x="6" y="26" fontSize="28" fontFamily="monospace" fontWeight="bold">
        F
      </text>
    </svg>
  );
}

function LedgerTimer() {
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    setElapsed(0);
    const id = setInterval(() => {
      setElapsed((s) => (s + 1) % 6);
    }, 1000);
    return () => clearInterval(id);
  }, []);

  const pct = (elapsed / 5) * 100;

  return (
    <div className="flex flex-col items-center gap-2">
      <div className="relative h-12 w-12">
        <svg viewBox="0 0 36 36" className="h-12 w-12 -rotate-90">
          <circle
            cx="18"
            cy="18"
            r="15.9"
            fill="none"
            stroke="currentColor"
            strokeWidth="3"
            className="text-border-color"
          />
          <circle
            cx="18"
            cy="18"
            r="15.9"
            fill="none"
            stroke="currentColor"
            strokeWidth="3"
            strokeDasharray={`${pct} 100`}
            className="text-accent-primary transition-all duration-1000"
          />
        </svg>
        <span className="absolute inset-0 flex items-center justify-center font-mono text-sm text-text-primary">
          {elapsed}s
        </span>
      </div>
      <p className="font-mono text-sm text-text-muted">
        Awaiting ledger close…
      </p>
    </div>
  );
}

function StageContent({
  stage,
  txHash,
  error,
}: {
  stage: TransactionStage;
  txHash?: string | null;
  error?: string | null;
}) {
  switch (stage) {
    case "AWAITING_SIGNATURE":
      return (
        <div className="flex flex-col items-center gap-3 text-center">
          <motion.span
            className="text-3xl"
            animate={{ x: [0, 8, 0] }}
            transition={{ repeat: Infinity, duration: 1.2 }}
            aria-hidden="true"
          >
            →
          </motion.span>
          <p className="font-mono text-sm text-text-primary">
            Open Freighter in your browser extensions to sign this transaction.
          </p>
        </div>
      );

    case "SIGNING":
      return (
        <div className="flex flex-col items-center gap-3">
          <div className="relative">
            <FreighterLogo />
            <span
              role="status"
              aria-label="Signing"
              className="absolute -right-1 -top-1 h-4 w-4 animate-spin rounded-full border-2 border-accent-primary border-t-transparent"
            />
          </div>
          <p className="font-mono text-sm text-text-primary">
            Signing transaction…
          </p>
        </div>
      );

    case "SUBMITTING":
      return (
        <div className="flex flex-col items-center gap-3">
          <div className="relative flex h-10 w-10 items-center justify-center">
            {[0, 1, 2].map((i) => (
              <motion.span
                key={i}
                className="absolute h-full w-full rounded-full border border-accent-primary"
                animate={{ scale: [1, 1.8 + i * 0.4], opacity: [0.6, 0] }}
                transition={{
                  repeat: Infinity,
                  duration: 1.5,
                  delay: i * 0.4,
                }}
                aria-hidden="true"
              />
            ))}
            <span
              role="status"
              aria-label="Submitting"
              className="h-4 w-4 animate-spin rounded-full border-2 border-accent-primary border-t-transparent"
            />
          </div>
          <p className="font-mono text-sm text-text-primary">
            Broadcasting to Stellar…
          </p>
        </div>
      );

    case "CONFIRMING":
      return <LedgerTimer />;

    case "CONFIRMED":
      return (
        <div className="flex flex-col items-center gap-3 text-center">
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            transition={{ type: "spring", stiffness: 300, damping: 20 }}
          >
            <svg
              viewBox="0 0 24 24"
              width="48"
              height="48"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              className="text-green-500"
              aria-hidden="true"
            >
              <circle cx="12" cy="12" r="10" />
              <path d="M9 12l2 2 4-4" />
            </svg>
          </motion.div>
          <p className="font-mono text-sm font-semibold text-green-500">
            Transaction confirmed!
          </p>
          {txHash && (
            <div className="flex flex-col items-center gap-1">
              <p className="font-mono text-xs text-text-muted">
                {txHash.slice(0, 6)}…{txHash.slice(-4)}
              </p>
              <a
                href={`https://stellar.expert/explorer/testnet/tx/${txHash}`}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-xs text-accent-primary underline hover:no-underline"
              >
                View on Stellar Explorer →
              </a>
            </div>
          )}
        </div>
      );

    case "FAILED":
      return (
        <div className="flex flex-col items-center gap-3 text-center">
          <svg
            viewBox="0 0 24 24"
            width="48"
            height="48"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className="text-red-500"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="10" />
            <path d="M15 9l-6 6M9 9l6 6" />
          </svg>
          <p className="font-mono text-sm font-semibold text-red-500">
            Transaction failed
          </p>
          {error && (
            <p className="font-mono text-xs text-text-muted">{error}</p>
          )}
        </div>
      );
  }
}

// ── Stepper ───────────────────────────────────────────────────────────────

function ProgressStepper({ stage }: { stage: TransactionStage | null }) {
  const currentIdx = stageToStepIndex(stage);

  return (
    <div className="flex w-full items-center justify-between" role="list" aria-label="Transaction steps">
      {STEPS.map((label, idx) => {
        const isCompleted = idx < currentIdx;
        const isCurrent = idx === currentIdx;

        return (
          <React.Fragment key={label}>
            <div className="flex flex-col items-center gap-1" role="listitem">
              <div
                aria-current={isCurrent ? "step" : undefined}
                className={[
                  "flex h-6 w-6 items-center justify-center rounded-full border-2 text-xs font-mono transition-all",
                  isCompleted
                    ? "border-accent-primary bg-accent-primary text-bg-primary"
                    : isCurrent
                    ? "border-accent-primary bg-transparent text-accent-primary animate-pulse"
                    : "border-text-muted bg-transparent text-text-muted",
                ].join(" ")}
              >
                {isCompleted ? "✓" : idx + 1}
              </div>
              <span className="font-mono text-xs text-text-muted">{label}</span>
            </div>

            {idx < STEPS.length - 1 && (
              <div
                aria-hidden="true"
                className={[
                  "h-px flex-1 mx-1 transition-all",
                  idx < currentIdx ? "bg-accent-primary" : "bg-border-color",
                ].join(" ")}
              />
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
}

// ── Main component ────────────────────────────────────────────────────────

export function TransactionPendingOverlay({
  isOpen,
  stage,
  txHash,
  error,
  onClose,
  onRetry,
  title = "Transaction",
}: TransactionPendingOverlayProps) {
  const isDone = stage === "CONFIRMED" || stage === "FAILED";

  return (
    <AnimatePresence>
      {isOpen && stage && (
        <motion.div
          role="dialog"
          aria-modal="true"
          aria-label={title}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm"
        >
          <motion.div
            initial={{ scale: 0.95, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.95, opacity: 0 }}
            className="relative w-full max-w-md bg-surface border border-border-color p-6 shadow-xl"
          >
            {/* Title */}
            <h2 className="mb-4 font-orbitron text-base font-semibold uppercase tracking-wider text-text-primary">
              {title}
            </h2>

            {/* Stepper */}
            <ProgressStepper stage={stage} />

            {/* Stage content */}
            <div className="my-6 flex min-h-[100px] flex-col items-center justify-center">
              <StageContent stage={stage} txHash={txHash} error={error} />
            </div>

            {/* Action buttons — only after terminal stages */}
            {isDone && (
              <div className="flex justify-end gap-3">
                {stage === "FAILED" && onRetry && (
                  <button
                    type="button"
                    onClick={onRetry}
                    className="inline-flex items-center justify-center px-5 py-2 font-orbitron text-sm font-bold uppercase tracking-wider border border-accent-primary bg-transparent text-accent-primary transition-colors hover:bg-accent-primary hover:text-bg-primary"
                  >
                    Retry
                  </button>
                )}
                {onClose && (
                  <button
                    type="button"
                    onClick={onClose}
                    className="inline-flex items-center justify-center px-5 py-2 font-orbitron text-sm font-bold uppercase tracking-wider bg-accent-primary text-bg-primary transition-opacity hover:opacity-90"
                  >
                    Close
                  </button>
                )}
              </div>
            )}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
