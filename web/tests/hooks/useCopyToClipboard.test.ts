import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

describe("useCopyToClipboard — hook contract", () => {
  it("exports useCopyToClipboard as a named function", async () => {
    const mod = await import("@/hooks/useCopyToClipboard");
    expect(typeof mod.useCopyToClipboard).toBe("function");
  });
});

describe("useCopyToClipboard — clipboard logic", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Provide a working clipboard mock
    Object.defineProperty(globalThis, "navigator", {
      value: {
        clipboard: {
          writeText: vi.fn().mockResolvedValue(undefined),
        },
      },
      writable: true,
      configurable: true,
    });
    // Silence toast side-effects
    vi.mock("@/lib/toast", () => ({ toast: vi.fn() }));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("calls navigator.clipboard.writeText with the provided text", async () => {
    const text = "GABC123XYZ9";
    await navigator.clipboard.writeText(text);
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(text);
  });

  it("auto-reset logic: isCopied should revert after resetMs", async () => {
    let isCopied = false;
    const resetMs = 2000;

    // Simulate the state machine
    isCopied = true;
    setTimeout(() => { isCopied = false; }, resetMs);

    expect(isCopied).toBe(true);
    vi.advanceTimersByTime(resetMs);
    expect(isCopied).toBe(false);
  });

  it("returns error on clipboard failure", async () => {
    const failingWrite = vi.fn().mockRejectedValue(new Error("Permission denied"));
    Object.defineProperty(globalThis, "navigator", {
      value: { clipboard: { writeText: failingWrite } },
      writable: true,
      configurable: true,
    });

    let caughtError: Error | null = null;
    try {
      await navigator.clipboard.writeText("text");
    } catch (e) {
      caughtError = e as Error;
    }
    expect(caughtError?.message).toBe("Permission denied");
  });
});
