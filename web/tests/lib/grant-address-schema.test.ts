/**
 * Issue #1285 — client-side Stellar address validation skipped the StrKey
 * checksum.
 *
 * The grant schemas validated recipient and reviewer addresses with a
 * format-only `/^G[A-Z2-7]{55}$/` regex, so a mistyped address with a broken
 * CRC16 checksum passed the form and only failed later at transaction
 * build/sign time — if a reviewer slot ever signed at all.
 */

import { describe, it, expect } from "vitest";
import { StrKey } from "@stellar/stellar-sdk";

import { step1Schema, step3Schema } from "@/lib/schemas/grant";

const VALID_ADDRESS = "GCK4LZF4RXGPTVCQFSAZ5RYS7CY26BSSML5TJLSBVWUNFJDTNGJ2LOWH";
const SECOND_ADDRESS = "GCZLRS6VV4I7XJGM4WBGSFBSZRH2VFKPTW43ONK4PLWPYDLCE7AYTZPU";
const THIRD_ADDRESS = "GBE5F52IVCZSTRFUWYR5IYGYPSNDMSUY77CEFIZAS5HJ24WVGMWUOTMT";

/** Right length and alphabet, wrong CRC16 checksum. */
const INVALID_CHECKSUM_ADDRESS = `${VALID_ADDRESS.slice(0, -1)}A`;

const validStep1 = {
  title: "A sufficiently long grant title",
  description:
    "A sufficiently detailed grant description that comfortably clears the fifty character minimum.",
  totalBudget: 1000,
  budgetToken: "native" as const,
  deadline: "2027-12-31T23:59",
};

describe("grant schema address validation (#1285)", () => {
  it("crafts an address the old format-only regex would have accepted", () => {
    expect(INVALID_CHECKSUM_ADDRESS).toMatch(/^G[A-Z2-7]{55}$/);
    expect(StrKey.isValidEd25519PublicKey(INVALID_CHECKSUM_ADDRESS)).toBe(false);
    expect(StrKey.isValidEd25519PublicKey(VALID_ADDRESS)).toBe(true);
  });

  it("rejects a recipient address with an invalid checksum", () => {
    const result = step1Schema.safeParse({
      ...validStep1,
      recipientAddress: INVALID_CHECKSUM_ADDRESS,
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      const issue = result.error.issues.find(
        (i) => i.path.join(".") === "recipientAddress",
      );
      expect(issue?.message).toBe("Invalid Stellar address");
    }
  });

  it("accepts a recipient address with a valid checksum", () => {
    const result = step1Schema.safeParse({
      ...validStep1,
      recipientAddress: VALID_ADDRESS,
    });

    expect(result.success).toBe(true);
  });

  it("rejects a reviewer address with an invalid checksum", () => {
    const result = step3Schema.safeParse({
      reviewers: [VALID_ADDRESS, SECOND_ADDRESS, INVALID_CHECKSUM_ADDRESS],
      quorum: 2,
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues.some((i) => i.message === "Invalid Stellar address")).toBe(true);
    }
  });

  it("accepts reviewers whose checksums verify", () => {
    const result = step3Schema.safeParse({
      reviewers: [VALID_ADDRESS, SECOND_ADDRESS, THIRD_ADDRESS],
      quorum: 2,
    });

    expect(result.success).toBe(true);
  });
});
