/**
 * Badge Component
 *
 * Generic status / label primitive used throughout the app:
 * milestone statuses, leaderboard ranks, filter chips, etc.
 *
 * Design system rules (from spec):
 *  - IBM Plex Mono font, uppercase, tight letter-spacing
 *  - Sharp corners (rounded-none)
 *  - 1px border at 40% opacity matching the variant colour
 *  - No external animation library required — CSS-only
 *
 * Variants and their design-system colours:
 *  default  → #E8EDF5  (text-primary)
 *  success  → #22C55E
 *  danger   → #EF4444
 *  warning  → #F59E0B
 *  info     → #3B82F6  (stellar blue / accent-secondary)
 *  muted    → #6B7FA3  (text-muted)
 */

import React from "react";

export type BadgeVariant = "default" | "success" | "danger" | "warning" | "info" | "muted";
export type BadgeSize = "sm" | "md";

export interface BadgeProps {
  /** Visual variant controlling colour */
  variant?: BadgeVariant;
  /** Size — sm for secondary contexts, md (default) for primary display */
  size?: BadgeSize;
  children: React.ReactNode;
  className?: string;
}

// Per-variant colour palette — text colour + 40 % opacity border
const VARIANT_STYLES: Record<BadgeVariant, { color: string; borderColor: string }> = {
  default: { color: "#E8EDF5", borderColor: "rgba(232,237,245,0.4)" },
  success:  { color: "#22C55E", borderColor: "rgba(34,197,94,0.4)"  },
  danger:   { color: "#EF4444", borderColor: "rgba(239,68,68,0.4)"  },
  warning:  { color: "#F59E0B", borderColor: "rgba(245,158,11,0.4)" },
  info:     { color: "#3B82F6", borderColor: "rgba(59,130,246,0.4)" },
  muted:    { color: "#6B7FA3", borderColor: "rgba(107,127,163,0.4)"},
};

// Size: padding + font size
const SIZE_CLASSES: Record<BadgeSize, string> = {
  sm: "px-1.5 py-px text-[9px]",
  md: "px-2 py-0.5 text-[10px]",
};

export function Badge({
  variant = "default",
  size = "md",
  children,
  className = "",
}: BadgeProps) {
  const { color, borderColor } = VARIANT_STYLES[variant];

  return (
    <span
      className={[
        "inline-flex items-center",
        "font-mono uppercase tracking-widest",
        "rounded-none border",
        SIZE_CLASSES[size],
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      style={{ color, borderColor }}
    >
      {children}
    </span>
  );
}
