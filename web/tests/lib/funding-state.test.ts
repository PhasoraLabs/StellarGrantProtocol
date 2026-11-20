import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  fetchGrantFundingState,
  remainingCapacity,
} from "@/lib/grants/funding";

const fetchMock = vi.fn();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("remainingCapacity", () => {
  it("returns budget minus funded, clamped at zero", () => {
    expect(remainingCapacity({ budget: 100n, funded: 40n })).toBe(60n);
    expect(remainingCapacity({ budget: 100n, funded: 100n })).toBe(0n);
    expect(remainingCapacity({ budget: 100n, funded: 120n })).toBe(0n);
  });
});

describe("fetchGrantFundingState", () => {
  it("loads fresh budget/funded via the API proxy", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({ grant: { budget: "250", funded: "100" } }),
        { status: 200 },
      ),
    );

    const state = await fetchGrantFundingState("42");
    expect(fetchMock.mock.calls[0][0]).toBe("/api/grants/42");
    expect(state).toEqual({ budget: 250n, funded: 100n });
  });

  it("returns null when the proxy errors", async () => {
    fetchMock.mockResolvedValueOnce(new Response("nope", { status: 503 }));
    expect(await fetchGrantFundingState("1")).toBeNull();
  });
});
