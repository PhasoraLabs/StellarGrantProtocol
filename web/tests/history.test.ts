/**
 * Transaction History Retrieval Tests — Issue #256
 *
 * All Horizon network calls are mocked via vi.mock() — no live RPC needed.
 * Tests cover:
 *   - getTransactionHistory()  pagination, ordering, cursor forwarding
 *   - getGrantHistory()        grant-ID filtering via memo convention
 *   - parseOperation()         operation-type mapping from function names
 *   - Edge cases               empty results, missing contractId, limit clamping
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  getTransactionHistory,
  getGrantHistory,
} from "../lib/stellar/history";
import type { GrantHistoryRecord } from "../lib/stellar/history";

// ── Horizon mock ──────────────────────────────────────────────────────────────

type RawTx = {
  hash: string;
  created_at: string;
  successful: boolean;
  source_account: string;
  fee_charged: string;
  paging_token: string;
  memo?: string;
  envelope_xdr?: string;
};

function makeTx(overrides: Partial<RawTx> & { hash: string }): RawTx {
  return {
    created_at: "2026-01-01T00:00:00Z",
    successful: true,
    source_account: "GABC",
    fee_charged: "100",
    paging_token: overrides.hash,
    ...overrides,
  };
}

/** Build a mock Horizon builder that resolves with the given records. */
function makeHorizonMock(records: RawTx[]) {
  const builder = {
    forAccount: vi.fn().mockReturnThis(),
    limit: vi.fn().mockReturnThis(),
    order: vi.fn().mockReturnThis(),
    cursor: vi.fn().mockReturnThis(),
    call: vi.fn().mockResolvedValue({ records }),
  };

  return {
    transactions: vi.fn(() => builder),
    _builder: builder,
  };
}

// Mock the client module so getHorizonClient() returns our mock
vi.mock("../lib/stellar/client", () => {
  let currentMock: ReturnType<typeof makeHorizonMock> | null = null;

  return {
    getHorizonClient: () => {
      if (!currentMock) throw new Error("horizonMock not initialised");
      return currentMock;
    },
    __setMock: (m: ReturnType<typeof makeHorizonMock>) => {
      currentMock = m;
    },
  };
});

// Helper to inject a new mock before each test
async function setHorizonMock(records: RawTx[]) {
  const mod = await import("../lib/stellar/client");
  const setter = (mod as unknown as { __setMock: (m: ReturnType<typeof makeHorizonMock>) => void }).__setMock;
  const mock = makeHorizonMock(records);
  setter(mock);
  return mock;
}

// ── getTransactionHistory ────────────────────────────────────────────────────

describe("getTransactionHistory", () => {
  it("returns records mapped to GrantHistoryRecord shape", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", memo: "grant:5", successful: true }),
      makeTx({ hash: "tx2", successful: false }),
    ]);

    const { records } = await getTransactionHistory("GABC");

    expect(records).toHaveLength(2);
    const [r1, r2] = records as [GrantHistoryRecord, GrantHistoryRecord];
    expect(r1.txHash).toBe("tx1");
    expect(r1.successful).toBe(true);
    expect(r1.grantId).toBe("5");
    expect(r2.txHash).toBe("tx2");
    expect(r2.successful).toBe(false);
    expect(r2.grantId).toBeUndefined();
  });

  it("passes the address to forAccount()", async () => {
    const mock = await setHorizonMock([]);
    await getTransactionHistory("GXYZ");
    expect(mock._builder.forAccount).toHaveBeenCalledWith("GXYZ");
  });

  it("uses desc order by default", async () => {
    const mock = await setHorizonMock([]);
    await getTransactionHistory("GABC");
    expect(mock._builder.order).toHaveBeenCalledWith("desc");
  });

  it("respects the order option", async () => {
    const mock = await setHorizonMock([]);
    await getTransactionHistory("GABC", { order: "asc" });
    expect(mock._builder.order).toHaveBeenCalledWith("asc");
  });

  it("respects the limit option", async () => {
    const mock = await setHorizonMock([]);
    await getTransactionHistory("GABC", { limit: 10 });
    expect(mock._builder.limit).toHaveBeenCalledWith(10);
  });

  it("clamps limit to 200", async () => {
    const mock = await setHorizonMock([]);
    await getTransactionHistory("GABC", { limit: 9999 });
    expect(mock._builder.limit).toHaveBeenCalledWith(200);
  });

  it("passes cursor to builder when provided", async () => {
    const mock = await setHorizonMock([]);
    await getTransactionHistory("GABC", { cursor: "tok-99" });
    expect(mock._builder.cursor).toHaveBeenCalledWith("tok-99");
  });

  it("does not call cursor() when no cursor option given", async () => {
    const mock = await setHorizonMock([]);
    await getTransactionHistory("GABC");
    expect(mock._builder.cursor).not.toHaveBeenCalled();
  });

  it("returns nextCursor from the last record's paging_token", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", paging_token: "page-1" }),
      makeTx({ hash: "tx2", paging_token: "page-2" }),
    ]);

    const { nextCursor } = await getTransactionHistory("GABC");
    expect(nextCursor).toBe("page-2");
  });

  it("returns undefined nextCursor for empty results", async () => {
    await setHorizonMock([]);
    const { nextCursor } = await getTransactionHistory("GABC");
    expect(nextCursor).toBeUndefined();
  });

  it("maps operationType to unknown_contract_call when no function name", async () => {
    await setHorizonMock([makeTx({ hash: "tx1" })]);
    const { records } = await getTransactionHistory("GABC");
    expect(records[0].operationType).toBe("unknown_contract_call");
  });

  it("extracts grantId from memo in format 'grant:<id>'", async () => {
    await setHorizonMock([makeTx({ hash: "tx1", memo: "grant:42" })]);
    const { records } = await getTransactionHistory("GABC");
    expect(records[0].grantId).toBe("42");
  });

  it("handles memo with no grant pattern gracefully", async () => {
    await setHorizonMock([makeTx({ hash: "tx1", memo: "just a memo" })]);
    const { records } = await getTransactionHistory("GABC");
    expect(records[0].grantId).toBeUndefined();
    expect(records[0].memo).toBe("just a memo");
  });

  it("exposes sourceAccount and feeCharged", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", source_account: "GOWNER", fee_charged: "500" }),
    ]);
    const { records } = await getTransactionHistory("GABC");
    expect(records[0].sourceAccount).toBe("GOWNER");
    expect(records[0].feeCharged).toBe("500");
  });

  it("includes createdAt timestamp as ISO string", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", created_at: "2026-04-01T12:00:00Z" }),
    ]);
    const { records } = await getTransactionHistory("GABC");
    expect(records[0].createdAt).toBe("2026-04-01T12:00:00Z");
  });
});

// ── getGrantHistory ───────────────────────────────────────────────────────────

describe("getGrantHistory", () => {
  beforeEach(async () => {
    // Set a contract ID for these tests via env
    process.env.NEXT_PUBLIC_CONTRACT_ID = "CONTRACT_ABC";
  });

  it("returns only records matching the grant ID by memo", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", memo: "grant:7" }),
      makeTx({ hash: "tx2", memo: "grant:8" }),
      makeTx({ hash: "tx3", memo: "grant:7" }),
      makeTx({ hash: "tx4" }),
    ]);

    const { records } = await getGrantHistory(7);
    expect(records).toHaveLength(2);
    expect(records.every((r) => r.grantId === "7")).toBe(true);
  });

  it("accepts bigint grantId", async () => {
    await setHorizonMock([makeTx({ hash: "tx1", memo: "grant:99" })]);
    const { records } = await getGrantHistory(99n);
    expect(records).toHaveLength(1);
    expect(records[0].grantId).toBe("99");
  });

  it("returns empty records when no contract ID is configured", async () => {
    const saved = process.env.NEXT_PUBLIC_CONTRACT_ID;
    process.env.NEXT_PUBLIC_CONTRACT_ID = "";

    // Re-import to pick up cleared env (module reads env at call time)
    const { getGrantHistory: fn } = await import("../lib/stellar/history");
    const { records } = await fn(1);
    expect(records).toHaveLength(0);

    process.env.NEXT_PUBLIC_CONTRACT_ID = saved;
  });

  it("returns empty records when no txs match the grant ID", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", memo: "grant:1" }),
      makeTx({ hash: "tx2", memo: "grant:2" }),
    ]);
    const { records } = await getGrantHistory(99);
    expect(records).toHaveLength(0);
  });

  it("respects limit and order options", async () => {
    const mock = await setHorizonMock([]);
    await getGrantHistory(1, { limit: 15, order: "asc" });
    expect(mock._builder.limit).toHaveBeenCalledWith(15);
    expect(mock._builder.order).toHaveBeenCalledWith("asc");
  });

  it("passes cursor when provided", async () => {
    const mock = await setHorizonMock([]);
    await getGrantHistory(1, { cursor: "page-5" });
    expect(mock._builder.cursor).toHaveBeenCalledWith("page-5");
  });

  it("includes nextCursor only when matching records exist", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", memo: "grant:3", paging_token: "page-1" }),
    ]);
    const { nextCursor } = await getGrantHistory(3);
    expect(nextCursor).toBe("page-1");
  });

  it("returns undefined nextCursor when no matching records", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx1", memo: "grant:1", paging_token: "page-1" }),
    ]);
    const { nextCursor } = await getGrantHistory(99);
    expect(nextCursor).toBeUndefined();
  });

  it("maps successful field correctly", async () => {
    await setHorizonMock([
      makeTx({ hash: "tx-ok", memo: "grant:4", successful: true }),
      makeTx({ hash: "tx-fail", memo: "grant:4", successful: false }),
    ]);
    const { records } = await getGrantHistory(4);
    const ok = records.find((r) => r.txHash === "tx-ok");
    const fail = records.find((r) => r.txHash === "tx-fail");
    expect(ok?.successful).toBe(true);
    expect(fail?.successful).toBe(false);
  });
});
