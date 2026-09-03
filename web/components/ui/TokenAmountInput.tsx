"use client";

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
}

const XLM_USD_PRICE = 0.11;

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
}: TokenAmountInputProps) {
  const decimals = getDecimals(token);
  const symbol = getTokenSymbol(token);
  const showTokenSelector = availableTokens.length > 1;

  const usdValue = value
    ? (parseFloat(value) * XLM_USD_PRICE).toFixed(2)
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
          {value && !error && (
            <p className="font-mono text-xs text-text-muted">
              ≈ ${usdValue} USD
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
