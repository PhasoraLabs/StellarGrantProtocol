"use client";

import { useEffect, useState } from "react";

type TimestampInput = Date | bigint | number | string | null;

function toDate(value: TimestampInput): Date | null {
  if (value === null || value === undefined) return null;
  if (value instanceof Date) return value;
  if (typeof value === "bigint") {
    // bigint unix timestamp in seconds
    return new Date(Number(value) * 1000);
  }
  if (typeof value === "number") return new Date(value);
  if (typeof value === "string") {
    const d = new Date(value);
    return isNaN(d.getTime()) ? null : d;
  }
  return null;
}

function formatRelative(date: Date): string {
  const diffMs = Date.now() - date.getTime();
  const diffSec = Math.round(diffMs / 1000);
  const abs = Math.abs(diffSec);

  if (abs < 45) return "just now";
  if (abs < 3600) return `${Math.round(abs / 60)}m ago`;
  if (abs < 86400) return `${Math.round(abs / 3600)}h ago`;
  if (abs < 604800) return `${Math.round(abs / 86400)}d ago`;
  return date.toLocaleDateString();
}

export function useRelativeTime(timestamp: TimestampInput): string {
  const [label, setLabel] = useState<string>(() => {
    const d = toDate(timestamp);
    return d ? formatRelative(d) : "";
  });

  useEffect(() => {
    const target = toDate(timestamp);
    if (!target) return;
    const tick = () => setLabel(formatRelative(target));
    queueMicrotask(tick);
    const id = setInterval(tick, 60_000);
    return () => clearInterval(id);
  }, [timestamp]);

  return label;
}
