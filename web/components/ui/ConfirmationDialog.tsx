/**
 * ConfirmationDialog
 *
 * A two-step "are you sure?" modal used before destructive or irreversible
 * actions (vote submission, dispute resolution, grant cancellation). Renders
 * its own overlay so it has no external Modal dependency.
 */

"use client";

import React from "react";

export interface ConfirmationDialogProps {
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: "default" | "danger";
  isLoading?: boolean;
}

export function ConfirmationDialog({
  isOpen,
  onConfirm,
  onCancel,
  title,
  description,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  variant = "default",
  isLoading = false,
}: ConfirmationDialogProps) {
  if (!isOpen) return null;

  const confirmClasses =
    variant === "danger"
      ? "bg-danger text-bg-primary hover:bg-opacity-90"
      : "bg-accent-primary text-bg-primary hover:bg-opacity-90";

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={title}
      data-variant={variant}
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
    >
      <div
        aria-hidden="true"
        onClick={isLoading ? undefined : onCancel}
        className="absolute inset-0 bg-black/60"
      />
      <div className="relative w-full max-w-md bg-surface border border-border-color rounded-none p-6">
        <h2 className="font-orbitron text-lg font-medium text-text-primary">{title}</h2>
        <p className="mt-2 font-mono text-sm text-text-muted">{description}</p>

        <div className="mt-6 flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={isLoading}
            className="inline-flex items-center justify-center px-6 py-2.5 font-orbitron text-sm font-bold uppercase tracking-wider rounded-none border border-accent-primary bg-transparent text-accent-primary transition-colors hover:bg-accent-primary hover:text-bg-primary disabled:cursor-not-allowed disabled:opacity-50"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={isLoading}
            className={`inline-flex items-center justify-center gap-2 px-6 py-2.5 font-orbitron text-sm font-bold uppercase tracking-wider rounded-none border-0 transition-colors disabled:cursor-not-allowed disabled:opacity-70 ${confirmClasses}`}
          >
            {isLoading && (
              <span
                role="status"
                aria-label="Loading"
                className="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
              />
            )}
            {isLoading ? "Processing…" : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
