"use client";

import React, { useState, useEffect, useCallback } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { FundingProgress } from "@/components/grants/FundingProgress";
import { GrantStatusBadge } from "@/components/grants/GrantStatusBadge";
import { useWalletStore } from "@/lib/store/walletStore";
import { useFundGrant } from "@/hooks/useFundGrant";
import { stellarExplorerTx } from "@/lib/constants";
import { Grant } from "@/types";

// ── Wallet guard ──────────────────────────────────────────────────────────────

function WalletGuard({ children }: { children: React.ReactNode }) {
  const { address } = useWalletStore();
  if (address) return <>{children}</>;

  return (
    <div className="rounded-none border border-accent-primary p-8 text-center">
      <p className="text-text-secondary mb-4 font-orbitron text-sm uppercase tracking-wider">
        Connect your wallet to fund this grant
      </p>
      <button
        className="inline-flex items-center justify-center px-8 py-3 font-orbitron text-sm font-bold uppercase tracking-wider bg-accent-primary text-bg-primary hover:bg-opacity-90 transition-all duration-300"
        onClick={() => {
          // Wallet connection is handled globally; dispatch a connect event.
          window.dispatchEvent(new CustomEvent("stellar:connect-wallet"));
        }}
      >
        Connect Wallet
      </button>
    </div>
  );
}

// ── Amount input helpers ──────────────────────────────────────────────────────

const STROOP = 10_000_000n; // 1 XLM = 10_000_000 stroops

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

// ── Main client component ─────────────────────────────────────────────────────

interface FundGrantClientProps {
  grantId: string;
}

export function FundGrantClient({ grantId }: FundGrantClientProps) {
  const router = useRouter();
  const { address } = useWalletStore();
  const { fund, txStatus, error: fundError, reset: resetFund } = useFundGrant();
  const isLoading = txStatus === "pending";

  const [grant, setGrant] = useState<Grant | null>(null);
  const [isGrantLoading, setIsGrantLoading] = useState(true);

  const [token, setToken] = useState<"native" | "USDC">("native");
  const [amountInput, setAmountInput] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [successTx, setSuccessTx] = useState<string | null>(null);

  // Fetch grant data
  useEffect(() => {
    let cancelled = false;
    setIsGrantLoading(true);

    fetch(`/api/grants/${grantId}`)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
      .then((json: { grant: Grant & { budget: string | bigint; funded: string | bigint; deadline: string | bigint; created_at: string | bigint } }) => {
        if (!cancelled && json.grant) {
          const g = json.grant;
          setGrant({
            ...g,
            budget: BigInt(g.budget),
            funded: BigInt(g.funded),
            deadline: BigInt(g.deadline),
            created_at: BigInt(g.created_at),
          });
        } else if (!cancelled) {
          setGrant(null);
        }
      })
      .catch(() => {
        if (!cancelled) setGrant(null);
      })
      .finally(() => {
        if (!cancelled) setIsGrantLoading(false);
      });

    return () => { cancelled = true; };
  }, [grantId]);

  const unfundedStroops = grant ? grant.budget - grant.funded : 0n;

  const validate = useCallback((): boolean => {
    const stroops = xlmToStroops(amountInput);
    if (stroops <= 0n) {
      setValidationError("Amount must be at least 1 stroop (0.0000001 XLM).");
      return false;
    }
    if (stroops > unfundedStroops) {
      setValidationError(
        `Amount exceeds the remaining unfunded balance (${stroopsToXlm(unfundedStroops)} XLM).`
      );
      return false;
    }
    setValidationError(null);
    return true;
  }, [amountInput, unfundedStroops]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    resetFund();
    if (!validate()) return;

    const stroops = xlmToStroops(amountInput);
    try {
      const result = await fund({ grantId, amount: stroops, token });
      setSuccessTx(result.txHash);
      // Redirect to grant detail after 2 seconds
      setTimeout(() => router.push(`/grants/${grantId}`), 2000);
    } catch {
      // error already captured in fundError
    }
  };

  // ── Loading skeleton ────────────────────────────────────────────────────────

  if (isGrantLoading) {
    return (
      <div className="container mx-auto px-4 py-8 max-w-2xl">
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-surface-secondary rounded w-1/3" />
          <div className="h-10 bg-surface-secondary rounded w-2/3" />
          <div className="h-48 bg-surface-secondary rounded" />
        </div>
      </div>
    );
  }

  // ── Grant not found ─────────────────────────────────────────────────────────

  if (!grant) {
    return (
      <div className="container mx-auto px-4 py-8 max-w-2xl">
        <p className="text-red-400 font-orbitron">Grant #{grantId} not found.</p>
        <Link href="/grants" className="text-accent-primary underline mt-4 inline-block">
          ← Back to Grants
        </Link>
      </div>
    );
  }

  // ── Success state ───────────────────────────────────────────────────────────

  if (successTx) {
    return (
      <div className="container mx-auto px-4 py-8 max-w-2xl text-center">
        <div className="border border-green-500 p-8 rounded-none">
          <p className="text-green-400 font-orbitron text-lg font-bold uppercase mb-2">
            Funding Successful!
          </p>
          <p className="text-text-secondary text-sm mb-4">
            Transaction:{" "}
            <a
              href={stellarExplorerTx(successTx)}
              target="_blank"
              rel="noopener noreferrer"
              className="text-accent-primary underline font-mono"
            >
              {successTx.slice(0, 20)}…
            </a>
          </p>
          <p className="text-text-secondary text-xs">Redirecting to grant page…</p>
        </div>
      </div>
    );
  }

  // ── Main form ───────────────────────────────────────────────────────────────

  return (
    <div className="container mx-auto px-4 py-8 max-w-2xl">
      {/* Header */}
      <div className="mb-6">
        <Link
          href={`/grants/${grantId}`}
          className="text-accent-primary text-sm uppercase tracking-wider hover:underline"
        >
          ← Back to Grant
        </Link>
        <div className="flex items-center gap-3 mt-3">
          <h1 className="font-orbitron text-2xl font-bold text-text-primary">
            {grant.title}
          </h1>
          <GrantStatusBadge status={grant.status} />
        </div>
      </div>

      {/* Funding progress */}
      <div className="mb-8 border border-surface-secondary p-4">
        <p className="text-text-secondary text-xs uppercase tracking-wider mb-3 font-orbitron">
          Funding Progress
        </p>
        <FundingProgress
          current={grant.funded}
          target={grant.budget}
          token={grant.token}
        />
        <p className="text-text-secondary text-xs mt-2">
          Remaining:{" "}
          <span className="text-text-primary font-bold">
            {stroopsToXlm(unfundedStroops)} XLM
          </span>
        </p>
      </div>

      {/* Wallet guard */}
      <WalletGuard>
        <form onSubmit={handleSubmit} className="space-y-6">
          {/* Wallet balance info */}
          {address && (
            <div className="text-text-secondary text-xs font-orbitron">
              Connected: <span className="text-text-primary font-mono">{address.slice(0, 10)}…{address.slice(-6)}</span>
            </div>
          )}

          {/* Token selector */}
          <div>
            <label className="block text-xs font-orbitron uppercase tracking-wider text-text-secondary mb-2">
              Token
            </label>
            <div className="flex gap-2">
              {(["native", "USDC"] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setToken(t)}
                  className={[
                    "px-4 py-2 font-orbitron text-xs font-bold uppercase tracking-wider transition-all duration-200 border",
                    token === t
                      ? "bg-accent-primary text-bg-primary border-accent-primary"
                      : "bg-transparent text-accent-primary border-accent-primary hover:bg-accent-primary hover:text-bg-primary",
                  ].join(" ")}
                >
                  {t === "native" ? "XLM" : t}
                </button>
              ))}
            </div>
          </div>

          {/* Amount input */}
          <div>
            <label
              htmlFor="amount"
              className="block text-xs font-orbitron uppercase tracking-wider text-text-secondary mb-2"
            >
              Amount ({token === "native" ? "XLM" : "USDC"})
            </label>
            <input
              id="amount"
              type="number"
              min="0.0000001"
              step="any"
              value={amountInput}
              onChange={(e) => {
                setAmountInput(e.target.value);
                setValidationError(null);
                resetFund();
              }}
              placeholder="0.00"
              className="w-full bg-surface-secondary border border-neutral-700 focus:border-accent-primary text-text-primary font-mono px-4 py-3 outline-none transition-colors duration-200"
            />
            {validationError && (
              <p className="text-red-400 text-xs mt-1">{validationError}</p>
            )}
          </div>

          {/* API / contract error */}
          {fundError && !validationError && (
            <div className="border border-red-500 p-4">
              <p className="text-red-400 text-sm">{fundError}</p>
              <button
                type="button"
                onClick={resetFund}
                className="text-accent-primary text-xs underline mt-2"
              >
                Dismiss
              </button>
            </div>
          )}

          {/* Submit */}
          <button
            type="submit"
            disabled={isLoading || !amountInput}
            className={[
              "w-full font-orbitron text-sm font-bold uppercase tracking-wider px-8 py-3 transition-all duration-300",
              isLoading || !amountInput
                ? "bg-neutral-700 text-neutral-500 cursor-not-allowed"
                : "bg-accent-primary text-bg-primary hover:bg-opacity-90",
            ].join(" ")}
          >
            {isLoading ? "Signing & Broadcasting…" : "Fund Grant"}
          </button>
        </form>
      </WalletGuard>
    </div>
  );
}
