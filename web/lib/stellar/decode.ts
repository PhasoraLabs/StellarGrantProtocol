/**
 * ScVal Decoder
 *
 * Converts Soroban XDR ScVal objects into plain TypeScript values.
 * Used by ContractClient read methods to interpret simulation results.
 */

import { xdr } from "@stellar/stellar-sdk";

/**
 * Decode an XDR ScVal to a typed TypeScript value.
 *
 * Handles the common conversions required by the StellarGrants contract:
 * - ScvU64 / ScvI128 → bigint
 * - ScvAddress        → string (strkey)
 * - ScvString / ScvSymbol → string
 * - ScvVec           → array (each element recursively decoded)
 * - ScvMap           → Record<string, unknown>
 * - ScvBool          → boolean
 * - ScvVoid          → null
 */
export function decodeScVal<T = unknown>(val: xdr.ScVal): T {
  switch (val.switch().name) {
    case "scvBool":
      return val.b() as unknown as T;

    case "scvU32":
      return val.u32() as unknown as T;

    case "scvI32":
      return val.i32() as unknown as T;

    case "scvU64": {
      const u64 = val.u64();
      return BigInt(u64.toBigInt ? u64.toBigInt() : u64.toString()) as unknown as T;
    }

    case "scvI64": {
      const i64 = val.i64();
      return BigInt(i64.toBigInt ? i64.toBigInt() : i64.toString()) as unknown as T;
    }

    case "scvU128": {
      const u128 = val.u128();
      const hi = BigInt(u128.hi().toString());
      const lo = BigInt(u128.lo().toString());
      return ((hi << 64n) | lo) as unknown as T;
    }

    case "scvI128": {
      const i128 = val.i128();
      const hi = BigInt(i128.hi().toString());
      const lo = BigInt(i128.lo().toString());
      return ((hi << 64n) | lo) as unknown as T;
    }

    case "scvString": {
      const raw = val.str();
      const str = raw instanceof Buffer ? raw.toString("utf8") : String(raw);
      return str as unknown as T;
    }

    case "scvSymbol": {
      const raw = val.sym();
      const sym = raw instanceof Buffer ? raw.toString("utf8") : String(raw);
      return sym as unknown as T;
    }

    case "scvAddress":
      return val.address().toString() as unknown as T;

    case "scvBytes": {
      const bytes = val.bytes();
      return bytes.toString("hex") as unknown as T;
    }

    case "scvVec": {
      const vec = val.vec();
      if (!vec) return [] as unknown as T;
      return vec.map((item) => decodeScVal(item)) as unknown as T;
    }

    case "scvMap": {
      const map = val.map();
      if (!map) return {} as unknown as T;
      const obj: Record<string, unknown> = {};
      for (const entry of map) {
        const key = String(decodeScVal(entry.key()));
        obj[key] = decodeScVal(entry.val());
      }
      return obj as unknown as T;
    }

    case "scvVoid":
      return null as unknown as T;

    default:
      // Return the raw value for unhandled types so callers can inspect if needed
      return val as unknown as T;
  }
}
