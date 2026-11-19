import { describe, it, expect } from "vitest";
import { RECONNECT_OPTIONS } from "@/hooks/useSocket";

describe("useSocket reconnect options", () => {
  it("retries indefinitely with capped exponential backoff", () => {
    expect(RECONNECT_OPTIONS.reconnection).toBe(true);
    expect(RECONNECT_OPTIONS.reconnectionAttempts).toBe(Infinity);
    expect(RECONNECT_OPTIONS.reconnectionDelay).toBeLessThan(RECONNECT_OPTIONS.reconnectionDelayMax);
    expect(RECONNECT_OPTIONS.randomizationFactor).toBeGreaterThan(0);
  });
});
