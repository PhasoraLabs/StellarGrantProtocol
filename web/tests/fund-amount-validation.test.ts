import { describe, it, expect } from "vitest";
import { validateFundAmount } from "@/hooks/useFundGrant";

const balance = { xlm: 100n, tokens: { USDC: 50n } };

describe("validateFundAmount", () => {
  it("rejects zero and negative amounts", () => {
    expect(validateFundAmount(0n, "native", balance)).toMatch(/greater than zero/);
    expect(validateFundAmount(-1n, "native", balance)).toMatch(/greater than zero/);
  });
  it("rejects amounts above the balance", () => {
    expect(validateFundAmount(101n, "native", balance)).toMatch(/exceeds/);
    expect(validateFundAmount(51n, "USDC", balance)).toMatch(/exceeds/);
  });
  it("accepts valid amounts", () => {
    expect(validateFundAmount(100n, "native", balance)).toBeNull();
    expect(validateFundAmount(1n, "USDC", null)).toBeNull();
  });
});
