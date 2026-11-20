"use client";

import { useEffect } from "react";
import { useFormContext, Controller } from "react-hook-form";
import { AddressInput } from "@/components/ui/AddressInput";
import type { GrantFormData } from "@/lib/schemas/grant";

export function Step3Reviewers() {
  const {
    control,
    register,
    watch,
    setValue,
    formState: { errors },
  } = useFormContext<GrantFormData>();

  let reviewers = watch("reviewers") as string[];
  if (!reviewers || reviewers.length === 0) {
    reviewers = ["", "", ""];
  }

  useEffect(() => {
    const current = watch("reviewers");
    if (!current || current.length === 0) {
      setValue("reviewers", ["", "", ""]);
    }
  }, [setValue, watch]);

  const recipientAddress = watch("recipientAddress") || "";

  // Check if recipient is added as reviewer
  const isRecipientAddedAsReviewer = reviewers.some(
    (addr) => addr && addr.trim() === recipientAddress.trim()
  );

  const handleAddReviewer = () => {
    if (reviewers.length >= 7) return;
    setValue("reviewers", [...reviewers, ""]);
  };

  const handleRemoveReviewer = (index: number) => {
    if (reviewers.length <= 3) return;
    const next = reviewers.filter((_, idx) => idx !== index);
    setValue("reviewers", next, { shouldValidate: true });
  };

  return (
    <div className="space-y-6">
      <div className="border-b border-border-color/20 pb-4">
        <h2 className="font-orbitron text-lg font-bold text-text-primary uppercase tracking-wider">
          Step 3: Reviewers & Quorum
        </h2>
        <p className="font-mono text-xs text-text-muted">
          Add between 3 and 7 reviewers to vote on milestone completion. Define the required quorum.
        </p>
      </div>

      {/* Recipient Added as Reviewer Warning */}
      {isRecipientAddedAsReviewer && (
        <div className="border border-warning/40 bg-warning/10 p-4 font-mono text-xs text-warning">
          ⚠️ Warning: The recipient address you defined in Step 1 is also added as a reviewer.
          While allowed, it is generally recommended to separate recipient and reviewer roles.
        </div>
      )}

      {/* Dynamic Reviewers List */}
      <div className="space-y-4">
        {reviewers.map((_, index) => {
          const itemError = errors.reviewers?.[index];

          return (
            <div
              key={index}
              className="p-4 border border-border-color/40 bg-surface/30 flex flex-col md:flex-row items-stretch md:items-end gap-4"
            >
              <div className="grow">
                <Controller
                  name={`reviewers.${index}`}
                  control={control}
                  render={({ field: inputField }) => (
                    <AddressInput
                      value={inputField.value || ""}
                      onChange={inputField.onChange}
                      label={`Reviewer #${index + 1} Stellar Address`}
                      placeholder="e.g. GB..."
                      showAvatar
                      error={itemError?.message}
                    />
                  )}
                />
              </div>

              {reviewers.length > 3 && (
                <button
                  type="button"
                  onClick={() => handleRemoveReviewer(index)}
                  className="px-4 py-2 font-mono text-xs uppercase tracking-wider text-text-muted hover:text-danger border border-transparent hover:border-danger/20 transition-colors shrink-0 md:h-[38px] flex items-center justify-center"
                >
                  Remove
                </button>
              )}
            </div>
          );
        })}
      </div>

      {errors.reviewers && !Array.isArray(errors.reviewers) && (errors.reviewers as { message?: string }).message && (
        <p className="font-mono text-xs text-danger">{(errors.reviewers as { message?: string }).message}</p>
      )}

      {/* Add Reviewer Button */}
      <div className="flex justify-end">
        <button
          type="button"
          disabled={reviewers.length >= 7}
          onClick={handleAddReviewer}
          className="px-4 py-2 font-mono text-xs uppercase tracking-widest border border-accent-primary text-accent-primary hover:bg-accent-primary/10 transition-colors disabled:cursor-not-allowed disabled:opacity-50 shrink-0"
        >
          + Add Reviewer
        </button>
      </div>

      {/* Quorum Selector */}
      <div className="p-4 border border-border-color/40 bg-surface/30 space-y-3">
        <div className="space-y-1">
          <label htmlFor="quorum" className="block font-mono text-xs text-text-muted">
            Quorum Required approvals to pass a milestone vote
          </label>
          <div className="flex items-center gap-3">
            <input
              id="quorum"
              type="number"
              min={1}
              max={reviewers.length}
              {...register("quorum", { valueAsNumber: true })}
              className={`max-w-[120px] rounded-none border bg-surface px-3 py-1.5 font-mono text-sm text-text-primary outline-none transition-colors ${
                errors.quorum ? "border-danger" : "border-border-color"
              }`}
            />
            <span className="font-mono text-xs text-text-muted">
              out of {reviewers.length} reviewers
            </span>
          </div>
          {errors.quorum && (
            <p className="font-mono text-xs text-danger">{errors.quorum.message}</p>
          )}
        </div>
      </div>
    </div>
  );
}
