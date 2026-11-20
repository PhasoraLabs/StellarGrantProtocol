"use client";

import { useFormContext, useWatch } from "react-hook-form";
import type { CreateGrantFormValues } from "./types";

export function TotalBudgetField() {
  const { register } = useFormContext<CreateGrantFormValues>();
  const token = useWatch({ name: "token" });

  return (
    <div className="border border-border-color bg-surface p-4 ring-1 ring-border-color space-y-3">
      <label className="block font-mono text-[10px] uppercase tracking-wider text-text-muted">
        Total budget ({token || "XLM"})
      </label>
      <input
        type="number"
        min={0}
        {...register("totalBudget", { valueAsNumber: true })}
        className="w-full border border-border-color bg-bg-secondary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent-primary"
      />
    </div>
  );
}
