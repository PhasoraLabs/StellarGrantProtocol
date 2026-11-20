/**
 * ContractClient.approveToken validation tests.
 *
 * The approval build goes through Horizon (account load) and Soroban RPC
 * (latest ledger), so both clients are mocked here to keep the suite
 * hermetic; the guards run before any network access and are exercised as-is.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { Account, xdr } from "@stellar/stellar-sdk";

vi.mock("@/lib/stellar/client", () => ({
  getRpcClient: vi.fn(),
  getHorizonClient: vi.fn(),
  rpcClient: { getLatestLedger: vi.fn() },
  horizonClient: { loadAccount: vi.fn() },
  networkPassphraseConfig: "Test SDF Network ; September 2015",
}));

import { horizonClient, rpcClient } from "@/lib/stellar/client";
import { ContractClient } from "@/lib/stellar/contract";

const client = new ContractClient();

// Valid StrKey addresses: the fake "GOWNER…"/"CCONTRACT…" placeholders never
// made it past Horizon / nativeToScVal anyway.
const OWNER = "GADQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOZPI";
const SPENDER = "CADQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQOBYHA4DQP5KR";

const valid = {
  tokenAddress: "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
  spender: "CCONTRACT00000000000000000000000000000000000000000000000",
  amount: 1_000_000n,
  owner: "GOWNER00000000000000000000000000000000000000000000000000",
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ContractClient.approveToken", () => {
  it("requires a token address", async () => {
    await expect(client.approveToken({ ...valid, tokenAddress: "" })).rejects.toThrow(/tokenAddress/);
  });

  it("requires a spender", async () => {
    await expect(client.approveToken({ ...valid, spender: "" })).rejects.toThrow(/spender/);
  });

  it("requires an owner", async () => {
    await expect(client.approveToken({ ...valid, owner: "" })).rejects.toThrow(/owner/);
  });

  it("rejects a non-positive amount", async () => {
    await expect(client.approveToken({ ...valid, amount: 0n })).rejects.toThrow(/amount/);
  });

  it("builds an unsigned approval XDR for well-formed params", async () => {
    vi.mocked(horizonClient.loadAccount).mockResolvedValue(
      new Account(OWNER, "0") as unknown as Awaited<ReturnType<typeof horizonClient.loadAccount>>,
    );
    vi.mocked(rpcClient.getLatestLedger).mockResolvedValue({
      id: "",
      protocolVersion: 22,
      sequence: 5_000_000,
    } as unknown as Awaited<ReturnType<typeof rpcClient.getLatestLedger>>);

    const txXdr = await client.approveToken({ ...valid, owner: OWNER, spender: SPENDER });

    expect(typeof txXdr).toBe("string");
    expect(() => xdr.TransactionEnvelope.fromXDR(txXdr, "base64")).not.toThrow();
    expect(horizonClient.loadAccount).toHaveBeenCalledWith(OWNER);
    expect(rpcClient.getLatestLedger).toHaveBeenCalled();
  });
});
