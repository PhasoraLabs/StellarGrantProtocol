/**
 * Type-safe ScVal decoders for Soroban contract return values.
 *
 * Each helper narrows an `xdr.ScVal` to a concrete TypeScript type and throws
 * with a descriptive message when the variant does not match expectations, so
 * callers never receive silent `undefined` or `null` for required fields.
 */

import { xdr } from "@stellar/stellar-sdk";

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
  return val.address().toString();
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
