/**
 * ToggleSwitch — Issue #386.
 *
 * A sliding pill toggle styled with design system tokens. Renders an accessible
 * checkbox `<input>` underneath the styled track so keyboard / screen reader
 * use just works.
 */

"use client";

import { useId } from "react";

export interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
}

export function ToggleSwitch({
  checked,
  onChange,
  label,
  description,
  disabled,
}: ToggleSwitchProps) {
  const id = useId();
  return (
    <label
      htmlFor={id}
      className={`flex items-start justify-between gap-4 py-3 cursor-pointer ${
        disabled ? "opacity-50 cursor-not-allowed" : ""
      }`}
    >
      <div className="min-w-0">
        <span className="block font-mono text-sm text-text-primary">{label}</span>
        {description && (
          <span className="block font-mono text-xs text-text-muted mt-1">{description}</span>
        )}
      </div>
      <span className="relative inline-flex shrink-0 items-center">
        <input
          id={id}
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.target.checked)}
          className="peer sr-only"
        />
        <span
          aria-hidden="true"
          className={`block h-6 w-11 rounded-none border border-border-color transition-colors ${
            checked ? "bg-accent-primary" : "bg-surface"
          } peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-accent-secondary`}
        />
        <span
          aria-hidden="true"
          className={`absolute left-0.5 top-0.5 h-5 w-5 rounded-none bg-bg-primary transition-transform ${
            checked ? "translate-x-5" : "translate-x-0"
          }`}
        />
      </span>
    </label>
  );
}
