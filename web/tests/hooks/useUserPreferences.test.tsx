import { describe, it, expect, beforeEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import {
  useUserPreferences,
  readPreferences,
  mergePreferences,
  DEFAULT_PREFERENCES,
  STORAGE_KEY,
} from "@/hooks/useUserPreferences";

beforeEach(() => {
  window.localStorage.clear();
});

describe("mergePreferences", () => {
  it("returns defaults for missing keys", () => {
    expect(mergePreferences({})).toEqual(DEFAULT_PREFERENCES);
  });

  it("ignores out-of-range values", () => {
    const merged = mergePreferences({
      addressFormat: "weird",
      xlmDecimals: 5,
      dateFormat: "bogus",
      notifyOnFunding: "yes",
    });
    expect(merged.addressFormat).toBe(DEFAULT_PREFERENCES.addressFormat);
    expect(merged.xlmDecimals).toBe(DEFAULT_PREFERENCES.xlmDecimals);
    expect(merged.dateFormat).toBe(DEFAULT_PREFERENCES.dateFormat);
    expect(merged.notifyOnFunding).toBe(DEFAULT_PREFERENCES.notifyOnFunding);
  });

  it("keeps valid values", () => {
    const merged = mergePreferences({
      addressFormat: "medium",
      xlmDecimals: 2,
      dateFormat: "absolute",
      notifyOnFunding: false,
    });
    expect(merged.addressFormat).toBe("medium");
    expect(merged.xlmDecimals).toBe(2);
    expect(merged.dateFormat).toBe("absolute");
    expect(merged.notifyOnFunding).toBe(false);
  });
});

describe("readPreferences", () => {
  it("returns defaults when storage is empty", () => {
    expect(readPreferences()).toEqual(DEFAULT_PREFERENCES);
  });

  it("returns defaults when storage is malformed", () => {
    window.localStorage.setItem(STORAGE_KEY, "{not json");
    expect(readPreferences()).toEqual(DEFAULT_PREFERENCES);
  });

  it("reads from storage", () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ ...DEFAULT_PREFERENCES, addressFormat: "medium" }),
    );
    expect(readPreferences().addressFormat).toBe("medium");
  });
});

describe("useUserPreferences", () => {
  it("starts with defaults on first render", () => {
    const { result } = renderHook(() => useUserPreferences());
    expect(result.current.preferences).toEqual(DEFAULT_PREFERENCES);
  });

  it("persists a patch to localStorage and reflects it in state", () => {
    const { result } = renderHook(() => useUserPreferences());
    act(() => {
      result.current.setPreferences({ addressFormat: "medium", xlmDecimals: 2 });
    });
    expect(result.current.preferences.addressFormat).toBe("medium");
    expect(result.current.preferences.xlmDecimals).toBe(2);
    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY)!);
    expect(stored.addressFormat).toBe("medium");
  });

  it("survives a 'page refresh' (re-mount reads from storage)", () => {
    const { result, unmount } = renderHook(() => useUserPreferences());
    act(() => result.current.setPreferences({ dateFormat: "absolute" }));
    unmount();
    const { result: result2 } = renderHook(() => useUserPreferences());
    // useEffect synchronously runs on mount under @testing-library/react
    expect(result2.current.preferences.dateFormat).toBe("absolute");
  });

  it("reset wipes storage and reverts to defaults", () => {
    const { result } = renderHook(() => useUserPreferences());
    act(() => result.current.setPreferences({ notifyOnFunding: false }));
    expect(result.current.preferences.notifyOnFunding).toBe(false);
    act(() => result.current.reset());
    expect(result.current.preferences).toEqual(DEFAULT_PREFERENCES);
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });
});
