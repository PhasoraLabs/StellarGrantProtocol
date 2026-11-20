jest.mock("@stellar/stellar-sdk", () => {
  return {
    StrKey: {
      decodeEd25519PublicKey: (key: string) => {
        if (key === "GBAD") throw new Error("bad key");
        const out = new Uint8Array(32);
        // encode last 4 bytes as 0x01 0x02 0x03 0x04 so hint = 01020304
        out[28] = 0x01;
        out[29] = 0x02;
        out[30] = 0x03;
        out[31] = 0x04;
        return out;
      },
    },
    xdr: {
      DecoratedSignature: {
        fromXDR: (b64: string) => ({
          hint: () => {
            if (b64 === "SIG_MATCH") return new Uint8Array([0x01, 0x02, 0x03, 0x04]);
            return new Uint8Array([0x09, 0x09, 0x09, 0x09]);
          },
        }),
      },
    },
    TransactionBuilder: {
      fromXDR: (_xdr: string) => ({ signatures: [
        { hint: () => new Uint8Array([0x01, 0x02, 0x03, 0x04]) },
      ], toXDR: () => "TX" }),
    },
  };
});

import {
  computeSignatureWeight,
  meetsThreshold,
} from "../src/utils/transactions";

describe("transactions utils", () => {
  it("computeSignatureWeight sums signer weights whose hint matches", () => {
    const weight = computeSignatureWeight(
      "TX_XDR",
      "NETWORK",
      [
        { key: "GSIGNER", weight: 2 },
        { key: "GBAD", weight: 10 },
      ],
    );

    expect(weight).toBe(2);
  });

  it("meetsThreshold compares against requested level", () => {
    const thresholds = { low_threshold: 1, med_threshold: 3, high_threshold: 5 };

    expect(meetsThreshold(2, thresholds, "low")).toBe(true);
    expect(meetsThreshold(2, thresholds, "medium")).toBe(false);
    expect(meetsThreshold(5, thresholds, "high")).toBe(true);
  });
});
