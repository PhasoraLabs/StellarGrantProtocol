import { describe, it, expect } from "vitest";
import {
  mapApiGrantDetail,
  normalizeGrantToken,
  type ApiGrantDetail,
} from "@/lib/grants/api";

describe("normalizeGrantToken", () => {
  it("maps XLM labels to the native sentinel", () => {
    expect(normalizeGrantToken("XLM")).toBe("native");
    expect(normalizeGrantToken("xlm")).toBe("native");
    expect(normalizeGrantToken("native")).toBe("native");
  });

  it("preserves USDC and contract addresses", () => {
    expect(normalizeGrantToken("USDC")).toBe("USDC");
    expect(normalizeGrantToken("CABC123")).toBe("CABC123");
  });

  it("defaults missing values to native", () => {
    expect(normalizeGrantToken(undefined)).toBe("native");
    expect(normalizeGrantToken("")).toBe("native");
  });
});

describe("mapApiGrantDetail token", () => {
  const base: ApiGrantDetail = {
    id: 2,
    title: "USDC Grant",
    status: "active",
    recipient: "GRECIPIENT",
    totalAmount: "100000000",
  };

  it("threads the API token through instead of hardcoding native", () => {
    const { grant } = mapApiGrantDetail({ ...base, token: "USDC" });
    expect(grant.token).toBe("USDC");
  });

  it("normalises XLM from the API to native", () => {
    const { grant } = mapApiGrantDetail({ ...base, token: "XLM" });
    expect(grant.token).toBe("native");
  });

  it("falls back to a milestone token when grant-level token is absent", () => {
    const { grant } = mapApiGrantDetail({
      ...base,
      milestones: [
        {
          idx: 0,
          title: "m0",
          deadline: "2026-12-31",
          token: "USDC",
        },
      ],
    });
    expect(grant.token).toBe("USDC");
  });
});
