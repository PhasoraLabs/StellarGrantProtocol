/**
 * ContractClient.approveToken validation tests.
 *
 * The on-chain build is a stub (like the other contract methods), but the
 * input guards are exercised here so callers get clear errors for the USDC
 * approve → deposit flow.
 */

import { describe, it, expect } from "vitest";
import { ContractClient } from "@/lib/stellar/contract";

const client = new ContractClient();

const valid = {
  tokenAddress: "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
  spender: "CCONTRACT00000000000000000000000000000000000000000000000",
  amount: 1_000_000n,
  owner: "GOWNER00000000000000000000000000000000000000000000000000",
};

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

  it("passes validation for well-formed params (build still pending)", async () => {
    // Guards pass; the unimplemented on-chain build throws a distinct error.
    await expect(client.approveToken(valid)).rejects.toThrow(/Not implemented/);
  });
});
