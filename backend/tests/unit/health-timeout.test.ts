import { afterEach, describe, expect, it, vi } from "vitest";
import { withTimeout } from "../../src/routes/health";

describe("health check timeout", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("rejects an RPC check that exceeds its timeout", async () => {
    vi.useFakeTimers();
    const result = withTimeout(new Promise<number>(() => {}), 100);
    const rejection = expect(result).rejects.toThrow(
      "Operation timed out after 100 ms",
    );

    await vi.advanceTimersByTimeAsync(100);

    await rejection;
  });

  it("clears the timeout when the RPC check completes", async () => {
    vi.useFakeTimers();

    await expect(withTimeout(Promise.resolve(42), 100)).resolves.toBe(42);

    expect(vi.getTimerCount()).toBe(0);
  });
});
