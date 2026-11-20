/**
 * Issue #493 — Token Allowance Management tests
 *
 * Tests getAllowance / setAllowance / ensureAllowance logic, including the
 * native XLM shortcut paths and input validation.
 */

import { describe, it, expect } from "vitest";
import { ContractClient } from "@/lib/stellar/contract";
import { isNativeXlmAddress } from "@/lib/stellar/contract";

const client = new ContractClient();

const TOKEN = "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA";
const OWNER = "GOWNER00000000000000000000000000000000000000000000000000";
const SPENDER = "GSPEND00000000000000000000000000000000000000000000000000";

// ── isNativeXlmAddress ───────────────────────────────────────────────────

describe("isNativeXlmAddress (#493 / #504)", () => {
  it('returns true for "native" sentinel (case-insensitive)', () => {
    expect(isNativeXlmAddress("native")).toBe(true);
    expect(isNativeXlmAddress("NATIVE")).toBe(true);
  });

  it("returns false for a regular SAC token address", () => {
    expect(isNativeXlmAddress(TOKEN)).toBe(false);
  });

  it("returns false for an empty string", () => {
    expect(isNativeXlmAddress("")).toBe(false);
  });
});

// ── getAllowance ─────────────────────────────────────────────────────────

describe("ContractClient.getAllowance (#493)", () => {
  it("returns 0n for native XLM without calling RPC", async () => {
    const result = await client.getAllowance({
      tokenAddress: "native",
      owner: OWNER,
      spender: SPENDER,
    });
    expect(result).toBe(0n);
  });
});

// ── setAllowance ─────────────────────────────────────────────────────────

describe("ContractClient.setAllowance (#493)", () => {
  it("returns null for native XLM (no allowance needed)", async () => {
    const result = await client.setAllowance({
      tokenAddress: "native",
      amount: 1_000_000n,
      owner: OWNER,
      spender: SPENDER,
    });
    expect(result).toBeNull();
  });

  it("throws when amount is 0", async () => {
    await expect(
      client.setAllowance({ tokenAddress: TOKEN, amount: 0n, owner: OWNER, spender: SPENDER })
    ).rejects.toThrow(/amount/);
  });

  it("throws when owner is empty", async () => {
    await expect(
      client.setAllowance({ tokenAddress: TOKEN, amount: 100n, owner: "", spender: SPENDER })
    ).rejects.toThrow(/owner/);
  });

  it("throws when spender is empty", async () => {
    await expect(
      client.setAllowance({ tokenAddress: TOKEN, amount: 100n, owner: OWNER, spender: "" })
    ).rejects.toThrow(/spender/);
  });
});

// ── ensureAllowance ───────────────────────────────────────────────────────

describe("ContractClient.ensureAllowance (#493)", () => {
  it("returns null for native XLM immediately", async () => {
    const result = await client.ensureAllowance({
      tokenAddress: "native",
      requiredAmount: 5_000_000n,
      owner: OWNER,
      spender: SPENDER,
    });
    expect(result).toBeNull();
  });
});
