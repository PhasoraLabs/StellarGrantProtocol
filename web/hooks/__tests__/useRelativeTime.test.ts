import { renderHook } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useRelativeTime } from "../useRelativeTime";

describe("useRelativeTime", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("formats timestamps less than 45 seconds ago as 'just now'", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const { result: result20s } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 20_000))
    );
    expect(result20s.current).toBe("just now");

    const { result: result44s } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 44_000))
    );
    expect(result44s.current).toBe("just now");
  });

  it("formats timestamps in minutes", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const { result: result1m } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 60_000))
    );
    expect(result1m.current).toMatch(/^1m ago$/);

    const { result: result30m } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 30 * 60_000))
    );
    expect(result30m.current).toMatch(/^30m ago$/);

    const { result: result59m } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 59 * 60_000))
    );
    expect(result59m.current).toMatch(/^59m ago$/);
  });

  it("formats timestamps in hours", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const { result: result1h } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 3600_000))
    );
    expect(result1h.current).toMatch(/^1h ago$/);

    const { result: result12h } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 12 * 3600_000))
    );
    expect(result12h.current).toMatch(/^12h ago$/);

    const { result: result23h } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 23 * 3600_000))
    );
    expect(result23h.current).toMatch(/^23h ago$/);
  });

  it("formats timestamps in days", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const { result: result1d } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 86400_000))
    );
    expect(result1d.current).toMatch(/^1d ago$/);

    const { result: result3d } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 3 * 86400_000))
    );
    expect(result3d.current).toMatch(/^3d ago$/);

    const { result: result6d } = renderHook(() =>
      useRelativeTime(new Date(now.getTime() - 6 * 86400_000))
    );
    expect(result6d.current).toMatch(/^6d ago$/);
  });

  it("formats timestamps older than 7 days as absolute date", () => {
    const now = new Date("2025-01-15");
    vi.setSystemTime(now);

    const oldDate = new Date("2025-01-01");
    const { result } = renderHook(() => useRelativeTime(oldDate));

    expect(result.current).toBe(oldDate.toLocaleDateString());
  });

  it("handles future dates", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const futureDate = new Date(now.getTime() + 60_000);
    const { result } = renderHook(() => useRelativeTime(futureDate));

    expect(result.current).toBe("just now");
  });

  it("handles bigint unix timestamps in seconds", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const bigintTimestamp = BigInt(Math.floor(now.getTime() / 1000) - 60);
    const { result } = renderHook(() => useRelativeTime(bigintTimestamp));

    expect(result.current).toMatch(/^1m ago$/);
  });

  it("handles number timestamps in milliseconds", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const numberTimestamp = now.getTime() - 120_000;
    const { result } = renderHook(() => useRelativeTime(numberTimestamp));

    expect(result.current).toMatch(/^2m ago$/);
  });

  it("handles string timestamps", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const stringDate = new Date(now.getTime() - 180_000).toISOString();
    const { result } = renderHook(() => useRelativeTime(stringDate));

    expect(result.current).toMatch(/^3m ago$/);
  });

  it("handles null/undefined input gracefully", () => {
    const { result: resultNull } = renderHook(() => useRelativeTime(null));
    expect(resultNull.current).toBe("");

    const { result: resultUndefined } = renderHook(() =>
      useRelativeTime(undefined as unknown as string)
    );
    expect(resultUndefined.current).toBe("");
  });

  it("handles invalid string input gracefully", () => {
    const { result } = renderHook(() => useRelativeTime("not a date"));
    expect(result.current).toBe("");
  });

  it("updates periodically as time passes", () => {
    const now = new Date();
    vi.setSystemTime(now);

    const timestamp = new Date(now.getTime() - 60_000);
    const { result } = renderHook(() => useRelativeTime(timestamp));

    expect(result.current).toMatch(/^1m ago$/);

    vi.advanceTimersByTime(60_000);
    expect(result.current).toMatch(/^2m ago$/);

    vi.advanceTimersByTime(60_000);
    expect(result.current).toMatch(/^3m ago$/);
  });
});
