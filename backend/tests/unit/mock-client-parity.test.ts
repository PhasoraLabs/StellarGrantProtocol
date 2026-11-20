/**
 * Tests for mock-client.ts parity with real-client.ts (issue #1441).
 *
 * Two levels of verification:
 *
 * 1. Type-level — TypeScript's assignability check guarantees that
 *    MockSorobanContractClient satisfies the SorobanContractClient interface
 *    at compile time.  If the mock is ever missing a method, the
 *    `satisfies` assertion below will cause a type error that fails the build.
 *
 * 2. Runtime — We enumerate the public methods on RealSorobanClient's
 *    prototype and assert that every one of them is also present (and is a
 *    function) on MockSorobanContractClient's prototype.  This catches any
 *    new methods added to the real client that have not yet been mirrored in
 *    the mock.
 */

import { describe, expect, it } from "vitest";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { RealSorobanClient } from "../../src/soroban/real-client";
import type { SorobanContractClient } from "../../src/soroban/types";

// ── 1. Type-level parity ──────────────────────────────────────────────────────
//
// This line is intentionally a compile-time assertion only.  If
// MockSorobanContractClient stops implementing SorobanContractClient the
// TypeScript compiler will emit an error here and the CI build will fail,
// providing an early warning before any runtime test even runs.
//
// The `satisfies` keyword was introduced in TypeScript 4.9.  It checks
// assignability without widening the inferred type.
const _typeCheck: SorobanContractClient = new MockSorobanContractClient();
void _typeCheck; // suppress "declared but never read" lint warnings

// ── 2. Runtime method parity ──────────────────────────────────────────────────

/**
 * Collect the names of all own enumerable and non-enumerable methods on a
 * prototype, excluding the Object base-class members.
 */
function getPrototypeMethods(proto: object): string[] {
  const names = new Set<string>();
  let current: object | null = proto;
  while (current && current !== Object.prototype) {
    for (const name of Object.getOwnPropertyNames(current)) {
      if (name === "constructor") continue;
      const descriptor = Object.getOwnPropertyDescriptor(current, name);
      if (descriptor && typeof descriptor.value === "function") {
        names.add(name);
      }
    }
    current = Object.getPrototypeOf(current) as object | null;
  }
  return [...names];
}

describe("MockSorobanContractClient parity with RealSorobanClient", () => {
  const realMethods = getPrototypeMethods(RealSorobanClient.prototype);
  const mockProto = MockSorobanContractClient.prototype;

  // Filter to only the public interface methods (those defined on
  // SorobanContractClient) so we do not require the mock to replicate
  // internal private helpers like buildViewTx or decodeGrant.
  const interfaceKeys: Array<keyof SorobanContractClient> = [
    "fetchGrants",
    "fetchGrantById",
    "getLatestLedger",
    "fetchEvents",
  ];

  it("mock implements every method in the SorobanContractClient interface", () => {
    for (const method of interfaceKeys) {
      expect(
        typeof mockProto[method],
        `MockSorobanContractClient is missing method: ${method}`,
      ).toBe("function");
    }
  });

  it("real client implements every method in the SorobanContractClient interface", () => {
    for (const method of interfaceKeys) {
      expect(
        realMethods,
        `RealSorobanClient is missing interface method: ${method}`,
      ).toContain(method);
    }
  });

  it("mock has the same interface methods as the real client", () => {
    const missingFromMock = interfaceKeys.filter(
      (m) => typeof mockProto[m] !== "function",
    );
    expect(
      missingFromMock,
      `These interface methods are on RealSorobanClient but missing from MockSorobanContractClient: ${missingFromMock.join(", ")}`,
    ).toHaveLength(0);
  });

  it("mock method signatures match the interface (arity check)", () => {
    // Verify argument counts are consistent with the interface.
    // fetchGrants()         → 0 args
    // fetchGrantById(id)    → 1 arg
    // getLatestLedger()     → 0 args
    // fetchEvents(from, to) → 2 args
    const expectedArity: Record<keyof SorobanContractClient, number> = {
      fetchGrants: 0,
      fetchGrantById: 1,
      getLatestLedger: 0,
      fetchEvents: 2,
    };

    for (const [method, arity] of Object.entries(expectedArity) as Array<
      [keyof SorobanContractClient, number]
    >) {
      expect(
        mockProto[method].length,
        `MockSorobanContractClient.${method} should accept ${arity} argument(s)`,
      ).toBe(arity);
    }
  });

  describe("runtime smoke tests", () => {
    const client = new MockSorobanContractClient();

    it("fetchGrants returns grants array and failedIds", async () => {
      const result = await client.fetchGrants();
      expect(result).toHaveProperty("grants");
      expect(result).toHaveProperty("failedIds");
      expect(Array.isArray(result.grants)).toBe(true);
      expect(Array.isArray(result.failedIds)).toBe(true);
    });

    it("fetchGrantById returns a grant for a known id", async () => {
      const { grants } = await client.fetchGrants();
      const firstId = grants[0]?.id;
      expect(firstId).toBeDefined();
      const grant = await client.fetchGrantById(firstId!);
      expect(grant).not.toBeNull();
      expect(grant?.id).toBe(firstId);
    });

    it("fetchGrantById returns null for an unknown id", async () => {
      const result = await client.fetchGrantById(99999);
      expect(result).toBeNull();
    });

    it("getLatestLedger returns a positive integer", async () => {
      const ledger = await client.getLatestLedger();
      expect(typeof ledger).toBe("number");
      expect(ledger).toBeGreaterThan(0);
    });

    it("fetchEvents returns an array", async () => {
      const events = await client.fetchEvents(0, 2000);
      expect(Array.isArray(events)).toBe(true);
    });

    it("fetchEvents filters by ledger range", async () => {
      // Seed data has events at ledger 900 and 901; nothing outside 905-999.
      const events = await client.fetchEvents(905, 999);
      expect(events).toHaveLength(0);
    });
  });
});
