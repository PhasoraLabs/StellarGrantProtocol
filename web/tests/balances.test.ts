/**
 * Balance Monitoring Tests
 *
 * Unit tests for parseBalanceToStroops, formatStroops,
 * getGrantBalances, and listenToBalanceChanges.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  parseBalanceToStroops,
  formatStroops,
  getGrantBalances,
  getGrantXlmBalance,
  getGrantTokenBalance,
  listenToBalanceChanges,
  type GrantBalances,
} from "../lib/stellar/balances";

// ── Mock the RPC client ───────────────────────────────────────────────────

vi.mock("../lib/stellar/client", () => ({
  getHorizonClient: vi.fn(),
}));

import { getHorizonClient } from "../lib/stellar/client";

function makeMockHorizon(balances: object[]) {
  return {
    loadAccount: vi.fn().mockResolvedValue({
      balances,
      last_modified_ledger: 123456,
    }),
  };
}

const MOCK_ADDRESS = "GABC1234567890ABCDEF";

// ── parseBalanceToStroops ─────────────────────────────────────────────────

describe("parseBalanceToStroops", () => {
  it("parses whole number balance", () => {
    expect(parseBalanceToStroops("100.0000000")).toBe(1_000_000_000n);
  });

  it("parses fractional balance", () => {
    expect(parseBalanceToStroops("0.0000001")).toBe(1n);
  });

  it("parses balance without fractional part", () => {
    expect(parseBalanceToStroops("50")).toBe(500_000_000n);
  });

  it("parses zero balance", () => {
    expect(parseBalanceToStroops("0.0000000")).toBe(0n);
  });

  it("parses large balance accurately", () => {
    expect(parseBalanceToStroops("1000000.0000000")).toBe(10_000_000_000_000n);
  });
});

// ── formatStroops ────────────────────────────────────────────────────────

describe("formatStroops", () => {
  it("formats whole number stroops", () => {
    expect(formatStroops(1_000_000_000n)).toBe("100.0000000");
  });

  it("formats 1 stroop", () => {
    expect(formatStroops(1n)).toBe("0.0000001");
  });

  it("formats zero", () => {
    expect(formatStroops(0n)).toBe("0.0000000");
  });

  it("is the inverse of parseBalanceToStroops", () => {
    const original = "123.4560000";
    const stroops = parseBalanceToStroops(original);
    expect(formatStroops(stroops)).toBe(original);
  });
});

// ── getGrantBalances ─────────────────────────────────────────────────────

describe("getGrantBalances", () => {
  beforeEach(() => {
    vi.mocked(getHorizonClient).mockReturnValue(
      makeMockHorizon([
        { asset_type: "native", balance: "100.0000000" },
        { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GCENTER...", balance: "500.0000000" },
      ]) as ReturnType<typeof getHorizonClient>
    );
  });

  afterEach(() => vi.clearAllMocks());

  it("returns a GrantBalances snapshot", async () => {
    const result = await getGrantBalances(MOCK_ADDRESS);
    expect(result.contractAddress).toBe(MOCK_ADDRESS);
    expect(result.balances).toHaveLength(2);
    expect(result.fetchedAt).toBeInstanceOf(Date);
  });

  it("places native XLM first", async () => {
    const result = await getGrantBalances(MOCK_ADDRESS);
    expect(result.balances[0].isNative).toBe(true);
    expect(result.balances[0].assetCode).toBe("XLM");
  });

  it("correctly maps XLM balance", async () => {
    const result = await getGrantBalances(MOCK_ADDRESS);
    const xlm = result.balances[0];
    expect(xlm.assetCode).toBe("XLM");
    expect(xlm.assetIssuer).toBe("");
    expect(xlm.balanceStroops).toBe(1_000_000_000n);
    expect(xlm.formatted).toBe("100.0000000");
  });

  it("correctly maps SAC token balance", async () => {
    const result = await getGrantBalances(MOCK_ADDRESS);
    const usdc = result.balances[1];
    expect(usdc.assetCode).toBe("USDC");
    expect(usdc.assetIssuer).toBe("GCENTER...");
    expect(usdc.balanceStroops).toBe(5_000_000_000n);
  });

  it("throws on empty contractAddress", async () => {
    await expect(getGrantBalances("")).rejects.toThrow("contractAddress must not be empty");
  });

  it("throws on whitespace contractAddress", async () => {
    await expect(getGrantBalances("   ")).rejects.toThrow("contractAddress must not be empty");
  });
});

// ── getGrantXlmBalance ───────────────────────────────────────────────────

describe("getGrantXlmBalance", () => {
  beforeEach(() => {
    vi.mocked(getHorizonClient).mockReturnValue(
      makeMockHorizon([
        { asset_type: "native", balance: "42.5000000" },
      ]) as ReturnType<typeof getHorizonClient>
    );
  });

  afterEach(() => vi.clearAllMocks());

  it("returns only the XLM balance", async () => {
    const xlm = await getGrantXlmBalance(MOCK_ADDRESS);
    expect(xlm).not.toBeNull();
    expect(xlm?.assetCode).toBe("XLM");
    expect(xlm?.balanceStroops).toBe(425_000_000n);
  });
});

// ── getGrantTokenBalance ─────────────────────────────────────────────────

describe("getGrantTokenBalance", () => {
  beforeEach(() => {
    vi.mocked(getHorizonClient).mockReturnValue(
      makeMockHorizon([
        { asset_type: "native", balance: "10.0000000" },
        { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "ISSUER1", balance: "200.0000000" },
      ]) as ReturnType<typeof getHorizonClient>
    );
  });

  afterEach(() => vi.clearAllMocks());

  it("finds a token by asset code", async () => {
    const balance = await getGrantTokenBalance(MOCK_ADDRESS, "USDC");
    expect(balance?.assetCode).toBe("USDC");
    expect(balance?.balanceStroops).toBe(2_000_000_000n);
  });

  it("returns null for unknown token", async () => {
    const balance = await getGrantTokenBalance(MOCK_ADDRESS, "BTC");
    expect(balance).toBeNull();
  });

  it("matches token by code and issuer", async () => {
    const balance = await getGrantTokenBalance(MOCK_ADDRESS, "USDC", "ISSUER1");
    expect(balance).not.toBeNull();
  });

  it("returns null when issuer does not match", async () => {
    const balance = await getGrantTokenBalance(MOCK_ADDRESS, "USDC", "WRONG_ISSUER");
    expect(balance).toBeNull();
  });
});

// ── listenToBalanceChanges ────────────────────────────────────────────────

/** Drain all pending microtasks / promise callbacks */
const flushPromises = () => new Promise<void>((r) => setTimeout(r, 50));

describe("listenToBalanceChanges", () => {
  beforeEach(() => {
    vi.mocked(getHorizonClient).mockReturnValue(
      makeMockHorizon([
        { asset_type: "native", balance: "100.0000000" },
      ]) as ReturnType<typeof getHorizonClient>
    );
  });

  afterEach(() => vi.clearAllMocks());

  it("calls onChange on first fetch", async () => {
    const onChange = vi.fn();
    const stop = listenToBalanceChanges(MOCK_ADDRESS, { onChange, pollInterval: 60_000 });

    await flushPromises();

    expect(onChange).toHaveBeenCalledOnce();
    const [current, previous] = onChange.mock.calls[0] as [GrantBalances, null];
    expect(current.contractAddress).toBe(MOCK_ADDRESS);
    expect(previous).toBeNull();

    stop();
  });

  it("stops polling after cleanup — no extra calls after stop()", async () => {
    const onChange = vi.fn();
    const stop = listenToBalanceChanges(MOCK_ADDRESS, { onChange, pollInterval: 60_000 });

    await flushPromises();
    const countBeforeStop = onChange.mock.calls.length;

    stop();
    await flushPromises();

    expect(onChange.mock.calls.length).toBe(countBeforeStop);
  });

  it("calls onError when Horizon fails", async () => {
    vi.mocked(getHorizonClient).mockReturnValue({
      loadAccount: vi.fn().mockRejectedValue(new Error("Horizon failure")),
    } as ReturnType<typeof getHorizonClient>);

    const onError = vi.fn();
    const stop = listenToBalanceChanges(MOCK_ADDRESS, {
      onChange: vi.fn(),
      onError,
      pollInterval: 60_000,
    });

    await flushPromises();

    expect(onError).toHaveBeenCalledWith(expect.any(Error));

    stop();
  });
});


