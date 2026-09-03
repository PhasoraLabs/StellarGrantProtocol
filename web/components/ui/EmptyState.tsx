"use client";

import React from "react";
import Link from "next/link";

export interface EmptyStateProps {
  title: string;
  description?: string;
  action?: { label: string; href?: string; onClick?: () => void };
  icon?: React.ReactNode;
  size?: "sm" | "md" | "lg";
}

function SatelliteIcon({ size }: { size: "sm" | "md" | "lg" }) {
  const dim = size === "lg" ? 64 : size === "sm" ? 40 : 48;
  return (
    <svg
      width={dim}
      height={dim}
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <rect x="26" y="26" width="12" height="12" rx="1" fill="var(--text-muted)" />
      <line x1="32" y1="26" x2="32" y2="16" stroke="var(--text-muted)" strokeWidth="2" />
      <circle cx="32" cy="14" r="2" fill="var(--text-muted)" />
      <rect x="8" y="28" width="16" height="8" rx="1" fill="var(--text-muted)" opacity="0.6" />
      <line x1="24" y1="32" x2="26" y2="32" stroke="var(--text-muted)" strokeWidth="2" />
      <rect x="40" y="28" width="16" height="8" rx="1" fill="var(--text-muted)" opacity="0.6" />
      <line x1="38" y1="32" x2="40" y2="32" stroke="var(--text-muted)" strokeWidth="2" />
    </svg>
  );
}

const actionClass =
  "inline-flex items-center justify-center border border-accent-primary px-6 py-2 font-orbitron text-sm font-bold uppercase tracking-wider text-accent-primary transition-all duration-300 hover:bg-accent-primary hover:text-bg-primary";

export function EmptyState({ title, description, action, icon, size = "md" }: EmptyStateProps) {
  const titleClass = size === "sm" ? "text-lg" : "text-xl";

  return (
    <div className="flex flex-col items-center justify-center px-4 py-16 text-center">
      <div className="mb-4 text-text-muted">
        {icon ?? <SatelliteIcon size={size} />}
      </div>
      <h3 className={`font-orbitron ${titleClass} mb-2 text-text-primary`}>{title}</h3>
      {description && (
        <p className="mb-6 max-w-sm font-mono text-sm text-text-muted">{description}</p>
      )}
      {action && (
        <div className="mt-4">
          {action.href ? (
            <Link href={action.href} className={actionClass}>
              {action.label}
            </Link>
          ) : (
            <button onClick={action.onClick} className={actionClass}>
              {action.label}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
