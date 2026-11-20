/**
 * Multi-Token Milestone Tests
 *
 * Tests for handling milestones with different tokens.
 */

import { describe, it, expect } from "vitest";
import { Milestone, Grant } from "@/types";
import { formatTokenAmount } from "@/lib/tokens";

describe("Multi-Token Milestone Handling", () => {
  describe("milestone with different tokens", () => {
    it("should handle milestone with native token", () => {
      const milestone: Milestone = {
        idx: 0,
        title: "Phase 1",
        description: "First phase",
        proof_hash: null,
        submitted: false,
        approved: false,
        paid: false,
        submitted_at: null,
        approved_at: null,
        paid_at: null,
        token: "native",
        amount: 1000000000n, // 100 XLM
      };

      expect(milestone.token).toBe("native");
      expect(milestone.amount).toBe(1000000000n);
      
      const formatted = formatTokenAmount(milestone.amount!, 7, { symbol: "XLM", showSymbol: true });
      expect(formatted).toBe("100 XLM");
    });

    it("should handle milestone with USDC token", () => {
      const milestone: Milestone = {
        idx: 1,
        title: "Phase 2",
        description: "Second phase",
        proof_hash: null,
        submitted: false,
        approved: false,
        paid: false,
        submitted_at: null,
        approved_at: null,
        paid_at: null,
        token: "USDC",
        amount: 500000000n, // 500 USDC
      };

      expect(milestone.token).toBe("USDC");
      expect(milestone.amount).toBe(500000000n);
      
      const formatted = formatTokenAmount(milestone.amount!, 6, { symbol: "USDC", showSymbol: true });
      expect(formatted).toBe("500 USDC");
    });

    it("should handle milestone without token (fallback to grant token)", () => {
      const milestone: Milestone = {
        idx: 2,
        title: "Phase 3",
        description: "Third phase",
        proof_hash: null,
        submitted: false,
        approved: false,
        paid: false,
        submitted_at: null,
        approved_at: null,
        paid_at: null,
        // No token specified
      };

      expect(milestone.token).toBeUndefined();
    });
  });

  describe("grant with multiple token milestones", () => {
    it("should support milestones with mixed tokens", () => {
      const _grant: Grant = {
        id: "1",
        owner: "GABC123",
        title: "Multi-Token Grant",
        description: "Grant with mixed tokens",
        budget: 10000000000n,
        funded: 5000000000n,
        deadline: 1700000000n,
        status: 1,
        milestones: 3,
        reviewers: [],
        created_at: 1699000000n,
        token: "native",
      };

      const milestones: Milestone[] = [
        {
          idx: 0,
          title: "Milestone 1",
          description: "XLM payout",
          proof_hash: null,
          submitted: false,
          approved: false,
          paid: false,
          submitted_at: null,
          approved_at: null,
          paid_at: null,
          token: "native",
          amount: 3000000000n, // 300 XLM
        },
        {
          idx: 1,
          title: "Milestone 2",
          description: "USDC payout",
          proof_hash: null,
          submitted: false,
          approved: false,
          paid: false,
          submitted_at: null,
          approved_at: null,
          paid_at: null,
          token: "USDC",
          amount: 200000000n, // 200 USDC
        },
        {
          idx: 2,
          title: "Milestone 3",
          description: "XLM payout",
          proof_hash: null,
          submitted: false,
          approved: false,
          paid: false,
          submitted_at: null,
          approved_at: null,
          paid_at: null,
          token: "native",
          amount: 5000000000n, // 500 XLM
        },
      ];

      // Verify each milestone has correct token
      expect(milestones[0].token).toBe("native");
      expect(milestones[1].token).toBe("USDC");
      expect(milestones[2].token).toBe("native");

      // Verify amounts are formatted correctly per token
      const xlmMilestone = milestones[0];
      const usdcMilestone = milestones[1];

      const xlmFormatted = formatTokenAmount(xlmMilestone.amount!, 7, { symbol: "XLM", showSymbol: true });
      const usdcFormatted = formatTokenAmount(usdcMilestone.amount!, 6, { symbol: "USDC", showSymbol: true });

      expect(xlmFormatted).toBe("300 XLM");
      expect(usdcFormatted).toBe("200 USDC");
    });
  });

  describe("no cross-token aggregation", () => {
    it("should not aggregate amounts from different tokens", () => {
      const milestones: Milestone[] = [
        {
          idx: 0,
          title: "Milestone 1",
          description: "XLM",
          proof_hash: null,
          submitted: false,
          approved: false,
          paid: false,
          submitted_at: null,
          approved_at: null,
          paid_at: null,
          token: "native",
          amount: 1000000000n, // 100 XLM
        },
        {
          idx: 1,
          title: "Milestone 2",
          description: "USDC",
          proof_hash: null,
          submitted: false,
          approved: false,
          paid: false,
          submitted_at: null,
          approved_at: null,
          paid_at: null,
          token: "USDC",
          amount: 100000000n, // 100 USDC
        },
      ];

      // Group by token
      const byToken = milestones.reduce((acc, m) => {
        const token = m.token || "unknown";
        if (!acc[token]) {
          acc[token] = 0n;
        }
        acc[token] += m.amount || 0n;
        return acc;
      }, {} as Record<string, bigint>);

      // Verify separate totals
      expect(byToken["native"]).toBe(1000000000n);
      expect(byToken["USDC"]).toBe(100000000n);

      // These should NOT be equal (different tokens, different values)
      expect(byToken["native"]).not.toBe(byToken["USDC"]);
    });
  });

  describe("edge cases", () => {
    it("should handle milestone with undefined amount", () => {
      const milestone: Milestone = {
        idx: 0,
        title: "No Amount",
        description: "Amount not set",
        proof_hash: null,
        submitted: false,
        approved: false,
        paid: false,
        submitted_at: null,
        approved_at: null,
        paid_at: null,
        token: "native",
        // amount is undefined
      };

      expect(milestone.amount).toBeUndefined();
    });

    it("should handle milestone with zero amount", () => {
      const milestone: Milestone = {
        idx: 0,
        title: "Free Milestone",
        description: "No payout",
        proof_hash: null,
        submitted: false,
        approved: false,
        paid: false,
        submitted_at: null,
        approved_at: null,
        paid_at: null,
        token: "native",
        amount: 0n,
      };

      expect(milestone.amount).toBe(0n);
      const formatted = formatTokenAmount(milestone.amount!, 7, { symbol: "XLM", showSymbol: true });
      expect(formatted).toBe("0 XLM");
    });

    it("should handle large amounts without precision loss", () => {
      const milestone: Milestone = {
        idx: 0,
        title: "Large Milestone",
        description: "Big payout",
        proof_hash: null,
        submitted: false,
        approved: false,
        paid: false,
        submitted_at: null,
        approved_at: null,
        paid_at: null,
        token: "USDC",
        amount: BigInt("1000000000000"), // 1 million USDC
      };

      const formatted = formatTokenAmount(milestone.amount!, 6, { symbol: "USDC", showSymbol: true });
      expect(formatted).toBe("1,000,000 USDC");
    });

    it("should handle very small amounts correctly", () => {
      const milestone: Milestone = {
        idx: 0,
        title: "Tiny Milestone",
        description: "Small payout",
        proof_hash: null,
        submitted: false,
        approved: false,
        paid: false,
        submitted_at: null,
        approved_at: null,
        paid_at: null,
        token: "native",
        amount: 1n, // 1 stroop
      };

      const formatted = formatTokenAmount(milestone.amount!, 7, { symbol: "XLM", showSymbol: true });
      expect(formatted).toBe("0.0000001 XLM");
    });
  });

  describe("token metadata handling", () => {
    it("should use correct decimals per token", () => {
      const xlmAmount = 1000000000n; // 100 XLM
      const usdcAmount = 100000000n; // 100 USDC

      const xlmFormatted = formatTokenAmount(xlmAmount, 7, { symbol: "XLM", showSymbol: true });
      const usdcFormatted = formatTokenAmount(usdcAmount, 6, { symbol: "USDC", showSymbol: true });

      // Both represent 100 units but with different decimal representations
      expect(xlmFormatted).toBe("100 XLM");
      expect(usdcFormatted).toBe("100 USDC");
    });

    it("should handle unknown token with fallback", () => {
      const amount = 1000000000n;
      const formatted = formatTokenAmount(amount, 7, { symbol: "UNKNOWN", showSymbol: true });
      expect(formatted).toBe("100 UNKNOWN");
    });
  });
});
