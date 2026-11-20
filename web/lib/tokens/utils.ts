/**
 * Token Utilities
 *
 * Helper functions for token formatting, parsing, and calculations.
 */

import type { TokenMetadata as _TokenMetadata } from "@/types";

/**
 * Format a token amount for display
 * @param amount - The raw amount (in smallest units, e.g., stroops). Pass a
 *   bigint or numeric string for values that may exceed Number.MAX_SAFE_INTEGER;
 *   unsafe `number` inputs are rejected as "Invalid" rather than silently
 *   formatted with lost precision.
 * @param decimals - Number of decimals for the token
 * @param options - Formatting options
 * @returns Formatted amount string
 */
export function formatTokenAmount(
  amount: bigint | number | string,
  decimals: number,
  options?: {
    symbol?: string;
    locale?: string;
    showSymbol?: boolean;
    precision?: number;
  }
): string {
  const {
    symbol,
    locale = "en-US",
    showSymbol = false,
    precision,
  } = options || {};

  // Convert to bigint if string or number
  let amountBigInt: bigint;
  if (typeof amount === "bigint") {
    amountBigInt = amount;
  } else if (typeof amount === "string") {
    try {
      amountBigInt = BigInt(amount);
    } catch {
      return showSymbol && symbol ? `Invalid ${symbol}` : "Invalid";
    }
  } else {
    // A number above MAX_SAFE_INTEGER has already lost precision before it
    // reaches us, so rounding cannot recover it. Callers must pass a bigint
    // or numeric string for amounts that may exceed the safe range.
    if (!Number.isFinite(amount) || Math.abs(amount) > Number.MAX_SAFE_INTEGER) {
      return showSymbol && symbol ? `Invalid ${symbol}` : "Invalid";
    }
    amountBigInt = BigInt(Math.round(amount));
  }

  // Handle negative amounts
  const isNegative = amountBigInt < BigInt(0);
  if (isNegative) {
    amountBigInt = -amountBigInt;
  }

  const divisor = BigInt(10 ** decimals);
  const whole = amountBigInt / divisor;
  const fractional = amountBigInt % divisor;

  // Format fractional part with proper padding
  let fractionalStr = fractional.toString().padStart(decimals, "0");

  // Remove trailing zeros if no specific precision requested
  if (precision === undefined) {
    fractionalStr = fractionalStr.replace(/0+$/, "");
    // Keep at least one decimal if there's a fractional part
    if (fractionalStr.length > 0 && fractionalStr.length < decimals) {
      // Don't strip all decimals for very small amounts
      const nonZeroIndex = fractionalStr.search(/[1-9]/);
      if (nonZeroIndex !== -1 && nonZeroIndex > 2) {
        fractionalStr = fractionalStr.slice(0, nonZeroIndex + 1);
      }
    }
  } else {
    fractionalStr = fractionalStr.slice(0, precision);
  }

  // Format whole number with locale separators
  const wholeStr = whole.toLocaleString(locale);

  let result = fractionalStr.length > 0 ? `${wholeStr}.${fractionalStr}` : wholeStr;
  if (isNegative) {
    result = `-${result}`;
  }

  if (showSymbol && symbol) {
    result = `${result} ${symbol}`;
  }

  return result;
}

/**
 * Parse a display amount back to raw units (stroops)
 * @param amount - Display amount string (e.g., "1.5")
 * @param decimals - Number of decimals for the token
 * @returns Raw amount as bigint
 */
export function parseTokenAmount(amount: string, decimals: number): bigint {
  // Remove any symbol or whitespace
  const cleanAmount = amount.replace(/[A-Za-z\s]/g, "").trim();
  
  if (!cleanAmount) {
    return BigInt(0);
  }

  const [wholePart, fractionalPart = ""] = cleanAmount.split(".");
  
  // Pad or truncate fractional part to match decimals
  const paddedFractional = fractionalPart.padEnd(decimals, "0").slice(0, decimals);
  
  const whole = BigInt(wholePart || "0");
  const fractional = BigInt(paddedFractional);
  
  return whole * BigInt(10 ** decimals) + fractional;
}

/**
 * Rounding modes for precision-*reducing* token conversions.
 *
 * - `"trunc"` — truncate toward zero (default, historical behaviour)
 * - `"floor"` — round toward −Infinity
 * - `"ceil"` — round toward +Infinity
 * - `"round"` — half away from zero
 */
export type TokenRoundingMode = "trunc" | "floor" | "ceil" | "round";

/**
 * Divide `value` by a positive `divisor`, applying `mode` to the digits that
 * plain BigInt division would discard (BigInt division truncates toward zero).
 */
function divideWithRounding(
  value: bigint,
  divisor: bigint,
  mode: TokenRoundingMode
): bigint {
  const quotient = value / divisor;
  const remainder = value % divisor;
  if (remainder === 0n || mode === "trunc") return quotient;

  if (mode === "floor") return value < 0n ? quotient - 1n : quotient;
  if (mode === "ceil") return value > 0n ? quotient + 1n : quotient;

  // "round": half away from zero — ties resolve away from zero in both
  // directions so ±x.5 rounds to ±(x+1).
  const magnitude = remainder < 0n ? -remainder : remainder;
  const half = divisor / 2n + (divisor % 2n); // ceil(divisor / 2)
  if (magnitude < half) return quotient;
  return value < 0n ? quotient - 1n : quotient + 1n;
}

/**
 * Convert amount between different decimal precision.
 *
 * Precision-increasing conversions (`toDecimals > fromDecimals`) multiply by
 * a power of ten and are always exact, regardless of `rounding`.
 *
 * Precision-*reducing* conversions divide by a power of ten, and BigInt
 * division discards the remainder. That discarded remainder is now an
 * explicit, documented decision via `rounding`:
 *
 * - `"trunc"` (**default**) — truncate toward zero: the dropped digits are
 *   silently discarded. This is lossy — repeated conversions (e.g. in a
 *   loop) accumulate value loss — and is retained only as the function's
 *   historical behaviour. Prefer a non-lossy mode where the result is shown
 *   to, or settled with, a user.
 * - `"floor"` — round toward −Infinity (a positive amount never grows).
 * - `"ceil"` — round toward +Infinity (nothing owed to a recipient is dusted
 *   away).
 * - `"round"` — half away from zero (conventional display/settlement
 *   rounding).
 *
 * Where the discarded value matters financially, use
 * {@link convertTokenDecimalsWithRemainder} to read the exact remainder.
 *
 * @param amount - Raw amount in `fromDecimals` smallest units
 * @param fromDecimals - Source token decimals
 * @param toDecimals - Target token decimals
 * @param rounding - How dropped digits are handled when precision is reduced
 * @returns Converted amount as bigint
 */
export function convertTokenDecimals(
  amount: bigint,
  fromDecimals: number,
  toDecimals: number,
  rounding: TokenRoundingMode = "trunc"
): bigint {
  if (fromDecimals === toDecimals) {
    return amount;
  }

  const diff = toDecimals - fromDecimals;
  if (diff > 0) {
    return amount * (10n ** BigInt(diff));
  } else {
    return divideWithRounding(amount, 10n ** BigInt(Math.abs(diff)), rounding);
  }
}

/** Result of {@link convertTokenDecimalsWithRemainder}. */
export interface ConvertedTokenAmount {
  /** Converted amount, rounded according to `rounding`. */
  value: bigint;
  /**
   * The portion of `amount` plain truncation discards (`amount % divisor`),
   * signed like `amount`, expressed in the *source* token's smallest units.
   * `0n` whenever no precision was lost.
   */
  remainder: bigint;
}

/**
 * Convert between decimal precisions and surface the value that truncation
 * would otherwise discard, so callers can log, warn on, or account for the
 * difference when it matters financially.
 *
 * `remainder` is always computed against truncation, independent of the
 * `rounding` mode applied to `value`.
 */
export function convertTokenDecimalsWithRemainder(
  amount: bigint,
  fromDecimals: number,
  toDecimals: number,
  rounding: TokenRoundingMode = "trunc"
): ConvertedTokenAmount {
  if (fromDecimals === toDecimals || toDecimals > fromDecimals) {
    return {
      value: convertTokenDecimals(amount, fromDecimals, toDecimals, rounding),
      remainder: 0n,
    };
  }

  const divisor = 10n ** BigInt(fromDecimals - toDecimals);
  return {
    value: divideWithRounding(amount, divisor, rounding),
    remainder: amount % divisor,
  };
}

/**
 * Get the smaller of two amounts (useful for min calculations)
 */
export function minTokenAmount(a: bigint, b: bigint): bigint {
  return a < b ? a : b;
}

/**
 * Check if an amount is zero
 */
export function isZeroAmount(amount: bigint): boolean {
  return amount === BigInt(0);
}

/**
 * Safely add two token amounts (must have same decimals)
 */
export function addTokenAmounts(a: bigint, b: bigint): bigint {
  return a + b;
}

/**
 * Safely subtract two token amounts (must have same decimals)
 */
export function subTokenAmounts(a: bigint, b: bigint): bigint {
  return a - b;
}

/**
 * Format token address for display (shortened)
 */
export function formatTokenAddress(address: string, chars: number = 6): string {
  if (!address || address === "native") {
    return address;
  }
  if (address.length <= chars * 2) {
    return address;
  }
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}

/**
 * Check if a token address is the native token (XLM)
 */
export function isNativeToken(tokenAddress: string): boolean {
  return tokenAddress.toLowerCase() === "native";
}

/**
 * Get default decimals for common tokens
 */
export function getDefaultDecimals(tokenSymbol: string): number {
  const defaults: Record<string, number> = {
    XLM: 7,
    USDC: 6,
    USDT: 6,
    BTC: 8,
    ETH: 18,
  };
  return defaults[tokenSymbol.toUpperCase()] || 7;
}
