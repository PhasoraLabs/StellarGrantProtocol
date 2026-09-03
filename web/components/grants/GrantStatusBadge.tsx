/**
 * GrantStatusBadge Component
 *
 * Color-coded status chip mapping contract state to UI label.
 * Uses design-system tokens exclusively — no generic Tailwind colours.
 *
 * Contract States:
 * - 0 (Pending)     → muted border + text
 * - 1 (Active)      → accent-secondary border + text
 * - 2 (In Progress) → warning border + text
 * - 3 (Completed)   → success border + text
 * - 4 (Cancelled)   → danger border + text
 */

interface GrantStatusBadgeProps {
  status: number | string;
}

const statusConfig: Record<number, { label: string; classes: string }> = {
  0: {
    label: "Pending",
    classes: "border border-text-muted/40 text-text-muted",
  },
  1: {
    label: "Active",
    classes: "border border-accent-secondary/40 text-accent-secondary",
  },
  2: {
    label: "In Progress",
    classes: "border border-warning/40 text-warning",
  },
  3: {
    label: "Completed",
    classes: "border border-success/40 text-success",
  },
  4: {
    label: "Cancelled",
    classes: "border border-danger/40 text-danger",
  },
};

export function GrantStatusBadge({ status }: GrantStatusBadgeProps) {
  const config = statusConfig[Number(status)] ?? statusConfig[0];

  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded-none font-mono uppercase tracking-widest text-xs ${config.classes}`}
    >
      {config.label}
    </span>
  );
}
