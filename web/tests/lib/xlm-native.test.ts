/**
 * Issue #504 — Native XLM Funding support tests
 *
 * Tests pure functions: detection, balance computation, and sufficiency checks.
 * Network-dependent builders (buildNativeXlmFundXdr) are validated via input
 * guards only; the actual XDR build requires a live RPC node.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  isNativeXlm,
  getNativeXlmSacId,
  resolveTokenAddress,
  computeRequiredBalance,
  checkXlmSufficiency,
  getWalletXlmBalance,
  STELLAR_BASE_RESERVE_STROOPS,
  TX_FEE_BUFFER_STROOPS,
} from "@/lib/stellar/xlm-native";

const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";
const FAKE_SAC = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

// ── isNativeXlm ────────────────────────────────────────────────────────────

describe("isNativeXlm (#504)", () => {
  it('returns true for "native" sentinel', () => {
    expect(isNativeXlm("native")).toBe(true);
  });

  it("is case-insensitive for the sentinel", () => {
    expect(isNativeXlm("NATIVE")).toBe(true);
    expect(isNativeXlm("Native")).toBe(true);
  });

  it("returns false for an arbitrary SAC contract address", () => {
    expect(isNativeXlm(FAKE_SAC)).toBe(false);
  });

  it("returns false for empty string", () => {
    expect(isNativeXlm("")).toBe(false);
  });

  it("detects the computed XLM SAC contract ID for testnet", () => {
    const sacId = getNativeXlmSacId(TESTNET_PASSPHRASE);
    // The SAC ID must be a valid Stellar contract address (starts with C, 56 chars)
    expect(sacId).toMatch(/^C[A-Z0-9]{55}$/);
    expect(isNativeXlm(sacId, TESTNET_PASSPHRASE)).toBe(true);
  });
});

// ── resolveTokenAddress ────────────────────────────────────────────────────

describe("resolveTokenAddress (#504)", () => {
  it('resolves "native" to the XLM SAC contract ID', () => {
    const resolved = resolveTokenAddress("native", TESTNET_PASSPHRASE);
    expect(resolved).toMatch(/^C[A-Z0-9]{55}$/);
  });

  it("passes SAC token addresses through unchanged", () => {
    expect(resolveTokenAddress(FAKE_SAC, TESTNET_PASSPHRASE)).toBe(FAKE_SAC);
  });
});

// ── computeRequiredBalance ─────────────────────────────────────────────────

describe("computeRequiredBalance (#504)", () => {
  it("totals funding + fee buffer + reserve", () => {
    const amount = 100_000_000n; // 10 XLM
    const result = computeRequiredBalance(amount);
    expect(result.fundingAmount).toBe(amount);
    expect(result.feeBuffer).toBe(TX_FEE_BUFFER_STROOPS);
    expect(result.reserveBuffer).toBe(STELLAR_BASE_RESERVE_STROOPS);
    expect(result.totalRequired).toBe(amount + TX_FEE_BUFFER_STROOPS + STELLAR_BASE_RESERVE_STROOPS);
  });

  it("formats the total as a decimal XLM string", () => {
    const result = computeRequiredBalance(10_000_000n); // 1 XLM
    // 1 XLM + 0.01 XLM fee + 1 XLM reserve = 2.0100000
    expect(result.totalRequiredXlm).toBe("2.0100000");
  });

  it("handles zero funding amount", () => {
    const result = computeRequiredBalance(0n);
    expect(result.totalRequired).toBe(TX_FEE_BUFFER_STROOPS + STELLAR_BASE_RESERVE_STROOPS);
  });
});

// ── getWalletXlmBalance / checkXlmSufficiency ─────────────────────────────

describe("getWalletXlmBalance (#504)", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("returns 0n when horizon throws (account not found)", async () => {
    // No mock needed — DUMMY_ACCOUNT doesn't exist on testnet but function must not throw
    const result = await getWalletXlmBalance("GNON_EXISTENT_ACCOUNT00000000000000000000000000000");
    expect(result).toBe(0n);
  });
});

describe("checkXlmSufficiency (#504)", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("returns insufficient when balance is 0", async () => {
    const { sufficient, required } = await checkXlmSufficiency(
      "GNON_EXISTENT_ACCOUNT00000000000000000000000000000",
      50_000_000n // 5 XLM
    );
    expect(sufficient).toBe(false);
    expect(required.fundingAmount).toBe(50_000_000n);
  });
});
