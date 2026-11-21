/**
 * Type-safe ScVal decoders for Soroban contract return values.
 *
 * Each helper narrows an `xdr.ScVal` to a concrete TypeScript type and throws
 * with a descriptive message when the variant does not match expectations, so
 * callers never receive silent `undefined` or `null` for required fields.
 */

import { xdr, Address } from "@stellar/stellar-sdk";

export function decodeU64(val: xdr.ScVal): number {
  if (val.switch() !== xdr.ScValType.scvU64()) {
    throw new Error(`Expected scvU64, got ${val.switch().name}`);
  }
  // U64 fits in a JS number for ledger-scale counters (< 2^53)
  return Number(val.u64().toBigInt());
}

export function decodeU32(val: xdr.ScVal): number {
  if (val.switch() !== xdr.ScValType.scvU32()) {
    throw new Error(`Expected scvU32, got ${val.switch().name}`);
  }
  return val.u32();
}

export function decodeI128AsString(val: xdr.ScVal): string {
  if (val.switch() !== xdr.ScValType.scvI128()) {
    throw new Error(`Expected scvI128, got ${val.switch().name}`);
  }
  const parts = val.i128();
  const hi = BigInt(parts.hi().toBigInt());
  const lo = BigInt(parts.lo().toBigInt());
  return String((hi << 64n) | lo);
}

export function decodeString(val: xdr.ScVal): string {
  const t = val.switch();
  if (t === xdr.ScValType.scvString()) {
    return val.str().toString();
  }
  if (t === xdr.ScValType.scvSymbol()) {
    return val.sym().toString();
  }
  throw new Error(`Expected scvString or scvSymbol, got ${t.name}`);
}

export function decodeAddress(val: xdr.ScVal): string {
  if (val.switch() !== xdr.ScValType.scvAddress()) {
    throw new Error(`Expected scvAddress, got ${val.switch().name}`);
  }
  // #1243 — Address.fromScVal returns a proper strkey (G.../C...),
  // unlike val.address().toString() which returns "[object Object]".
  return Address.fromScVal(val).toString();
}

export function decodeBool(val: xdr.ScVal): boolean {
  if (val.switch() !== xdr.ScValType.scvBool()) {
    throw new Error(`Expected scvBool, got ${val.switch().name}`);
  }
  return val.b();
}

export function decodeVec(val: xdr.ScVal): xdr.ScVal[] {
  if (val.switch() !== xdr.ScValType.scvVec()) {
    throw new Error(`Expected scvVec, got ${val.switch().name}`);
  }
  return val.vec() ?? [];
}

export function decodeMap(val: xdr.ScVal): Map<string, xdr.ScVal> {
  if (val.switch() !== xdr.ScValType.scvMap()) {
    throw new Error(`Expected scvMap, got ${val.switch().name}`);
  }
  const entries = val.map() ?? [];
  const result = new Map<string, xdr.ScVal>();
  for (const entry of entries) {
    result.set(decodeString(entry.key()), entry.val());
  }
  return result;
}

/**
 * Fixtures for constructing valid ScVal instances for each supported Soroban type.
 * Useful for unit tests, RPC mocks, and testing decoders.
 */
export const scValFixtures = {
  u32: (n = 42): xdr.ScVal => xdr.ScVal.scvU32(n),

  u64: (n: number | bigint = 1000): xdr.ScVal =>
    xdr.ScVal.scvU64(new xdr.Uint64(BigInt(n))),

  i128: (n: bigint | number | string = "123456789012345678901234567890"): xdr.ScVal => {
    const val = BigInt(n);
    const hi = val >> 64n;
    const lo = val & 0xffffffffffffffffn;
    return xdr.ScVal.scvI128(
      new xdr.Int128Parts({
        hi: new xdr.Int64(hi),
        lo: new xdr.Uint64(lo),
      }),
    );
  },

  string: (s = "test-string"): xdr.ScVal => xdr.ScVal.scvString(s),

  symbol: (s = "TEST_SYM"): xdr.ScVal => xdr.ScVal.scvSymbol(s),

  address: (
    addr = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
  ): xdr.ScVal => Address.fromString(addr).toScVal(),

  bool: (b = true): xdr.ScVal => xdr.ScVal.scvBool(b),

  vec: (items: xdr.ScVal[] = []): xdr.ScVal => xdr.ScVal.scvVec(items),

  map: (
    entries: Record<string, xdr.ScVal> | [string, xdr.ScVal][] = {},
    keyType: "symbol" | "string" = "symbol",
  ): xdr.ScVal => {
    const entryPairs = Array.isArray(entries) ? entries : Object.entries(entries);
    const scMapEntries = entryPairs.map(
      ([k, v]) =>
        new xdr.ScMapEntry({
          key: keyType === "string" ? xdr.ScVal.scvString(k) : xdr.ScVal.scvSymbol(k),
          val: v,
        }),
    );
    return xdr.ScVal.scvMap(scMapEntries);
  },

  /** Helper creating nested maps with arbitrary depth */
  nestedMap: (): xdr.ScVal => {
    const innerMap = scValFixtures.map({
      innerKey: scValFixtures.string("innerValue"),
      innerNum: scValFixtures.u32(99),
      innerAmount: scValFixtures.i128("98765432109876543210"),
    });
    return scValFixtures.map({
      name: scValFixtures.string("outer"),
      subMap: innerMap,
      tags: scValFixtures.vec([
        scValFixtures.string("alpha"),
        scValFixtures.string("beta"),
      ]),
    });
  },

  /** Helper creating nested vecs with arbitrary depth */
  nestedVec: (): xdr.ScVal => {
    return scValFixtures.vec([
      scValFixtures.vec([scValFixtures.u32(1), scValFixtures.u32(2)]),
      scValFixtures.vec([
        scValFixtures.string("nested-item"),
        scValFixtures.i128("-5000000000000000000"),
      ]),
    ]);
  },
};
