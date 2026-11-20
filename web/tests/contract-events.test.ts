/**
 * Contract Events Tests — #1280
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { nativeToScVal, xdr } from "@stellar/stellar-sdk";

vi.mock("../lib/stellar/client", () => ({ getRpcClient: vi.fn() }));
vi.mock("@/lib/constants", () => ({ CONTRACT_ID: "CTEST" }));

import { getRpcClient } from "../lib/stellar/client";
import { fetchContractEvents } from "../lib/stellar/events";

const rawEvent = (name: string, grantId: number, ledger: number) => ({
  topic: [nativeToScVal(name, { type: "symbol" })],
  value: nativeToScVal({ grant_id: BigInt(grantId), amount: BigInt(500) }, {
    type: { grant_id: ["symbol", "u64"], amount: ["symbol", "i128"] },
  }),
  ledger,
  ledgerClosedAt: "2026-01-01T00:00:00Z",
});

describe("fetchContractEvents", () => {
  const getEvents = vi.fn();
  const getLatestLedger = vi.fn();

  beforeEach(() => {
    getEvents.mockReset();
    getLatestLedger.mockReset().mockResolvedValue({ sequence: 10_000 });
    vi.mocked(getRpcClient).mockReturnValue({ getEvents, getLatestLedger } as unknown as ReturnType<typeof getRpcClient>);
  });

  it("decodes on-chain events and keeps only the requested grant", async () => {
    getEvents.mockResolvedValue({
      events: [rawEvent("grant_funded", 42, 9_990), rawEvent("grant_funded", 7, 9_991)],
      cursor: "c1",
    });
    const result = await fetchContractEvents("42");
    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("GrantFunded");
    expect(result[0].ledger).toBe(9_990);
    expect(result[0].data).toEqual({ grant_id: "42", amount: "500" });
    expect(getEvents.mock.calls[0][0]).toMatchObject({ startLedger: 9_280 });
  });

  it("follows the cursor across pages without resending startLedger", async () => {
    const fullPage = Array.from({ length: 200 }, (_, i) => rawEvent("milestone_submitted", 1, 9_000 + i));
    getEvents
      .mockResolvedValueOnce({ events: fullPage, cursor: "c1" })
      .mockResolvedValueOnce({ events: [rawEvent("grant_funded", 1, 9_500)], cursor: "c2" });
    const result = await fetchContractEvents("1", { startLedger: 9_000 });
    expect(result).toHaveLength(201);
    expect(getEvents).toHaveBeenCalledTimes(2);
    expect(getEvents.mock.calls[1][0]).toMatchObject({ cursor: "c1" });
    expect(getEvents.mock.calls[1][0]).not.toHaveProperty("startLedger");
  });

  it("skips events it cannot decode", async () => {
    getEvents.mockResolvedValue({
      events: [{ topic: [], value: xdr.ScVal.scvVoid(), ledger: 1, ledgerClosedAt: "2026-01-01T00:00:00Z" }],
      cursor: "c",
    });
    expect(await fetchContractEvents("1")).toEqual([]);
  });
});
