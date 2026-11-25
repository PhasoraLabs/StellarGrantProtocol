import { describe, expect, it } from "vitest";

import { milestoneProofSchema } from "../../src/schemas";

const validProof = {
  grantId: 1,
  milestoneIdx: 0,
  proofCid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3dxf7u7xpv6z6t7i7l7f7q7e",
  submittedBy: "GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV",
  signature: "A".repeat(64), // typical ed25519 base64 payload sits around 88 chars
  nonce: "nonce-1234",
  timestamp: 1_700_000_000_000,
};

describe("milestoneProofSchema field length limits", () => {
  it("accepts a signature inside the 32..128 character window", () => {
    const parsed = milestoneProofSchema.safeParse(validProof);
    expect(parsed.success).toBe(true);
  });

  it("accepts the exact lower bound of 32 characters", () => {
    const parsed = milestoneProofSchema.safeParse({
      ...validProof,
      signature: "A".repeat(32),
    });
    expect(parsed.success).toBe(true);
  });

  it("accepts the exact upper bound of 128 characters", () => {
    const parsed = milestoneProofSchema.safeParse({
      ...validProof,
      signature: "A".repeat(128),
    });
    expect(parsed.success).toBe(true);
  });

  it("rejects a signature shorter than 32 characters", () => {
    const parsed = milestoneProofSchema.safeParse({
      ...validProof,
      signature: "A".repeat(31),
    });
    expect(parsed.success).toBe(false);
  });

  it("rejects an oversized signature payload", () => {
    const parsed = milestoneProofSchema.safeParse({
      ...validProof,
      signature: "A".repeat(129),
    });
    expect(parsed.success).toBe(false);
    if (!parsed.success) {
      expect(parsed.error.issues.some((issue) => issue.path.includes("signature"))).toBe(true);
    }
  });

  it("rejects a signature far beyond any valid encoding (payload-flood case)", () => {
    const parsed = milestoneProofSchema.safeParse({
      ...validProof,
      signature: "A".repeat(10_000),
    });
    expect(parsed.success).toBe(false);
  });
});
