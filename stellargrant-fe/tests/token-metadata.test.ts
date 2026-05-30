/**
 * Token Metadata Service Tests
 *
 * Tests for getTokenMetadata, caching, and batch operations.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  getTokenMetadata,
  getTokenMetadataBatch,
  clearTokenMetadataCache,
  getCachedTokenMetadata,
  USDC_CONTRACT_ADDRESS,
} from "@/lib/tokens/metadata";

describe("Token Metadata Service", () => {
  beforeEach(() => {
    clearTokenMetadataCache();
  });

  describe("getTokenMetadata", () => {
    it("should return metadata for native token", async () => {
      const metadata = await getTokenMetadata("native");
      expect(metadata.symbol).toBe("XLM");
      expect(metadata.decimals).toBe(7);
      expect(metadata.name).toBe("Stellar Lumens");
    });

    it("should return metadata for USDC", async () => {
      const metadata = await getTokenMetadata("USDC");
      expect(metadata.symbol).toBe("USDC");
      expect(metadata.decimals).toBe(6);
    });

    it("should cache metadata after first fetch", async () => {
      // First call
      const metadata1 = await getTokenMetadata("native");
      
      // Check cache
      const cached = getCachedTokenMetadata("native");
      expect(cached).toBeDefined();
      expect(cached?.symbol).toBe("XLM");
      
      // Second call should use cache
      const metadata2 = await getTokenMetadata("native");
      expect(metadata2).toBe(metadata1); // Same reference
    });

    it("should handle case-insensitive symbol lookup", async () => {
      const metadata = await getTokenMetadata("usdc");
      expect(metadata.symbol).toBe("USDC");
    });

    it("should return fallback metadata for unknown tokens", async () => {
      const metadata = await getTokenMetadata("UNKNOWN_TOKEN_ADDRESS");
      expect(metadata.symbol).toBe("UNKNOWN");
      expect(metadata.decimals).toBe(7);
      expect(metadata.address).toBe("UNKNOWN_TOKEN_ADDRESS");
    });

    it("should cache fallback metadata", async () => {
      await getTokenMetadata("GABC123");
      const cached = getCachedTokenMetadata("GABC123");
      expect(cached).toBeDefined();
      expect(cached?.symbol).toBe("UNKNOWN");
    });
  });

  describe("getTokenMetadataBatch", () => {
    it("should fetch metadata for multiple tokens", async () => {
      const tokens = ["native", "USDC", "GABC123"];
      const results = await getTokenMetadataBatch(tokens);

      expect(results.size).toBe(3);
      expect(results.get("native")?.symbol).toBe("XLM");
      expect(results.get("USDC")?.symbol).toBe("USDC");
      expect(results.get("GABC123")?.symbol).toBe("UNKNOWN");
    });

    it("should deduplicate tokens", async () => {
      const tokens = ["native", "native", "USDC", "native"];
      const results = await getTokenMetadataBatch(tokens);

      expect(results.size).toBe(2);
    });

    it("should use cache for already-fetched tokens", async () => {
      // Pre-populate cache
      await getTokenMetadata("native");

      // Batch fetch including cached token
      const results = await getTokenMetadataBatch(["native", "USDC"]);

      expect(results.size).toBe(2);
      expect(getCachedTokenMetadata("native")).toBeDefined();
      expect(getCachedTokenMetadata("USDC")).toBeDefined();
    });
  });

  describe("clearTokenMetadataCache", () => {
    it("should clear all cached metadata", async () => {
      // Populate cache
      await getTokenMetadata("native");
      await getTokenMetadata("USDC");

      expect(getCachedTokenMetadata("native")).toBeDefined();
      expect(getCachedTokenMetadata("USDC")).toBeDefined();

      // Clear cache
      clearTokenMetadataCache();

      expect(getCachedTokenMetadata("native")).toBeUndefined();
      expect(getCachedTokenMetadata("USDC")).toBeUndefined();
    });
  });

  describe("getCachedTokenMetadata", () => {
    it("should return undefined for uncached tokens", () => {
      expect(getCachedTokenMetadata("NOT_CACHED")).toBeUndefined();
    });

    it("should return metadata for cached tokens", async () => {
      await getTokenMetadata("native");
      const cached = getCachedTokenMetadata("native");
      expect(cached).toBeDefined();
      expect(cached?.symbol).toBe("XLM");
    });
  });

  describe("USDC SAC metadata", () => {
    it("exposes a testnet USDC contract address", () => {
      expect(USDC_CONTRACT_ADDRESS).toMatch(/^C[A-Z2-7]+$/); // Soroban contract id (StrKey)
    });

    it("carries the SAC address and 6 decimals when resolved by symbol", async () => {
      const metadata = await getTokenMetadata("USDC");
      expect(metadata.decimals).toBe(6);
      expect(metadata.address).toBe(USDC_CONTRACT_ADDRESS);
    });

    it("resolves USDC by its SAC contract address", async () => {
      const metadata = await getTokenMetadata(USDC_CONTRACT_ADDRESS);
      expect(metadata.symbol).toBe("USDC");
      expect(metadata.decimals).toBe(6);
    });
  });
});
