/**
 * Token Utilities
 *
 * Helper functions for token formatting, parsing, and calculations.
 */

import type { TokenMetadata as _TokenMetadata } from "@/types";

/**
 * Format a token amount for display
 * @param amount - The raw amount (in smallest units, e.g., stroops)
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
    // number - convert carefully to avoid precision loss
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
 * Convert amount between different decimal precision
 * @param amount - Raw amount
 * @param fromDecimals - Source token decimals
 * @param toDecimals - Target token decimals
 * @returns Converted amount as bigint
 */
export function convertTokenDecimals(
  amount: bigint,
  fromDecimals: number,
  toDecimals: number
): bigint {
  if (fromDecimals === toDecimals) {
    return amount;
  }

  const diff = toDecimals - fromDecimals;
  if (diff > 0) {
    return amount * (10n ** BigInt(diff));
  } else {
    return amount / (10n ** BigInt(Math.abs(diff)));
  }
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
