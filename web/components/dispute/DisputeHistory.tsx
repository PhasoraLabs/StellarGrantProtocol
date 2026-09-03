"use client";

import { formatTokenAmount } from "@/lib/tokens/utils";

export interface ResolvedDispute {
  id: string;
  grantTitle: string;
  milestoneIdx: number;
  milestoneTitle: string;
  resolution: "payout" | "refund";
  resolvedAt: Date | string;
  fundedAmount: bigint;
  token: string;
}

interface DisputeHistoryProps {
  history: ResolvedDispute[];
}

export function DisputeHistory({ history }: DisputeHistoryProps) {
  return (
    <details className="group border border-border-color/20 bg-surface/10 p-4 transition-all duration-300 [&_summary::-webkit-details-marker]:hidden">
      <summary className="flex items-center justify-between cursor-pointer list-none select-none font-orbitron text-xs font-bold text-text-primary uppercase tracking-widest">
        <span>Resolved Disputes ({history.length})</span>
        <span className="text-text-muted group-open:rotate-180 transition-transform duration-300">
          ▼
        </span>
      </summary>

      {history.length === 0 ? (
        <p className="font-mono text-xs text-text-muted mt-4 text-center">
          No resolved disputes in history.
        </p>
      ) : (
        <div className="mt-4 overflow-x-auto">
          <table className="w-full font-mono text-left text-xs border-collapse">
            <thead>
              <tr className="border-b border-border-color/10 text-text-muted text-[10px] uppercase tracking-wider">
                <th className="py-2 font-normal">Grant / Milestone</th>
                <th className="py-2 font-normal">Resolution</th>
                <th className="py-2 font-normal text-right">Amount</th>
                <th className="py-2 font-normal text-right">Date</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-color/5">
              {history.map((h) => {
                const isXlm = h.token.toLowerCase() === "native" || h.token.length < 10;
                const decimals = isXlm ? 7 : 6;
                const symbol = isXlm ? "XLM" : "USDC";

                const formatted = formatTokenAmount(h.fundedAmount, decimals, {
                  symbol,
                  showSymbol: true,
                });

                const formattedDate = new Date(h.resolvedAt).toLocaleDateString(undefined, {
                  year: "numeric",
                  month: "short",
                  day: "numeric",
                });

                return (
                  <tr key={h.id} className="hover:bg-white/5 transition-colors">
                    <td className="py-3 pr-4">
                      <span className="font-bold text-text-primary block text-[11px] uppercase tracking-wide truncate max-w-[200px] sm:max-w-xs">
                        {h.grantTitle}
                      </span>
                      <span className="text-[10px] text-text-muted block mt-0.5">
                        Milestone #{h.milestoneIdx + 1}: {h.milestoneTitle}
                      </span>
                    </td>
                    <td className="py-3">
                      {h.resolution === "payout" ? (
                        <span className="px-2 py-0.5 border border-success/20 bg-success/10 text-success text-[10px] font-semibold uppercase tracking-wider">
                          Payout Approved
                        </span>
                      ) : (
                        <span className="px-2 py-0.5 border border-danger/20 bg-danger/10 text-danger text-[10px] font-semibold uppercase tracking-wider">
                          Funders Refunded
                        </span>
                      )}
                    </td>
                    <td className="py-3 text-right font-semibold text-text-primary text-[11px]">
                      {formatted}
                    </td>
                    <td className="py-3 text-right text-text-muted text-[10px]">
                      {formattedDate}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </details>
  );
}
