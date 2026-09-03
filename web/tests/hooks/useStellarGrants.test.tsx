/**
 * Issue #494 — React hooks for StellarGrants SDK
 *
 * Verifies that StellarGrantsProvider exposes the sdk, client, logger, and
 * batch via context, and that useStellarGrants throws outside the provider.
 */

import { describe, it, expect } from "vitest";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { StellarGrantsProvider } from "@/components/StellarGrantsProvider";
import { useStellarGrants } from "@/hooks/useStellarGrants";
import { StellarGrantsSDK } from "@/lib/stellar/sdk";

function wrapper({ children }: { children: ReactNode }) {
  return <StellarGrantsProvider debug={false}>{children}</StellarGrantsProvider>;
}

describe("useStellarGrants (#494)", () => {
  it("throws when used outside StellarGrantsProvider", () => {
    expect(() => renderHook(() => useStellarGrants())).toThrow(
      /StellarGrantsProvider/
    );
  });

  it("returns client, logger, batch, and sdk from provider", () => {
    const { result } = renderHook(() => useStellarGrants(), { wrapper });
    expect(result.current.client).toBeDefined();
    expect(result.current.logger).toBeDefined();
    expect(result.current.batch).toBeDefined();
    expect(result.current.sdk).toBeDefined();
  });

  it("sdk is an instance of StellarGrantsSDK", () => {
    const { result } = renderHook(() => useStellarGrants(), { wrapper });
    expect(result.current.sdk).toBeInstanceOf(StellarGrantsSDK);
  });

  it("config reflects the debug/logLevel props", () => {
    function debugWrapper({ children }: { children: ReactNode }) {
      return <StellarGrantsProvider debug={true}>{children}</StellarGrantsProvider>;
    }
    const { result } = renderHook(() => useStellarGrants(), { wrapper: debugWrapper });
    expect(result.current.config.debug).toBe(true);
    expect(result.current.config.logLevel).toBe("debug");
  });

  it("sdk.hydrate + sdk.queryGrants works end-to-end", () => {
    const { result } = renderHook(() => useStellarGrants(), { wrapper });
    const { sdk } = result.current;

    sdk.hydrate([
      { id: "1", title: "Open Source Tooling", description: "A grant", budget: 100n, funded: 0n, deadline: 0n, created_at: 0n, status: "open", owner: "G1", token: "native", reviewers: [], milestones: [], num_milestones: 0, quorum: 1, tags: [], is_public_good: false },
      { id: "2", title: "DeFi Infra", description: "Another grant", budget: 200n, funded: 0n, deadline: 0n, created_at: 0n, status: "open", owner: "G2", token: "native", reviewers: [], milestones: [], num_milestones: 0, quorum: 1, tags: [], is_public_good: false },
    ] as never[]);

    const found = sdk.grantSearchByTitle("open source");
    expect(found).toHaveLength(1);
    expect(found[0].title).toBe("Open Source Tooling");
  });
});
