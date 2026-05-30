import { describe, it, expect, beforeEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import {
  useAddressFormat,
  formatAddressFor,
} from "@/hooks/useAddressFormat";
import { useUserPreferences, STORAGE_KEY } from "@/hooks/useUserPreferences";

const FULL = "GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";

beforeEach(() => {
  window.localStorage.clear();
});

describe("formatAddressFor", () => {
  it("short → 6 + 4", () => {
    expect(formatAddressFor("short", FULL)).toBe(`${FULL.slice(0, 6)}…${FULL.slice(-4)}`);
  });

  it("medium → 8 + 6", () => {
    expect(formatAddressFor("medium", FULL)).toBe(`${FULL.slice(0, 8)}…${FULL.slice(-6)}`);
  });

  it("full → unchanged", () => {
    expect(formatAddressFor("full", FULL)).toBe(FULL);
  });

  it("does not truncate addresses that are already short", () => {
    expect(formatAddressFor("short", "GABC")).toBe("GABC");
    expect(formatAddressFor("medium", "GABC")).toBe("GABC");
  });

  it("passes through an empty string", () => {
    expect(formatAddressFor("short", "")).toBe("");
  });
});

describe("useAddressFormat", () => {
  it("uses the short format by default", () => {
    const { result } = renderHook(() => useAddressFormat());
    expect(result.current(FULL)).toBe(`${FULL.slice(0, 6)}…${FULL.slice(-4)}`);
  });

  it("updates when the user switches to medium", () => {
    const { result } = renderHook(() => ({
      format: useAddressFormat(),
      prefs: useUserPreferences(),
    }));
    act(() => result.current.prefs.setPreferences({ addressFormat: "medium" }));
    expect(result.current.format(FULL)).toBe(`${FULL.slice(0, 8)}…${FULL.slice(-6)}`);
  });

  it("reflects 'full' from stored preferences on mount", () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ addressFormat: "full" }),
    );
    const { result } = renderHook(() => useAddressFormat());
    expect(result.current(FULL)).toBe(FULL);
  });
});
