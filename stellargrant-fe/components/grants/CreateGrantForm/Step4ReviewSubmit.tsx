"use client";

import { useState } from "react";
import { useFormContext } from "react-hook-form";
import { useRouter } from "next/navigation";
import { useContractTransaction } from "@/hooks/useContractTransaction";
import { contractClient } from "@/lib/stellar/contract";
import type { GrantFormData } from "@/lib/schemas/grant";
import { toast } from "@/lib/toast";

export function Step4ReviewSubmit() {
  const { watch } = useFormContext<GrantFormData>();
  const router = useRouter();
  const { execute, isPending, isSimulating, error, reset } = useContractTransaction();
  const [showModal, setShowModal] = useState(false);

  const values = watch();
  const budgetToken = values.budgetToken === "native" ? "XLM" : "USDC";

  // Address of native token and USDC token
  const tokenAddress =
    values.budgetToken === "native"
      ? process.env.NEXT_PUBLIC_NATIVE_TOKEN ||
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
      : process.env.NEXT_PUBLIC_USDC_TOKEN ||
        "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA";

  const handleDeploy = async () => {
    reset(); // clear past errors
    setShowModal(true);

    try {
      // Formulate Soroban inputs
      const totalAmount = BigInt(Math.round(values.totalBudget || 0));
      const milestoneAmount =
        values.milestones && values.milestones.length > 0
          ? BigInt(Math.round((values.milestones[0].reward || 0)))
          : 0n;

      const txParams = await contractClient.grantCreate({
        owner: values.recipientAddress,
        title: values.title,
        description: values.description,
        tokenAddress,
        totalAmount,
        milestoneAmount,
        numMilestones: values.milestones ? values.milestones.length : 0,
        reviewers: values.reviewers || [],
        quorum: values.quorum || 1,
      });

      const hash = await execute({
        method: txParams.method,
        args: txParams.args,
        onSuccess: (txHash) => {
          toast({
            title: "Grant Created Successfully!",
            description: `On-chain deployment confirmed. Tx: ${txHash.slice(0, 10)}...`,
            variant: "success",
          });
          setTimeout(() => {
            setShowModal(false);
            router.push("/grants");
          }, 2000);
        },
        onError: (err) => {
          console.error("Contract transaction failed:", err);
        },
      });

      if (!hash) {
        // execute returns null on failure or rejection
        // user transaction failed, showRetry inside modal/inline
      }
    } catch (err: unknown) {
      console.error("Error during deployment formulation:", err);
    }
  };

  const isDeploying = isPending || isSimulating;

  return (
    <div className="space-y-6">
      <div className="border-b border-border-color/20 pb-4">
        <h2 className="font-orbitron text-lg font-bold text-text-primary uppercase tracking-wider">
          Step 4: Review & Deploy
        </h2>
        <p className="font-mono text-xs text-text-muted">
          Review all information carefully before signing the transaction on-chain.
        </p>
      </div>

      {/* Summary Deck */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 font-mono text-xs">
        {/* Core Info */}
        <div className="p-4 border border-border-color/40 bg-surface/30 space-y-4">
          <h3 className="font-orbitron text-xs font-bold text-accent-secondary uppercase tracking-widest border-b border-border-color/10 pb-2">
            Core Metadata
          </h3>
          <div className="space-y-2">
            <div>
              <span className="text-text-muted block uppercase tracking-wider text-[10px]">
                Title
              </span>
              <span className="text-text-primary text-sm font-semibold">{values.title}</span>
            </div>
            <div>
              <span className="text-text-muted block uppercase tracking-wider text-[10px]">
                Budget Allocation
              </span>
              <span className="text-text-primary text-sm font-semibold">
                {values.totalBudget} {budgetToken}
              </span>
            </div>
            <div>
              <span className="text-text-muted block uppercase tracking-wider text-[10px]">
                Recipient Address
              </span>
              <span className="text-text-primary break-all">{values.recipientAddress}</span>
            </div>
            <div>
              <span className="text-text-muted block uppercase tracking-wider text-[10px]">
                Submission Deadline
              </span>
              <span className="text-text-primary">{values.deadline}</span>
            </div>
          </div>
        </div>

        {/* Reviewers & Quorum */}
        <div className="p-4 border border-border-color/40 bg-surface/30 space-y-4">
          <h3 className="font-orbitron text-xs font-bold text-accent-secondary uppercase tracking-widest border-b border-border-color/10 pb-2">
            Review Board
          </h3>
          <div className="space-y-2">
            <div>
              <span className="text-text-muted block uppercase tracking-wider text-[10px]">
                Required Quorum
              </span>
              <span className="text-text-primary text-sm font-semibold">
                {values.quorum} approvals required
              </span>
            </div>
            <div>
              <span className="text-text-muted block uppercase tracking-wider text-[10px]">
                Reviewers ({values.reviewers?.length})
              </span>
              <ul className="space-y-1 mt-1 text-text-primary">
                {values.reviewers?.map((r, i) => (
                  <li key={i} className="break-all list-decimal list-inside">
                    {r}
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      </div>

      {/* Milestones Card Summary */}
      <div className="p-4 border border-border-color/40 bg-surface/30 font-mono text-xs space-y-4">
        <h3 className="font-orbitron text-xs font-bold text-accent-secondary uppercase tracking-widest border-b border-border-color/10 pb-2">
          Milestone Breakdown ({values.milestones?.length})
        </h3>
        <div className="space-y-3">
          {values.milestones?.map((m, idx) => (
            <div
              key={idx}
              className="p-3 border border-border-color/20 bg-surface/10 flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3"
            >
              <div>
                <span className="font-orbitron text-[10px] font-semibold text-accent-primary uppercase block">
                  Milestone #{idx + 1}: {m.title}
                </span>
                <p className="text-text-muted text-[11px] mt-0.5 line-clamp-1">
                  {m.description}
                </p>
              </div>
              <span className="text-text-primary font-bold shrink-0">
                {m.reward} {budgetToken}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Trigger Button */}
      <div className="flex justify-end gap-4">
        <button
          type="button"
          disabled={isDeploying}
          onClick={handleDeploy}
          className="px-6 py-3 font-orbitron font-bold text-sm uppercase tracking-widest bg-accent-primary text-black hover:bg-accent-primary/80 transition-colors disabled:cursor-not-allowed disabled:opacity-50 shrink-0"
        >
          Deploy Grant On-Chain
        </button>
      </div>

      {/* Loading & Signature Modal */}
      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
          <div className="w-full max-w-md border border-border-color bg-surface p-6 space-y-6 text-center shadow-xl">
            <h3 className="font-orbitron text-lg font-bold text-text-primary uppercase tracking-wider">
              On-Chain Deployment
            </h3>

            {isSimulating && (
              <div className="space-y-4">
                <div className="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-accent-secondary border-t-transparent" />
                <p className="font-mono text-sm text-text-primary">
                  Simulating Soroban transaction...
                </p>
              </div>
            )}

            {isPending && (
              <div className="space-y-4">
                <div className="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-accent-primary border-t-transparent" />
                <p className="font-mono text-sm text-text-primary">
                  Awaiting Freighter signature & ledger confirmation...
                </p>
              </div>
            )}

            {error && (
              <div className="space-y-4">
                <div className="text-danger text-sm font-mono p-3 bg-danger/10 border border-danger/40 text-left overflow-auto max-h-[150px]">
                  {error}
                </div>
                <div className="flex justify-end gap-2">
                  <button
                    type="button"
                    onClick={() => {
                      reset();
                      setShowModal(false);
                    }}
                    className="px-4 py-2 font-mono text-xs uppercase tracking-wider text-text-muted hover:text-text-primary"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    onClick={handleDeploy}
                    className="px-4 py-2 font-mono text-xs uppercase tracking-wider bg-accent-primary text-black hover:bg-accent-primary/80"
                  >
                    Retry
                  </button>
                </div>
              </div>
            )}

            {!isDeploying && !error && (
              <p className="font-mono text-sm text-text-muted">Preparing deployment...</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
