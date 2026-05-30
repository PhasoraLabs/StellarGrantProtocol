"use client";

import { useState } from "react";
import { useFormContext, useFieldArray } from "react-hook-form";
import type { GrantFormData } from "@/lib/schemas/grant";

export function Step2Milestones() {
  const {
    register,
    control,
    watch,
    formState: { errors },
  } = useFormContext<GrantFormData>();

  const { fields, append, remove, move } = useFieldArray({
    control,
    name: "milestones",
  });

  const [draggedIdx, setDraggedIdx] = useState<number | null>(null);

  const milestones = watch("milestones") || [];
  const totalBudget = watch("totalBudget") || 0;
  const budgetToken = watch("budgetToken") === "native" ? "XLM" : "USDC";

  // Calculate allocations
  const totalAllocated = milestones.reduce(
    (sum, m) => sum + (Number(m?.reward) || 0),
    0
  );
  const isBalanced = Math.abs(totalAllocated - totalBudget) < 0.0001;

  const handleDragStart = (e: React.DragEvent, index: number) => {
    setDraggedIdx(index);
    e.dataTransfer.effectAllowed = "move";
  };

  const handleDragOver = (e: React.DragEvent, _index: number) => {
    e.preventDefault();
  };

  const handleDrop = (e: React.DragEvent, index: number) => {
    e.preventDefault();
    if (draggedIdx === null || draggedIdx === index) return;
    move(draggedIdx, index);
    setDraggedIdx(null);
  };

  const handleAddMilestone = () => {
    if (fields.length >= 10) return;
    append({ title: "", description: "", reward: 0 });
  };

  return (
    <div className="space-y-6">
      <div className="border-b border-border-color/20 pb-4">
        <h2 className="font-orbitron text-lg font-bold text-text-primary uppercase tracking-wider">
          Step 2: Define Milestones
        </h2>
        <p className="font-mono text-xs text-text-muted">
          Add up to 10 milestones defining key phases. Drag and drop rows to reorder them.
        </p>
      </div>

      {/* Warning Banner if unbalanced */}
      {!isBalanced && (
        <div className="border border-danger/40 bg-danger/10 p-4 font-mono text-xs text-danger">
          ⚠️ Reward total must match budget exactly: Allocated{" "}
          <span className="font-bold">{totalAllocated.toFixed(2)}</span> of{" "}
          <span className="font-bold">{totalBudget.toFixed(2)}</span> {budgetToken}.
        </div>
      )}

      {/* Dynamic List */}
      <div className="space-y-4">
        {fields.map((field, index) => {
          const itemError = errors.milestones?.[index];

          return (
            <div
              key={field.id}
              draggable
              onDragStart={(e) => handleDragStart(e, index)}
              onDragOver={(e) => handleDragOver(e, index)}
              onDrop={(e) => handleDrop(e, index)}
              onDragEnd={() => setDraggedIdx(null)}
              className={`p-4 border rounded-none bg-surface/50 backdrop-blur-sm transition-all duration-200 ${
                draggedIdx === index
                  ? "opacity-40 border-accent-primary"
                  : "border-border-color/60 hover:border-border-color"
              }`}
            >
              {/* Row Header */}
              <div className="flex items-center justify-between pb-3 border-b border-border-color/20 mb-3">
                <div className="flex items-center gap-2">
                  {/* Custom SVG drag handle */}
                  <span className="cursor-grab text-text-muted hover:text-text-primary select-none p-1">
                    <svg
                      width="16"
                      height="16"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2.5"
                    >
                      <circle cx="9" cy="5" r="1.5" />
                      <circle cx="15" cy="5" r="1.5" />
                      <circle cx="9" cy="12" r="1.5" />
                      <circle cx="15" cy="12" r="1.5" />
                      <circle cx="9" cy="19" r="1.5" />
                      <circle cx="15" cy="19" r="1.5" />
                    </svg>
                  </span>
                  <span className="font-orbitron text-xs font-semibold text-accent-secondary uppercase">
                    Milestone #{index + 1}
                  </span>
                </div>

                {fields.length > 1 && (
                  <button
                    type="button"
                    onClick={() => remove(index)}
                    className="font-mono text-xs uppercase tracking-wider text-text-muted hover:text-danger transition-colors"
                  >
                    Remove
                  </button>
                )}
              </div>

              {/* Grid Form Fields */}
              <div className="space-y-3">
                {/* Title */}
                <div className="space-y-1">
                  <input
                    type="text"
                    placeholder="Milestone Title (e.g. Prototype Deployment)"
                    {...register(`milestones.${index}.title` as const)}
                    className={`w-full rounded-none border bg-surface px-3 py-1.5 font-mono text-sm text-text-primary outline-none transition-colors ${
                      itemError?.title ? "border-danger" : "border-border-color"
                    }`}
                  />
                  {itemError?.title && (
                    <p className="font-mono text-xs text-danger">
                      {itemError.title.message}
                    </p>
                  )}
                </div>

                {/* Description */}
                <div className="space-y-1">
                  <textarea
                    placeholder="Provide a detailed description of what constitutes successful completion..."
                    rows={2}
                    {...register(`milestones.${index}.description` as const)}
                    className={`w-full rounded-none border bg-surface px-3 py-1.5 font-mono text-sm text-text-primary outline-none transition-colors resize-none ${
                      itemError?.description ? "border-danger" : "border-border-color"
                    }`}
                  />
                  {itemError?.description && (
                    <p className="font-mono text-xs text-danger">
                      {itemError.description.message}
                    </p>
                  )}
                </div>

                {/* Reward Reward */}
                <div className="space-y-1">
                  <div className="relative flex items-center">
                    <input
                      type="number"
                      step="any"
                      placeholder="Reward Allocation Amount"
                      {...register(`milestones.${index}.reward` as const, {
                        valueAsNumber: true,
                      })}
                      className={`w-full rounded-none border bg-surface px-3 py-1.5 font-mono text-sm text-text-primary outline-none transition-colors pr-16 ${
                        itemError?.reward ? "border-danger" : "border-border-color"
                      }`}
                    />
                    <span className="absolute right-3 font-mono text-xs text-text-muted uppercase">
                      {budgetToken}
                    </span>
                  </div>
                  {itemError?.reward && (
                    <p className="font-mono text-xs text-danger">
                      {itemError.reward.message}
                    </p>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Footer Allocations */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pt-4 border-t border-border-color/20">
        <div className="font-mono text-xs uppercase tracking-wider">
          Total allocated:{" "}
          <span
            className={`font-semibold ${
              isBalanced ? "text-success" : "text-danger"
            }`}
          >
            {totalAllocated.toFixed(2)}
          </span>{" "}
          / {totalBudget.toFixed(2)} {budgetToken}
        </div>

        <button
          type="button"
          disabled={fields.length >= 10}
          onClick={handleAddMilestone}
          className="px-4 py-2 font-mono text-xs uppercase tracking-widest border border-accent-primary text-accent-primary hover:bg-accent-primary/10 transition-colors disabled:cursor-not-allowed disabled:opacity-50 shrink-0"
        >
          + Add Milestone
        </button>
      </div>

      {errors.milestones?.root && (
        <p className="font-mono text-xs text-danger text-center">
          {errors.milestones.root.message}
        </p>
      )}
    </div>
  );
}
