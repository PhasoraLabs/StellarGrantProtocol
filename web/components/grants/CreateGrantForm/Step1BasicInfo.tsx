"use client";

import { Controller, useFormContext } from "react-hook-form";
import RichTextEditor from "@/components/ui/RichTextEditor";
import { AddressInput } from "@/components/ui/AddressInput";
import type { GrantFormData } from "@/lib/schemas/grant";

export function Step1BasicInfo() {
  const {
    register,
    control,
    watch,
    formState: { errors },
  } = useFormContext<GrantFormData>();

  const totalBudget = watch("totalBudget");
  const budgetToken = watch("budgetToken");

  const formattedBudget =
    typeof totalBudget === "number" && !isNaN(totalBudget)
      ? `${totalBudget.toFixed(7)} ${budgetToken === "native" ? "XLM" : "USDC"}`
      : `0.0000000 ${budgetToken === "native" ? "XLM" : "USDC"}`;

  return (
    <div className="space-y-6">
      <div className="border-b border-border-color/20 pb-4">
        <h2 className="font-orbitron text-lg font-bold text-text-primary uppercase tracking-wider">
          Step 1: Basic Information
        </h2>
        <p className="font-mono text-xs text-text-muted">
          Provide key details regarding the project title, rich description, and budget.
        </p>
      </div>

      {/* Grant Title */}
      <div className="space-y-1">
        <label htmlFor="title" className="block font-mono text-xs text-text-muted">
          Grant Title
        </label>
        <input
          id="title"
          type="text"
          placeholder="e.g. Building Stellar Bridges"
          {...register("title")}
          className={`w-full rounded-none border bg-surface px-3 py-2 font-mono text-sm text-text-primary outline-none transition-colors ${
            errors.title ? "border-danger" : "border-border-color"
          }`}
        />
        {errors.title && (
          <p className="font-mono text-xs text-danger">{errors.title.message}</p>
        )}
      </div>

      {/* Description (RichTextEditor) */}
      <div>
        <Controller
          name="description"
          control={control}
          render={({ field }) => (
            <RichTextEditor
              value={field.value || ""}
              onChange={field.onChange}
              label="Detailed Description"
              placeholder="Describe your grant objectives, core value proposition, and architecture..."
              maxLength={5000}
              required
            />
          )}
        />
        {errors.description && (
          <p className="font-mono text-xs text-danger mt-1">
            {errors.description.message}
          </p>
        )}
      </div>

      {/* Recipient Stellar Address */}
      <div>
        <Controller
          name="recipientAddress"
          control={control}
          render={({ field }) => (
            <AddressInput
              value={field.value || ""}
              onChange={field.onChange}
              label="Recipient Stellar Address"
              placeholder="e.g. GB..."
              showUseMyAddress
              showAvatar
              error={errors.recipientAddress?.message}
            />
          )}
        />
      </div>

      {/* Budget & Token */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Total Budget */}
        <div className="space-y-1">
          <label htmlFor="totalBudget" className="block font-mono text-xs text-text-muted">
            Total Budget
          </label>
          <input
            id="totalBudget"
            type="number"
            step="any"
            placeholder="e.g. 5000"
            {...register("totalBudget", { valueAsNumber: true })}
            className={`w-full rounded-none border bg-surface px-3 py-2 font-mono text-sm text-text-primary outline-none transition-colors ${
              errors.totalBudget ? "border-danger" : "border-border-color"
            }`}
          />
          <p className="font-mono text-[10px] text-accent-secondary mt-1">
            Amount: {formattedBudget}
          </p>
          {errors.totalBudget && (
            <p className="font-mono text-xs text-danger">{errors.totalBudget.message}</p>
          )}
        </div>

        {/* Budget Token */}
        <div className="space-y-2">
          <span className="block font-mono text-xs text-text-muted">Budget Token</span>
          <div className="flex gap-4 font-mono text-sm">
            <label className="flex items-center gap-2 cursor-pointer text-text-primary">
              <input
                type="radio"
                value="native"
                {...register("budgetToken")}
                className="accent-accent-primary"
              />
              XLM (Native)
            </label>
            <label className="flex items-center gap-2 cursor-pointer text-text-primary">
              <input
                type="radio"
                value="USDC"
                {...register("budgetToken")}
                className="accent-accent-primary"
              />
              USDC
            </label>
          </div>
          {errors.budgetToken && (
            <p className="font-mono text-xs text-danger">{errors.budgetToken.message}</p>
          )}
        </div>
      </div>

      {/* Deadline Picker */}
      <div className="space-y-1">
        <label htmlFor="deadline" className="block font-mono text-xs text-text-muted">
          Submission Expiry Deadline
        </label>
        <input
          id="deadline"
          type="datetime-local"
          {...register("deadline")}
          className={`w-full rounded-none border bg-surface px-3 py-2 font-mono text-sm text-text-primary outline-none transition-colors ${
            errors.deadline ? "border-danger" : "border-border-color"
          }`}
        />
        {errors.deadline && (
          <p className="font-mono text-xs text-danger">{errors.deadline.message}</p>
        )}
      </div>
    </div>
  );
}
