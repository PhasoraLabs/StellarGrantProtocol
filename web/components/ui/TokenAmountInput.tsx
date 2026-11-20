"use client";

import { useEffect, useState } from "react";
import { formatTokenAmount } from "@/lib/tokens";

interface TokenAmountInputProps {
  value: string;
  onChange: (value: string) => void;
  token: string;
  onTokenChange?: (token: string) => void;
  availableTokens?: string[];
  maxAmount?: bigint;
  label?: string;
  error?: string;
  disabled?: boolean;
  placeholder?: string;
  /**
   * Live USD price for one whole token unit, e.g. `0.12` for XLM.
   *
   * Callers that already have a price feed should pass it here. When it is
   * omitted the component resolves the price itself: a live feed for the
   * native token and the 1:1 USD peg for stablecoins. If no price can be
   * resolved the USD estimate is omitted instead of being guessed.
   */
  usdPrice?: number | null;
}

// ── USD price resolution ──────────────────────────────────────────────────────
//
// There is no price feed in the app yet, so XLM's USD price is fetched from a
// public endpoint and cached briefly. Every failure path resolves to `null`,
// which renders as "no estimate" — a missing number, never a wrong one.

const XLM_USD_PRICE_URL =
  "https://api.coingecko.com/api/v3/simple/price?ids=stellar&vs_currencies=usd";
const PRICE_CACHE_TTL_MS = 60_000;

/** Tokens pegged to USD — their price is 1:1 without needing a feed. */
const USD_STABLECOIN_SYMBOLS = new Set(["USDC", "USDT", "USDS", "EURC"]);

let xlmUsdCache: { price: number; fetchedAt: number } | null = null;
let xlmUsdInflight: Promise<number | null> | null = null;

async function fetchXlmUsdPrice(): Promise<number | null> {
  if (xlmUsdCache && Date.now() - xlmUsdCache.fetchedAt < PRICE_CACHE_TTL_MS) {
    return xlmUsdCache.price;
  }
  if (xlmUsdInflight) return xlmUsdInflight;

  xlmUsdInflight = (async () => {
    try {
      const res = await fetch(XLM_USD_PRICE_URL);
      if (!res.ok) return null;
      const json = (await res.json()) as { stellar?: { usd?: unknown } };
      const price = Number(json?.stellar?.usd);
      if (!Number.isFinite(price) || price <= 0) return null;
      xlmUsdCache = { price, fetchedAt: Date.now() };
      return price;
    } catch {
      return null; // offline, blocked or rate-limited → no estimate
    } finally {
      xlmUsdInflight = null;
    }
  })();

  return xlmUsdInflight;
}

/** Drop the cached XLM price (exposed for tests). */
export function clearUsdPriceCache(): void {
  xlmUsdCache = null;
}

const TOKEN_SYMBOLS: Record<string, string> = {
  native: "XLM",
};

function getTokenSymbol(token: string): string {
  return TOKEN_SYMBOLS[token] ?? token;
}

function getDecimals(token: string): number {
  if (token === "native") return 7;
  return 6;
}

export function TokenAmountInput({
  value,
  onChange,
  token,
  onTokenChange,
  availableTokens = ["native"],
  maxAmount,
  label,
  error,
  disabled = false,
  placeholder = "0.00",
  usdPrice,
}: TokenAmountInputProps) {
  const decimals = getDecimals(token);
  const symbol = getTokenSymbol(token);
  const showTokenSelector = availableTokens.length > 1;

  const isNative = token === "native" || symbol === "XLM";
  const isUsdStablecoin = USD_STABLECOIN_SYMBOLS.has(symbol.toUpperCase());
  const [liveXlmPrice, setLiveXlmPrice] = useState<number | null>(null);

  useEffect(() => {
    // A caller-provided price wins; only native XLM needs the live feed.
    if (typeof usdPrice === "number" || !isNative) return;

    let cancelled = false;
    void fetchXlmUsdPrice().then((price) => {
      if (!cancelled) setLiveXlmPrice(price);
    });
    return () => {
      cancelled = true;
    };
  }, [isNative, usdPrice]);

  const usdRate =
    typeof usdPrice === "number" && Number.isFinite(usdPrice) && usdPrice > 0
      ? usdPrice
      : isNative
        ? liveXlmPrice
        : isUsdStablecoin
          ? 1
          : null;

  const amount = Number(value);
  const usdValue =
    usdRate !== null && value !== "" && Number.isFinite(amount) && amount > 0
      ? (amount * usdRate).toFixed(2)
      : null;

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (
      e.key === "e" ||
      e.key === "E" ||
      e.key === "+" ||
      e.key === "-"
    ) {
      e.preventDefault();
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = e.target.value;
    if (raw === "") {
      onChange("");
      return;
    }

    if (!/^\d*\.?\d*$/.test(raw)) return;

    const parts = raw.split(".");
    if (parts.length === 2 && parts[1].length > decimals) {
      return;
    }

    onChange(raw);
  };

  const handleMax = () => {
    if (!maxAmount) return;
    const formatted = formatTokenAmount(maxAmount, decimals, {
      showSymbol: false,
    });
    onChange(formatted);
  };

  return (
    <div className="space-y-1">
      {label && (
        <label className="block font-mono text-[10px] uppercase tracking-wider text-text-muted mb-1">
          {label}
        </label>
      )}
      <div className="relative">
        <div
          className={`flex items-center border bg-surface ${
            error ? "border-danger" : "border-border-color"
          } rounded-none focus-within:border-accent-secondary focus-within:ring-0`}
        >
          <input
            type="text"
            inputMode="decimal"
            value={value}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            disabled={disabled}
            placeholder={placeholder}
            className="flex-1 bg-transparent px-3 py-2 font-mono text-sm text-text-primary outline-none placeholder:text-text-muted/50"
          />
          {maxAmount && (
            <button
              type="button"
              onClick={handleMax}
              disabled={disabled}
              className="px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-accent-secondary hover:text-accent-secondary/80 transition-colors shrink-0"
            >
              Max
            </button>
          )}
          {showTokenSelector ? (
            <select
              value={token}
              onChange={(e) => onTokenChange?.(e.target.value)}
              disabled={disabled}
              className="h-full bg-surface border-l border-border-color px-3 py-2 font-mono text-sm text-text-primary outline-none cursor-pointer hover:bg-bg-secondary transition-colors"
            >
              {availableTokens.map((t) => (
                <option key={t} value={t}>
                  {getTokenSymbol(t)}
                </option>
              ))}
            </select>
          ) : (
            <span className="px-3 py-2 font-mono text-sm text-text-muted border-l border-border-color">
              {symbol}
            </span>
          )}
        </div>
      </div>
      <div className="flex items-center justify-between">
        <div>
          {error && (
            <p className="font-mono text-xs text-danger">{error}</p>
          )}
          {value && !error && usdValue !== null && (
            <p className="font-mono text-xs text-text-muted">
              ≈ ${usdValue} USD
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
