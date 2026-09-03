import { describe, it, expect } from "vitest";
import { matchesQuery, addressToColor } from "@/lib/search/map";

describe("search map helpers", () => {
  it("matches all query words in text", () => {
    expect(matchesQuery("Stellar Infrastructure Grant", "stellar infra")).toBe(true);
    expect(matchesQuery("Stellar Grant", "stellar infra")).toBe(false);
  });

  it("returns deterministic identicon colours", () => {
    const a = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const b = "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
    expect(addressToColor(a)).toBe(addressToColor(a));
    expect(addressToColor(a)).not.toBe(addressToColor(b));
  });
});
