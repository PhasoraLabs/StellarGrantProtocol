/**
 * RadioGroup — Issue #386.
 *
 * A horizontal group of pill-style radio buttons. Backed by hidden native
 * `<input type="radio">` elements so the group is fully accessible (keyboard
 * navigation, screen reader announcements).
 */

"use client";

import { useId } from "react";

export interface RadioOption<T extends string | number> {
  value: T;
  label: string;
}

export interface RadioGroupProps<T extends string | number> {
  label: string;
  value: T;
  onChange: (value: T) => void;
  options: ReadonlyArray<RadioOption<T>>;
  name?: string;
}

export function RadioGroup<T extends string | number>({
  label,
  value,
  onChange,
  options,
  name,
}: RadioGroupProps<T>) {
  const fallbackName = useId();
  const groupName = name ?? fallbackName;
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <span className="font-mono text-sm text-text-primary">{label}</span>
      <div role="radiogroup" aria-label={label} className="flex gap-2">
        {options.map((opt) => {
          const id = `${groupName}-${opt.value}`;
          const selected = opt.value === value;
          return (
            <label
              key={String(opt.value)}
              htmlFor={id}
              className={`cursor-pointer rounded-none border px-3 py-1.5 font-mono text-xs uppercase tracking-wider transition-colors ${
                selected
                  ? "border-accent-primary bg-accent-primary text-bg-primary"
                  : "border-border-color bg-surface text-text-muted hover:text-text-primary"
              }`}
            >
              <input
                id={id}
                type="radio"
                name={groupName}
                value={String(opt.value)}
                checked={selected}
                onChange={() => onChange(opt.value)}
                className="sr-only"
              />
              {opt.label}
            </label>
          );
        })}
      </div>
    </div>
  );
}
