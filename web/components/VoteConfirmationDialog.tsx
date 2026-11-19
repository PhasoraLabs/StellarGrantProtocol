"use client";

import React from "react";
import { ConfirmationDialog } from "./ui/ConfirmationDialog";

export interface VoteConfirmationDialogProps {
  isOpen: boolean;
  isApproving: boolean | null;
  isLoading?: boolean;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
}

export function VoteConfirmationDialog({
  isOpen,
  isApproving,
  isLoading = false,
  onConfirm,
  onCancel,
}: VoteConfirmationDialogProps) {
  if (!isOpen || isApproving === null) {
    return null;
  }

  const title = isApproving ? "Approve Milestone?" : "Reject Milestone?";
  const description = isApproving
    ? "You are about to approve this milestone. This action is permanent and will count as your vote towards completion."
    : "You are about to reject this milestone. This action is permanent and will count as your vote against completion.";
  const confirmLabel = isApproving ? "Approve" : "Reject";
  const variant = isApproving ? "default" : "danger";

  return (
    <ConfirmationDialog
      isOpen={isOpen}
      onConfirm={onConfirm}
      onCancel={onCancel}
      title={title}
      description={description}
      confirmLabel={confirmLabel}
      cancelLabel="Cancel"
      variant={variant as "default" | "danger"}
      isLoading={isLoading}
    />
  );
}
