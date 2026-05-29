"use client";

import { useFieldArray, useFormContext } from "react-hook-form";
import { BudgetDistributionChart } from "./BudgetDistributionChart";
import type { CreateGrantFormValues } from "./types";

export function Step2Milestones() {
  const { register, watch, control } = useFormContext<CreateGrantFormValues>();
  const { fields, append, remove } = useFieldArray({
    control,
    name: "milestones",
  });

  const milestones = watch("milestones");
  const totalBudget = watch("totalBudget");
  const token = watch("token");

  return (
    <div className="space-y-6">
      <div>
        <h2 className="font-orbitron text-lg text-text-primary mb-1">Milestones</h2>
        <p className="font-mono text-xs text-text-muted">
          Assign rewards that sum to your total budget.
        </p>
      </div>

      <BudgetDistributionChart
        totalBudget={Number(totalBudget) || 0}
        milestones={(milestones ?? []).map((m) => ({
          title: m.title,
          reward: Number(m.reward) || 0,
        }))}
        token={token || "XLM"}
      />

      <ul className="space-y-4">
        {fields.map((field, index) => (
          <li
            key={field.id}
            className="border border-border-color bg-surface p-4 ring-1 ring-border-color space-y-3"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="font-orbitron text-xs uppercase tracking-wider text-text-muted">
                Milestone {index + 1}
              </span>
              {fields.length > 1 && (
                <button
                  type="button"
                  onClick={() => remove(index)}
                  className="font-mono text-[10px] uppercase text-danger hover:underline"
                >
                  Remove
                </button>
              )}
            </div>
            <input
              {...register(`milestones.${index}.title` as const)}
              placeholder="Title"
              className="w-full border border-border-color bg-bg-secondary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent-primary"
            />
            <input
              {...register(`milestones.${index}.reward` as const, { valueAsNumber: true })}
              type="number"
              min={0}
              step="any"
              placeholder="Reward"
              className="w-full border border-border-color bg-bg-secondary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent-primary"
            />
          </li>
        ))}
      </ul>

      <button
        type="button"
        onClick={() => append({ title: "", description: "", reward: 0 })}
        className="font-mono text-xs uppercase tracking-wider text-accent-secondary hover:underline"
      >
        + Add milestone
      </button>
    </div>
  );
}
