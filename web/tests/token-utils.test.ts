/**
 * Token Utilities Tests
 *
 * Tests for formatTokenAmount, parseTokenAmount, and other token utility functions.
 */

import { describe, it, expect } from "vitest";
import {
  formatTokenAmount,
  parseTokenAmount,
  convertTokenDecimals,
  convertTokenDecimalsWithRemainder,
  minTokenAmount,
  isZeroAmount,
  addTokenAmounts,
  subTokenAmounts,
  formatTokenAddress,
  isNativeToken,
  getDefaultDecimals,
} from "@/lib/tokens/utils";

describe("formatTokenAmount", () => {
  describe("basic formatting", () => {
    it("should format XLM amount (7 decimals) correctly", () => {
      // 1 XLM = 10000000 stroops
      expect(formatTokenAmount(10000000n, 7)).toBe("1");
      expect(formatTokenAmount(100000000n, 7)).toBe("10");
      expect(formatTokenAmount(150000000n, 7)).toBe("15");
    });

    it("should format USDC amount (6 decimals) correctly", () => {
      // 1 USDC = 1000000 units
      expect(formatTokenAmount(1000000n, 6)).toBe("1");
      expect(formatTokenAmount(5000000n, 6)).toBe("5");
      expect(formatTokenAmount(1500000n, 6)).toBe("1.5");
    });

    it("should format fractional amounts correctly", () => {
      expect(formatTokenAmount(150000000n, 7)).toBe("15");
      expect(formatTokenAmount(155000000n, 7)).toBe("15.5");
      expect(formatTokenAmount(155500000n, 7)).toBe("15.55");
    });

    it("should handle zero amount", () => {
      expect(formatTokenAmount(0n, 7)).toBe("0");
    });
  });

  describe("with symbol", () => {
    it("should append symbol when showSymbol is true", () => {
      expect(formatTokenAmount(10000000n, 7, { symbol: "XLM", showSymbol: true })).toBe("1 XLM");
      expect(formatTokenAmount(1000000n, 6, { symbol: "USDC", showSymbol: true })).toBe("1 USDC");
    });

    it("should not show symbol when showSymbol is false", () => {
      expect(formatTokenAmount(10000000n, 7, { symbol: "XLM", showSymbol: false })).toBe("1");
    });

    it("should not show symbol when not specified", () => {
      expect(formatTokenAmount(10000000n, 7)).toBe("1");
    });
  });

  describe("different input types", () => {
    it("should handle bigint input", () => {
      expect(formatTokenAmount(10000000n, 7)).toBe("1");
    });

    it("should handle number input", () => {
      expect(formatTokenAmount(10000000, 7)).toBe("1");
    });

    it("should reject number input above MAX_SAFE_INTEGER", () => {
      expect(formatTokenAmount(Number.MAX_SAFE_INTEGER + 2, 7)).toBe("Invalid");
      expect(formatTokenAmount(Infinity, 7, { symbol: "XLM", showSymbol: true })).toBe("Invalid XLM");
    });

    it("should format amounts above MAX_SAFE_INTEGER exactly as bigint/string", () => {
      expect(formatTokenAmount(90071992547409930000000n, 7)).toBe("9,007,199,254,740,993");
      expect(formatTokenAmount("90071992547409930000000", 7)).toBe("9,007,199,254,740,993");
    });

    it("should handle string input", () => {
      expect(formatTokenAmount("10000000", 7)).toBe("1");
    });

    it("should handle invalid string gracefully", () => {
      expect(formatTokenAmount("invalid", 7)).toBe("Invalid");
    });
  });

  describe("negative amounts", () => {
    it("should handle negative bigint", () => {
      expect(formatTokenAmount(-10000000n, 7)).toBe("-1");
    });

    it("should handle negative with symbol", () => {
      expect(formatTokenAmount(-10000000n, 7, { symbol: "XLM", showSymbol: true })).toBe("-1 XLM");
    });
  });

  describe("precision control", () => {
    it("should respect precision option", () => {
      expect(formatTokenAmount(155500000n, 7, { precision: 1 })).toBe("15.5");
      expect(formatTokenAmount(155500000n, 7, { precision: 2 })).toBe("15.55");
      expect(formatTokenAmount(155500000n, 7, { precision: 0 })).toBe("15");
    });

    it("should trim trailing zeros by default", () => {
      expect(formatTokenAmount(150000000n, 7)).toBe("15");
      expect(formatTokenAmount(150100000n, 7)).toBe("15.01");
    });
  });

  describe("large numbers", () => {
    it("should handle large amounts without precision loss", () => {
      const largeAmount = BigInt("10000000000000000"); // 1 billion XLM
      expect(formatTokenAmount(largeAmount, 7)).toBe("1,000,000,000");
    });

    it("should add locale separators for large numbers", () => {
      expect(formatTokenAmount(1000000000000n, 7)).toContain(",");
    });
  });

  describe("very small amounts", () => {
    it("should handle sub-unit amounts correctly", () => {
      expect(formatTokenAmount(1n, 7)).toBe("0.0000001");
      expect(formatTokenAmount(100n, 7)).toBe("0.00001");
    });
  });
});

describe("parseTokenAmount", () => {
  it("should parse display amount to raw units (XLM)", () => {
    expect(parseTokenAmount("1", 7)).toBe(10000000n);
    expect(parseTokenAmount("10", 7)).toBe(100000000n);
  });

  it("should parse fractional amounts correctly", () => {
    expect(parseTokenAmount("1.5", 7)).toBe(15000000n);
    expect(parseTokenAmount("0.5", 7)).toBe(5000000n);
  });

  it("should handle USDC decimals", () => {
    expect(parseTokenAmount("1", 6)).toBe(1000000n);
    expect(parseTokenAmount("1.5", 6)).toBe(1500000n);
  });

  it("should strip symbols and whitespace", () => {
    expect(parseTokenAmount("1 XLM", 7)).toBe(10000000n);
    expect(parseTokenAmount("  1.5 USDC  ", 6)).toBe(1500000n);
  });

  it("should handle empty string", () => {
    expect(parseTokenAmount("", 7)).toBe(0n);
  });
});

describe("convertTokenDecimals", () => {
  it("should convert from 7 to 6 decimals", () => {
    // 1 XLM (7 decimals) to 6 decimals representation
    expect(convertTokenDecimals(10000000n, 7, 6)).toBe(1000000n);
  });

  it("should convert from 6 to 7 decimals", () => {
    // 1 USDC (6 decimals) to 7 decimals representation
    expect(convertTokenDecimals(1000000n, 6, 7)).toBe(10000000n);
  });

  it("should return same value for same decimals", () => {
    expect(convertTokenDecimals(10000000n, 7, 7)).toBe(10000000n);
  });

  it("should handle large decimal differences", () => {
    expect(convertTokenDecimals(BigInt("1000000000000000000"), 18, 7)).toBe(10000000n);
  });

  it("defaults to truncation toward zero (documented, historical behaviour)", () => {
    // 15 dropped to 6 decimals → 1.5 → truncated, not rounded
    expect(convertTokenDecimals(15n, 7, 6)).toBe(1n);
    expect(convertTokenDecimals(-15n, 7, 6)).toBe(-1n);
  });

  it("supports explicit rounding modes when reducing precision", () => {
    expect(convertTokenDecimals(15n, 7, 6, "trunc")).toBe(1n);
    expect(convertTokenDecimals(15n, 7, 6, "floor")).toBe(1n);
    expect(convertTokenDecimals(15n, 7, 6, "ceil")).toBe(2n);
    expect(convertTokenDecimals(15n, 7, 6, "round")).toBe(2n);
    expect(convertTokenDecimals(14n, 7, 6, "round")).toBe(1n);
  });

  it("rounds negative amounts consistently per mode", () => {
    // -1.5 → trunc: -1, floor: -2, ceil: -1, round (half away from zero): -2
    expect(convertTokenDecimals(-15n, 7, 6, "trunc")).toBe(-1n);
    expect(convertTokenDecimals(-15n, 7, 6, "floor")).toBe(-2n);
    expect(convertTokenDecimals(-15n, 7, 6, "ceil")).toBe(-1n);
    expect(convertTokenDecimals(-15n, 7, 6, "round")).toBe(-2n);
    expect(convertTokenDecimals(-14n, 7, 6, "round")).toBe(-1n);
  });

  it("is exact when no digits are dropped, whatever the mode", () => {
    for (const mode of ["trunc", "floor", "ceil", "round"] as const) {
      expect(convertTokenDecimals(100n, 7, 6, mode)).toBe(10n);
      expect(convertTokenDecimals(15n, 6, 7, mode)).toBe(150n);
    }
  });
});

describe("convertTokenDecimalsWithRemainder", () => {
  it("surfaces the remainder truncation would discard", () => {
    expect(convertTokenDecimalsWithRemainder(15n, 7, 6)).toEqual({
      value: 1n,
      remainder: 5n,
    });
  });

  it("keeps the remainder independent of the rounding mode", () => {
    expect(convertTokenDecimalsWithRemainder(15n, 7, 6, "round")).toEqual({
      value: 2n,
      remainder: 5n,
    });
  });

  it("reports a zero remainder when precision is not reduced", () => {
    expect(convertTokenDecimalsWithRemainder(15n, 6, 7)).toEqual({
      value: 150n,
      remainder: 0n,
    });
    expect(convertTokenDecimalsWithRemainder(15n, 7, 7)).toEqual({
      value: 15n,
      remainder: 0n,
    });
  });

  it("signs the remainder like the source amount", () => {
    expect(convertTokenDecimalsWithRemainder(-15n, 7, 6)).toEqual({
      value: -1n,
      remainder: -5n,
    });
  });
});

describe("minTokenAmount", () => {
  it("should return smaller amount", () => {
    expect(minTokenAmount(100n, 200n)).toBe(100n);
    expect(minTokenAmount(200n, 100n)).toBe(100n);
  });

  it("should handle equal amounts", () => {
    expect(minTokenAmount(100n, 100n)).toBe(100n);
  });
});

describe("isZeroAmount", () => {
  it("should return true for zero", () => {
    expect(isZeroAmount(0n)).toBe(true);
  });

  it("should return false for non-zero", () => {
    expect(isZeroAmount(1n)).toBe(false);
    expect(isZeroAmount(-1n)).toBe(false);
  });
});

describe("addTokenAmounts and subTokenAmounts", () => {
  it("should add two amounts", () => {
    expect(addTokenAmounts(100n, 200n)).toBe(300n);
  });

  it("should subtract two amounts", () => {
    expect(subTokenAmounts(300n, 100n)).toBe(200n);
  });
});

describe("formatTokenAddress", () => {
  it("should shorten long addresses", () => {
    const address = "GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
    expect(formatTokenAddress(address)).toBe("GABCDE...567890");
  });

  it("should not shorten short addresses", () => {
    expect(formatTokenAddress("GABC123")).toBe("GABC123");
  });

  it("should return native as-is", () => {
    expect(formatTokenAddress("native")).toBe("native");
  });

  it("should handle custom char length", () => {
    const address = "GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
    expect(formatTokenAddress(address, 4)).toBe("GABC...7890");
  });
});

describe("isNativeToken", () => {
  it("should return true for native", () => {
    expect(isNativeToken("native")).toBe(true);
    expect(isNativeToken("NATIVE")).toBe(true);
    expect(isNativeToken("Native")).toBe(true);
  });

  it("should return false for other tokens", () => {
    expect(isNativeToken("USDC")).toBe(false);
    expect(isNativeToken("GABC123...")).toBe(false);
  });
});

describe("getDefaultDecimals", () => {
  it("should return correct decimals for known tokens", () => {
    expect(getDefaultDecimals("XLM")).toBe(7);
    expect(getDefaultDecimals("USDC")).toBe(6);
    expect(getDefaultDecimals("USDT")).toBe(6);
    expect(getDefaultDecimals("BTC")).toBe(8);
    expect(getDefaultDecimals("ETH")).toBe(18);
  });

  it("should return default for unknown tokens", () => {
    expect(getDefaultDecimals("UNKNOWN")).toBe(7);
  });

  it("should be case-insensitive", () => {
    expect(getDefaultDecimals("xlm")).toBe(7);
    expect(getDefaultDecimals("usdc")).toBe(6);
  });
});
