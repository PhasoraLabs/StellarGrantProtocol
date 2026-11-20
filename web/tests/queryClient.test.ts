import { describe, it, expect } from "vitest";
import { queryClient } from "@/lib/queryClient";

describe("queryClient default options", () => {
  const q = queryClient.getDefaultOptions().queries!;

  it("uses a 5 minute stale time and 10 minute gc time", () => {
    expect(q.staleTime).toBe(5 * 60 * 1000);
    expect(q.gcTime).toBe(10 * 60 * 1000);
  });

  it("retries once and does not refetch on window focus", () => {
    expect(q.retry).toBe(1);
    expect(q.refetchOnWindowFocus).toBe(false);
  });
});
