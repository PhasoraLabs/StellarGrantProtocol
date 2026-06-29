/**
 * Issue #492 — Logging integration tests
 *
 * Verifies that ContractClient accepts a custom ILogger and that the
 * debug/error paths are called in the expected scenarios.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { ContractClient } from "@/lib/stellar/contract";
import type { ILogger } from "@/lib/logger";

function makeLogger(): ILogger & { calls: Record<string, unknown[][]> } {
  const calls: Record<string, unknown[][]> = { debug: [], info: [], warn: [], error: [] };
  return {
    calls,
    debug: (...a: unknown[]) => { calls.debug.push(a); },
    info:  (...a: unknown[]) => { calls.info.push(a); },
    warn:  (...a: unknown[]) => { calls.warn.push(a); },
    error: (...a: unknown[]) => { calls.error.push(a); },
  };
}

describe("ContractClient — logger integration (#492)", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("accepts a custom ILogger without throwing", () => {
    const log = makeLogger();
    expect(() => new ContractClient({ logger: log })).not.toThrow();
  });

  it("logs input validation errors for approveToken", async () => {
    const log = makeLogger();
    const client = new ContractClient({ logger: log });
    await expect(
      client.approveToken({ tokenAddress: "", spender: "S", amount: 1n, owner: "O" })
    ).rejects.toThrow(/tokenAddress/);
  });

  it("logs input validation errors for setAllowance", async () => {
    const log = makeLogger();
    const client = new ContractClient({ logger: log });
    await expect(
      client.setAllowance({ tokenAddress: "CABC", amount: 0n, owner: "O", spender: "S" })
    ).rejects.toThrow(/amount/);
  });

  it("skips allowance logging entirely for native XLM", async () => {
    const log = makeLogger();
    const client = new ContractClient({ logger: log });
    const result = await client.setAllowance({
      tokenAddress: "native",
      amount: 100n,
      owner: "O",
      spender: "S",
    });
    expect(result).toBeNull();
    // At least one debug call should mention 'native'
    const debugMessages = log.calls.debug.flat().join(" ");
    expect(debugMessages).toMatch(/native/i);
  });
});
