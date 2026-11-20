/**
 * Issue #1286 — TokenAmountInput used a hardcoded, token-agnostic USD price.
 *
 * The old implementation multiplied every token's raw amount by a frozen
 * XLM_USD_PRICE (0.11), so a 10 USDC entry displayed as ≈ $1.10. The USD
 * estimate must now come from the token's actual price — a caller-supplied
 * rate or a live feed for XLM, the 1:1 peg for stablecoins — and must be
 * omitted entirely when no price is known.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import type { ComponentProps } from "react";

import {
  TokenAmountInput,
  clearUsdPriceCache,
} from "@/components/ui/TokenAmountInput";

type Props = ComponentProps<typeof TokenAmountInput>;

function renderInput(props: Partial<Props>) {
  render(
    <TokenAmountInput
      value="10"
      onChange={() => {}}
      token="native"
      {...props}
    />,
  );
}

beforeEach(() => {
  clearUsdPriceCache();
  vi.unstubAllGlobals();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("TokenAmountInput USD estimate (#1286)", () => {
  it("omits the estimate for a non-native token with no known price", () => {
    renderInput({ token: "CBADTOKENADDRESSNOTAPRICEFEED" });

    expect(screen.queryByText(/USD/)).toBeNull();
  });

  it("never applies the XLM constant to a non-native token", () => {
    renderInput({ token: "WEIRDTOKEN", value: "10" });

    // 10 × 0.11 would render as ≈ $1.10
    expect(screen.queryByText(/\$1\.10 USD/)).toBeNull();
  });

  it("shows a 1:1 estimate for the USD stablecoin USDC", () => {
    renderInput({ token: "USDC", value: "10" });

    expect(screen.getByText(/\$10\.00 USD/)).toBeInTheDocument();
  });

  it("uses a caller-supplied live price instead of a hardcoded constant", () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);

    renderInput({ token: "native", value: "10", usdPrice: 0.2 });

    expect(screen.getByText(/\$2\.00 USD/)).toBeInTheDocument();
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("fetches the live XLM price when no rate is supplied", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        new Response(JSON.stringify({ stellar: { usd: 0.45 } }), { status: 200 }),
      ),
    );

    renderInput({ token: "native", value: "10" });

    await waitFor(() => {
      expect(screen.getByText(/\$4\.50 USD/)).toBeInTheDocument();
    });
    expect(fetch).toHaveBeenCalledWith(expect.stringContaining("coingecko"));
  });

  it("omits the estimate when the price feed is unavailable", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("rate limited", { status: 429 })),
    );

    renderInput({ token: "native", value: "10" });

    await waitFor(() => expect(fetch).toHaveBeenCalled());
    expect(screen.queryByText(/USD/)).toBeNull();
  });
});
