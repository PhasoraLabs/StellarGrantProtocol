import { describe, it, expect } from "vitest";
import {
  QUORUM_THRESHOLD,
  requiredApprovals,
  quorumReached,
} from "@/lib/utils/quorum";

describe("requiredApprovals", () => {
  it("returns 0 for empty reviewer sets", () => {
    expect(requiredApprovals(0)).toBe(0);
    expect(requiredApprovals(-1)).toBe(0);
  });

  it("matches on-chain majority (strictly more than half)", () => {
    expect(requiredApprovals(1)).toBe(1);
    expect(requiredApprovals(2)).toBe(2);
    expect(requiredApprovals(3)).toBe(2);
    expect(requiredApprovals(4)).toBe(3);
    expect(requiredApprovals(5)).toBe(3);
  });
});

describe("quorumReached", () => {
  it("mirrors governance::quorum_reached", () => {
    expect(quorumReached(0, 0)).toBe(false);
    expect(quorumReached(1, 1)).toBe(true);
    expect(quorumReached(1, 2)).toBe(false);
    expect(quorumReached(2, 3)).toBe(true);
    expect(quorumReached(3, 5)).toBe(true);
    expect(quorumReached(2, 5)).toBe(false);
  });
});

describe("QUORUM_THRESHOLD", () => {
  it("is a simple majority (>50%)", () => {
    expect(QUORUM_THRESHOLD).toBe(0.5);
  });
});
