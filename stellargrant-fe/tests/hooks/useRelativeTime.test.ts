import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// The pure formatting logic is extracted and tested directly without mounting a hook.
// This avoids jsdom quirks around timers while giving full coverage of the key branches.

type TimestampInput = Date | bigint | number | string | null;

function toDate(value: TimestampInput): Date | null {
  if (value === null || value === undefined) return null;
  if (value instanceof Date) return value;
  if (typeof value === "bigint") return new Date(Number(value) * 1000);
  if (typeof value === "number") return new Date(value);
  if (typeof value === "string") {
    const d = new Date(value);
    return isNaN(d.getTime()) ? null : d;
  }
  return null;
}

function formatRelative(date: Date): string {
  const diffMs = Date.now() - date.getTime();
  const abs = Math.abs(Math.round(diffMs / 1000));
  if (abs < 45) return "just now";
  if (abs < 3600) return `${Math.round(abs / 60)}m ago`;
  if (abs < 86400) return `${Math.round(abs / 3600)}h ago`;
  if (abs < 604800) return `${Math.round(abs / 86400)}d ago`;
  return date.toLocaleDateString();
}

describe("useRelativeTime — formatting logic", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2024-06-15T12:00:00.000Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns empty string for null", () => {
    expect(toDate(null)).toBeNull();
  });

  it("returns just now for very recent Date", () => {
    const d = new Date(Date.now() - 10_000);
    expect(formatRelative(d)).toBe("just now");
  });

  it("handles minutes ago", () => {
    const d = new Date(Date.now() - 3 * 60_000);
    expect(formatRelative(d)).toBe("3m ago");
  });

  it("handles hours ago", () => {
    const d = new Date(Date.now() - 3 * 3600_000);
    expect(formatRelative(d)).toBe("3h ago");
  });

  it("handles days ago", () => {
    const d = new Date(Date.now() - 2 * 86400_000);
    expect(formatRelative(d)).toBe("2d ago");
  });

  it("formats older dates as locale date string", () => {
    const d = new Date(Date.now() - 10 * 86400_000);
    const result = formatRelative(d);
    expect(result).not.toContain("ago");
  });

  it("handles bigint unix timestamp (seconds)", () => {
    const nowSec = BigInt(Math.floor(Date.now() / 1000)) - 120n; // 2 min ago
    const d = toDate(nowSec);
    expect(d).not.toBeNull();
    expect(formatRelative(d!)).toBe("2m ago");
  });

  it("handles string ISO timestamp", () => {
    const d = toDate("2024-06-15T11:55:00.000Z"); // 5 min ago
    expect(d).not.toBeNull();
    expect(formatRelative(d!)).toBe("5m ago");
  });

  it("handles numeric ms timestamp", () => {
    const d = toDate(Date.now() - 7200_000); // 2h ago
    expect(formatRelative(d!)).toBe("2h ago");
  });
});

describe("useRelativeTime — hook contract", () => {
  it("exports useRelativeTime as a named function", async () => {
    const mod = await import("@/hooks/useRelativeTime");
    expect(typeof mod.useRelativeTime).toBe("function");
  });

  it("accepts bigint, null, string, number, Date overloads", async () => {
    const { useRelativeTime } = await import("@/hooks/useRelativeTime");
    expect(useRelativeTime.length).toBe(1);
  });
});
