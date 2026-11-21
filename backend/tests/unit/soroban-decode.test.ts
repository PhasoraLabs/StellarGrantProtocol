import { describe, it, expect } from "vitest";
import {
  decodeU32,
  decodeU64,
  decodeI128AsString,
  decodeString,
  decodeAddress,
  decodeBool,
  decodeVec,
  decodeMap,
  scValFixtures,
} from "../../src/soroban/decode";

describe("soroban/decode", () => {
  describe("decodeU32", () => {
    it("decodes a valid u32 value", () => {
      const val = scValFixtures.u32(100);
      expect(decodeU32(val)).toBe(100);
    });

    it("decodes u32 at boundaries (0 and max uint32)", () => {
      expect(decodeU32(scValFixtures.u32(0))).toBe(0);
      expect(decodeU32(scValFixtures.u32(4294967295))).toBe(4294967295);
    });

    it("throws when passed a non-u32 ScVal", () => {
      const notU32 = scValFixtures.string("not-a-number");
      expect(() => decodeU32(notU32)).toThrow(/Expected scvU32, got/);
    });
  });

  describe("decodeU64", () => {
    it("decodes a valid u64 value into a JS number", () => {
      const val = scValFixtures.u64(987654321);
      expect(decodeU64(val)).toBe(987654321);
    });

    it("decodes u64 zero boundary", () => {
      expect(decodeU64(scValFixtures.u64(0))).toBe(0);
    });

    it("throws when passed a non-u64 ScVal", () => {
      const notU64 = scValFixtures.u32(123);
      expect(() => decodeU64(notU64)).toThrow(/Expected scvU64, got/);
    });
  });

  describe("decodeI128AsString", () => {
    it("decodes zero i128 correctly", () => {
      const scv = scValFixtures.i128(0n);
      expect(decodeI128AsString(scv)).toBe("0");
    });

    it("decodes standard positive i128 values", () => {
      const num = 123456789012345678901234567890n;
      const scv = scValFixtures.i128(num);
      expect(decodeI128AsString(scv)).toBe(num.toString());
    });

    it("decodes boundary positive i128 exceeding 2^64", () => {
      const largeNum = 18446744073709551616n + 500n; // > 2^64
      const scv = scValFixtures.i128(largeNum);
      expect(decodeI128AsString(scv)).toBe(largeNum.toString());
    });

    it("decodes max signed 128-bit integer (2^127 - 1)", () => {
      const maxI128 = 2n ** 127n - 1n;
      const scv = scValFixtures.i128(maxI128);
      expect(decodeI128AsString(scv)).toBe(maxI128.toString());
    });

    it("decodes negative i128 values", () => {
      const negNum = -123456789012345678901234567890n;
      const scv = scValFixtures.i128(negNum);
      expect(decodeI128AsString(scv)).toBe(negNum.toString());
    });

    it("decodes min signed 128-bit integer (-2^127)", () => {
      const minI128 = -(2n ** 127n);
      const scv = scValFixtures.i128(minI128);
      expect(decodeI128AsString(scv)).toBe(minI128.toString());
    });

    it("throws when passed a non-i128 ScVal", () => {
      const notI128 = scValFixtures.u64(100);
      expect(() => decodeI128AsString(notI128)).toThrow(/Expected scvI128, got/);
    });
  });

  describe("decodeString", () => {
    it("decodes scvString variant", () => {
      const val = scValFixtures.string("stellar grants");
      expect(decodeString(val)).toBe("stellar grants");
    });

    it("decodes scvSymbol variant", () => {
      const val = scValFixtures.symbol("MILESTONE_STATUS");
      expect(decodeString(val)).toBe("MILESTONE_STATUS");
    });

    it("throws when passed non-string and non-symbol", () => {
      const val = scValFixtures.bool(true);
      expect(() => decodeString(val)).toThrow(/Expected scvString or scvSymbol, got/);
    });
  });

  describe("decodeAddress", () => {
    it("decodes scvAddress into a valid StrKey format", () => {
      const addr = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
      const val = scValFixtures.address(addr);
      expect(decodeAddress(val)).toBe(addr);
    });

    it("throws when passed a non-address ScVal", () => {
      const val = scValFixtures.string("GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5");
      expect(() => decodeAddress(val)).toThrow(/Expected scvAddress, got/);
    });
  });

  describe("decodeBool", () => {
    it("decodes boolean true and false", () => {
      expect(decodeBool(scValFixtures.bool(true))).toBe(true);
      expect(decodeBool(scValFixtures.bool(false))).toBe(false);
    });

    it("throws when passed a non-bool ScVal", () => {
      expect(() => decodeBool(scValFixtures.u32(1))).toThrow(/Expected scvBool, got/);
    });
  });

  describe("decodeVec (including nested vecs)", () => {
    it("decodes an empty vec", () => {
      const val = scValFixtures.vec([]);
      expect(decodeVec(val)).toEqual([]);
    });

    it("decodes a flat vec of items", () => {
      const val = scValFixtures.vec([
        scValFixtures.u32(1),
        scValFixtures.u32(2),
        scValFixtures.string("three"),
      ]);
      const elements = decodeVec(val);
      expect(elements).toHaveLength(3);
      expect(decodeU32(elements[0])).toBe(1);
      expect(decodeU32(elements[1])).toBe(2);
      expect(decodeString(elements[2])).toBe("three");
    });

    it("decodes nested vecs", () => {
      const nested = scValFixtures.nestedVec();
      const outerVec = decodeVec(nested);
      expect(outerVec).toHaveLength(2);

      const firstInner = decodeVec(outerVec[0]);
      expect(firstInner).toHaveLength(2);
      expect(decodeU32(firstInner[0])).toBe(1);
      expect(decodeU32(firstInner[1])).toBe(2);

      const secondInner = decodeVec(outerVec[1]);
      expect(secondInner).toHaveLength(2);
      expect(decodeString(secondInner[0])).toBe("nested-item");
      expect(decodeI128AsString(secondInner[1])).toBe("-5000000000000000000");
    });

    it("throws when passed a non-vec ScVal", () => {
      expect(() => decodeVec(scValFixtures.bool(true))).toThrow(/Expected scvVec, got/);
    });
  });

  describe("decodeMap (including nested maps and vecs in maps)", () => {
    it("decodes an empty map", () => {
      const val = scValFixtures.map({});
      const map = decodeMap(val);
      expect(map.size).toBe(0);
    });

    it("decodes a flat map with symbol keys", () => {
      const val = scValFixtures.map({
        grantId: scValFixtures.u64(42),
        title: scValFixtures.string("Open Source Grant"),
        funded: scValFixtures.bool(true),
        amount: scValFixtures.i128("1000000000000000000"),
      });
      const map = decodeMap(val);
      expect(map.size).toBe(4);
      expect(decodeU64(map.get("grantId")!)).toBe(42);
      expect(decodeString(map.get("title")!)).toBe("Open Source Grant");
      expect(decodeBool(map.get("funded")!)).toBe(true);
      expect(decodeI128AsString(map.get("amount")!)).toBe("1000000000000000000");
    });

    it("decodes a map with string keys", () => {
      const val = scValFixtures.map(
        { strKey: scValFixtures.u32(10) },
        "string",
      );
      const map = decodeMap(val);
      expect(decodeU32(map.get("strKey")!)).toBe(10);
    });

    it("decodes nested maps containing sub-maps, vecs and i128 values", () => {
      const val = scValFixtures.nestedMap();
      const outerMap = decodeMap(val);

      expect(decodeString(outerMap.get("name")!)).toBe("outer");

      // Verify nested sub-map
      const innerMapScVal = outerMap.get("subMap");
      expect(innerMapScVal).toBeDefined();
      const innerMap = decodeMap(innerMapScVal!);
      expect(innerMap.size).toBe(3);
      expect(decodeString(innerMap.get("innerKey")!)).toBe("innerValue");
      expect(decodeU32(innerMap.get("innerNum")!)).toBe(99);
      expect(decodeI128AsString(innerMap.get("innerAmount")!)).toBe("98765432109876543210");

      // Verify nested vec inside map
      const tagsScVal = outerMap.get("tags");
      expect(tagsScVal).toBeDefined();
      const tags = decodeVec(tagsScVal!).map(decodeString);
      expect(tags).toEqual(["alpha", "beta"]);
    });

    it("throws when passed a non-map ScVal", () => {
      expect(() => decodeMap(scValFixtures.u64(10))).toThrow(/Expected scvMap, got/);
    });
  });
});
