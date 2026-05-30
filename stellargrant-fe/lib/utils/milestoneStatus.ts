import type { Milestone } from "@/types";

export type MilestoneStatus =
  | "Paid"
  | "Approved"
  | "Submitted"
  | "Rejected"
  | "Overdue"
  | "Due Soon"
  | "Pending";

export function getMilestoneStatus(m: Milestone): MilestoneStatus {
  if (m.paid) return "Paid";
  if (m.approved) return "Approved";
  if (m.submitted) return "Submitted";
  if (m.overdue) return "Overdue";
  if ((m.daysUntilDeadline ?? Infinity) <= 7) return "Due Soon";
  return "Pending";
}

export function getMilestoneStatusClass(status: MilestoneStatus): string {
  switch (status) {
    case "Paid":
      return "bg-success/20 text-success border-success/40";
    case "Approved":
      return "bg-accent-secondary/20 text-accent-secondary border-accent-secondary/40";
    case "Submitted":
      return "bg-warning/20 text-warning border-warning/40";
    case "Rejected":
      return "bg-danger/20 text-danger border-danger/40";
    case "Overdue":
      return "bg-danger/20 text-danger border-danger/40";
    case "Due Soon":
      return "bg-warning/15 text-warning border-warning/30";
    default:
      return "bg-surface text-text-muted border-border-color";
  }
}

export function getMilestoneNodeClass(status: MilestoneStatus): string {
  switch (status) {
    case "Paid":
      return "bg-success border-success";
    case "Approved":
      return "bg-accent-secondary border-accent-secondary";
    case "Submitted":
      return "bg-warning border-warning";
    case "Rejected":
    case "Overdue":
      return "bg-danger border-danger";
    case "Due Soon":
      return "bg-warning border-warning";
    default:
      return "bg-bg-primary border-border-color";
  }
}