/**
 * DatePicker
 *
 * Styled wrapper around the native <input type="date"> that applies the
 * design-system tokens (dark theme, white calendar icon) and shows a relative
 * label ("In 45 days" / "In 5 days" / "This date is in the past") beneath the
 * input once a date is selected.
 */

"use client";

import React from "react";

export interface DatePickerProps {
  value: string; // ISO date "YYYY-MM-DD"
  onChange: (value: string) => void;
  label?: string;
  error?: string;
  min?: string;
  max?: string;
  disabled?: boolean;
  placeholder?: string; // used as aria-label
}

export type RelativeDateTone = "muted" | "warning" | "danger";

export interface RelativeDateLabel {
  text: string;
  tone: RelativeDateTone;
}

const DAY_MS = 24 * 60 * 60 * 1000;

/** Parse a "YYYY-MM-DD" string into a UTC-midnight epoch (NaN if invalid). */
function parseIsoDateUtc(value: string): number {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return NaN;
  return Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
}

/** Today's date as an ISO "YYYY-MM-DD" string (used as the default `min`). */
export function todayIso(now: Date = new Date()): string {
  return now.toISOString().split("T")[0];
}

/**
 * Compute the relative label for a selected date. Returns null when no/invalid
 * date is given.
 *  - past:            danger "This date is in the past"
 *  - today:           warning "Today"
 *  - within 7 days:   warning "In N days"
 *  - 7+ days ahead:   muted "In N days"
 */
export function computeRelativeDateLabel(
  value: string,
  now: Date = new Date(),
): RelativeDateLabel | null {
  const target = parseIsoDateUtc(value);
  if (Number.isNaN(target)) return null;

  const todayUtc = parseIsoDateUtc(todayIso(now));
  const diffDays = Math.round((target - todayUtc) / DAY_MS);

  if (diffDays < 0) {
    return { text: "This date is in the past", tone: "danger" };
  }
  if (diffDays === 0) {
    return { text: "Today", tone: "warning" };
  }
  const text = `In ${diffDays} ${diffDays === 1 ? "day" : "days"}`;
  return { text, tone: diffDays < 7 ? "warning" : "muted" };
}

const TONE_CLASS: Record<RelativeDateTone, string> = {
  muted: "text-text-muted",
  warning: "text-warning",
  danger: "text-danger",
};

export function DatePicker({
  value,
  onChange,
  label,
  error,
  min,
  max,
  disabled = false,
  placeholder,
}: DatePickerProps) {
  const effectiveMin = min ?? todayIso();
  const relative = computeRelativeDateLabel(value);

  const borderClass = error
    ? "border-danger"
    : "border-border-color focus:border-accent-secondary";

  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label className="font-mono text-sm text-text-primary">{label}</label>
      )}
      <input
        type="date"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        min={effectiveMin}
        max={max}
        disabled={disabled}
        aria-label={placeholder ?? label}
        aria-invalid={error ? true : undefined}
        className={`datepicker-input bg-surface border ${borderClass} rounded-none font-mono text-sm text-text-primary px-3 py-2 outline-none transition-colors disabled:cursor-not-allowed disabled:opacity-50`}
      />
      {error ? (
        <span className="font-mono text-xs text-danger">{error}</span>
      ) : (
        relative && (
          <span className={`font-mono text-xs ${TONE_CLASS[relative.tone]}`}>{relative.text}</span>
        )
      )}
    </div>
  );
}
