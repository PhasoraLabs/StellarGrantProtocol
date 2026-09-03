"use client";

import { useState } from "react";
import { useContractTransaction } from "@/hooks/useContractTransaction";
import { contractClient } from "@/lib/stellar/contract";
import { ProofViewer } from "@/components/milestones/ProofViewer";
import RichTextRenderer from "@/components/ui/RichTextRenderer";
import { formatTokenAmount } from "@/lib/tokens/utils";
import { useWalletStore } from "@/lib/store/walletStore";
import { toast } from "@/lib/toast";

interface DisputePanelProps {
  grantId: string;
  grantTitle: string;
  milestoneIdx: number;
  milestoneTitle: string;
  proofHash: string;
  contributorArgument: string | null;  // from API metadata
  funderArgument: string | null;       // from API metadata
  fundedAmount: bigint;
  token: string;
  priorVotes: { approved: number; rejected: number };
  onResolved: () => void;
}

export function DisputePanel({
  grantId,
  grantTitle,
  milestoneIdx,
  milestoneTitle,
  proofHash,
  contributorArgument,
  funderArgument,
  fundedAmount,
  token,
  priorVotes,
  onResolved,
}: DisputePanelProps) {
  const { address: councilAddress } = useWalletStore();
  const { execute, isPending, isSimulating, error: txError, reset } = useContractTransaction();
  const [confirmingAction, setConfirmingAction] = useState<"approve" | "refund" | null>(null);

  const isXlm = token.toLowerCase() === "native" || token.length < 10;
  const decimals = isXlm ? 7 : 6;
  const symbol = isXlm ? "XLM" : "USDC";

  const formattedAmount = formatTokenAmount(fundedAmount, decimals, {
    symbol,
    showSymbol: true,
  });

  const handleResolve = async (approvePayout: boolean) => {
    if (!councilAddress) {
      toast({
        title: "Wallet Not Connected",
        description: "Connect your wallet as a Council member to resolve disputes.",
        variant: "error",
      });
      return;
    }

    try {
      reset();
      const txParams = await contractClient.resolveDispute({
        grantId,
        milestoneIdx,
        approvePayout,
        councilAddress,
      });

      await execute({
        xdr: txParams,
        onSuccess: (txHash) => {
          toast({
            title: approvePayout ? "Payout Approved" : "Funders Refunded",
            description: `Dispute resolved successfully! Tx: ${txHash.slice(0, 10)}...`,
            variant: "success",
          });
          setConfirmingAction(null);
          onResolved();
        },
        onError: (err) => {
          console.error("Resolution transaction failed:", err);
        },
      });
    } catch (err: unknown) {
      console.error("Error executing resolution:", err);
    }
  };

  const isBusy = isPending || isSimulating;

  return (
    <div className="bg-surface/30 backdrop-blur-md border border-border-color/30 border-l-4 border-l-danger/60 p-6 space-y-6 hover:border-danger/30 transition-all duration-300">
      {/* Header Info */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-border-color/10">
        <div>
          <span className="font-mono text-[10px] tracking-widest text-danger font-semibold uppercase bg-danger/10 px-2 py-0.5 border border-danger/20 inline-block mb-1.5">
            ● DISPUTED
          </span>
          <h3 className="font-orbitron text-base font-bold text-text-primary uppercase tracking-wide">
            {grantTitle}
          </h3>
          <p className="font-mono text-xs text-text-muted mt-1">
            Milestone #{milestoneIdx + 1}: <span className="text-text-primary font-semibold">{milestoneTitle}</span>
          </p>
        </div>
        <div className="text-right shrink-0">
          <span className="font-mono text-[10px] text-text-muted block uppercase tracking-widest">
            Stake Amount
          </span>
          <span className="font-orbitron text-lg font-black text-accent-primary">
            {formattedAmount}
          </span>
        </div>
      </div>

      {/* Prior Votes / Tally */}
      <div className="flex flex-wrap items-center gap-4 bg-surface/20 border border-border-color/10 px-4 py-2 font-mono text-xs">
        <span className="text-text-muted uppercase tracking-wider text-[10px]">
          Milestone Vote Before Dispute:
        </span>
        <div className="flex gap-3">
          <span className="text-success font-semibold">✓ {priorVotes.approved} approved</span>
          <span className="text-danger font-semibold">✗ {priorVotes.rejected} rejected</span>
        </div>
      </div>

      {/* Arguments Section */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 font-mono text-xs">
        {/* Contributor Card */}
        <div className="p-4 border border-border-color/20 bg-surface/10 space-y-3">
          <h4 className="font-orbitron text-xs font-bold text-success uppercase tracking-widest border-b border-border-color/10 pb-2 flex items-center justify-between">
            <span>Contributor Argument</span>
            <span className="text-[10px] text-text-muted normal-case font-mono font-normal">Claim Payout</span>
          </h4>
          <div className="text-text-muted text-[11px] leading-relaxed max-h-[150px] overflow-y-auto pr-1">
            {contributorArgument ? (
              <RichTextRenderer content={contributorArgument} />
            ) : (
              "No argument submitted."
            )}
          </div>
        </div>

        {/* Funder Card */}
        <div className="p-4 border border-border-color/20 bg-surface/10 space-y-3">
          <h4 className="font-orbitron text-xs font-bold text-danger uppercase tracking-widest border-b border-border-color/10 pb-2 flex items-center justify-between">
            <span>Funder Argument</span>
            <span className="text-[10px] text-text-muted normal-case font-mono font-normal">Request Refund</span>
          </h4>
          <div className="text-text-muted text-[11px] leading-relaxed max-h-[150px] overflow-y-auto pr-1">
            {funderArgument ? (
              <RichTextRenderer content={funderArgument} />
            ) : (
              "No argument submitted."
            )}
          </div>
        </div>
      </div>

      {/* Milestone Proof Panel */}
      <div className="space-y-2">
        <span className="font-mono text-[10px] text-text-muted uppercase tracking-widest block">
          Milestone Proof
        </span>
        <ProofViewer proofHash={proofHash} />
      </div>

      {/* Actions / Confirmation Prompt */}
      <div className="pt-4 border-t border-border-color/10">
        {confirmingAction === null ? (
          <div className="flex flex-col sm:flex-row gap-3 justify-end">
            <button
              type="button"
              onClick={() => setConfirmingAction("refund")}
              className="px-5 py-2.5 font-mono text-xs uppercase tracking-widest border border-danger/40 text-danger hover:bg-danger/10 transition-colors"
            >
              ✗ Refund Funders
            </button>
            <button
              type="button"
              onClick={() => setConfirmingAction("approve")}
              className="px-5 py-2.5 font-mono text-xs uppercase tracking-widest bg-success text-black font-bold hover:bg-success/80 transition-colors"
            >
              ✓ Approve Payout to Contributor
            </button>
          </div>
        ) : (
          <div className="bg-surface/40 border border-border-color/30 p-4 space-y-4 font-mono text-xs text-left animate-fadeIn">
            <div>
              <span className="font-orbitron text-[11px] font-bold text-accent-secondary uppercase tracking-widest block mb-1">
                Confirm Resolution Plan
              </span>
              <p className="text-text-muted text-[11px]">
                {confirmingAction === "approve" ? (
                  <>
                    You are resolving this dispute in favor of the <span className="text-success font-semibold">Contributor</span>.
                    This will release the escrowed stake of <span className="text-text-primary font-bold">{formattedAmount}</span> to the contributor&apos;s payout address.
                  </>
                ) : (
                  <>
                    You are resolving this dispute in favor of the <span className="text-danger font-semibold">Funders</span>.
                    This will claw back the milestone allocation of <span className="text-text-primary font-bold">{formattedAmount}</span> and make it refundable to the original funders.
                  </>
                )}
              </p>
            </div>

            {txError && (
              <div className="text-danger bg-danger/10 border border-danger/20 p-2.5 break-all max-h-[100px] overflow-y-auto">
                Error: {txError}
              </div>
            )}

            <div className="flex flex-wrap items-center justify-between gap-3 pt-2">
              <span className="text-[10px] text-text-muted italic">
                {isSimulating
                  ? "Simulating resolution..."
                  : isPending
                  ? "Signing & broadcasting transaction..."
                  : "Freighter wallet signature required."}
              </span>
              <div className="flex gap-2">
                <button
                  type="button"
                  disabled={isBusy}
                  onClick={() => {
                    setConfirmingAction(null);
                    reset();
                  }}
                  className="px-4 py-2 border border-border-color/40 text-text-muted hover:text-text-primary transition-colors disabled:opacity-50"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  disabled={isBusy}
                  onClick={() => handleResolve(confirmingAction === "approve")}
                  className="px-4 py-2 bg-accent-primary text-black font-bold hover:bg-accent-primary/80 transition-colors disabled:opacity-50 flex items-center gap-2"
                >
                  {isBusy && <div className="h-3 w-3 animate-spin rounded-full border border-black border-t-transparent" />}
                  Confirm
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
