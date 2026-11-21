import { describe, it, expect } from "vitest";
import {
  normalizePagination,
  getPaginationOffset,
  calculatePaginationMeta,
  encodeCursor,
  decodeCursor,
  hasCursorPageConflict,
  DEFAULT_PAGE,
  DEFAULT_LIMIT,
  MIN_LIMIT,
  MAX_LIMIT,
  MAX_PAGE,
} from "../../src/utils/pagination";

describe("utils/pagination - boundary values and edge cases (#1431)", () => {
  describe("normalizePagination", () => {
    describe("zero values", () => {
      it("normalizes page=0 to default page", () => {
        const result = normalizePagination(0, 20);
        expect(result.page).toBe(DEFAULT_PAGE);
        expect(result.limit).toBe(20);
      });

      it("normalizes limit=0 to default limit", () => {
        const result = normalizePagination(1, 0);
        expect(result.page).toBe(1);
        expect(result.limit).toBe(DEFAULT_LIMIT);
      });

      it("normalizes both page=0 and limit=0 to defaults", () => {
        const result = normalizePagination(0, 0);
        expect(result.page).toBe(DEFAULT_PAGE);
        expect(result.limit).toBe(DEFAULT_LIMIT);
      });

      it("respects custom default options when normalizing zero values", () => {
        const result = normalizePagination(0, 0, {
          defaultPage: 5,
          defaultLimit: 50,
        });
        expect(result.page).toBe(5);
        expect(result.limit).toBe(50);
      });
    });

    describe("negative values", () => {
      it("normalizes negative page (-1, -100) to default page", () => {
        expect(normalizePagination(-1, 20).page).toBe(DEFAULT_PAGE);
        expect(normalizePagination(-100, 20).page).toBe(DEFAULT_PAGE);
      });

      it("normalizes negative limit (-1, -50) to default limit", () => {
        expect(normalizePagination(1, -1).limit).toBe(DEFAULT_LIMIT);
        expect(normalizePagination(1, -50).limit).toBe(DEFAULT_LIMIT);
      });

      it("normalizes both negative page and limit to defaults", () => {
        const result = normalizePagination(-5, -10);
        expect(result.page).toBe(DEFAULT_PAGE);
        expect(result.limit).toBe(DEFAULT_LIMIT);
      });
    });

    describe("very large values", () => {
      it("clamps very large limit exceeding MAX_LIMIT to MAX_LIMIT (100)", () => {
        expect(normalizePagination(1, 500).limit).toBe(MAX_LIMIT);
        expect(normalizePagination(1, 100000).limit).toBe(MAX_LIMIT);
        expect(normalizePagination(1, Number.MAX_SAFE_INTEGER).limit).toBe(MAX_LIMIT);
      });

      it("handles very large page clamped to MAX_PAGE (MAX_SAFE_INTEGER)", () => {
        const largePage = 10_000_000;
        expect(normalizePagination(largePage, 20).page).toBe(largePage);
        expect(normalizePagination(Number.MAX_SAFE_INTEGER, 20).page).toBe(MAX_PAGE);
      });

      it("respects custom maxLimit in options", () => {
        const result = normalizePagination(1, 50, { maxLimit: 25 });
        expect(result.limit).toBe(25);
      });
    });

    describe("floating point and non-integer values", () => {
      it("floors floating point page and limit numbers", () => {
        const result = normalizePagination(3.8, 15.9);
        expect(result.page).toBe(3);
        expect(result.limit).toBe(15);
      });

      it("floors page between 0 and 1 down, falling back to default", () => {
        expect(normalizePagination(0.7, 20).page).toBe(DEFAULT_PAGE);
      });
    });

    describe("invalid and non-numeric values", () => {
      it("falls back to defaults on NaN and non-finite numbers", () => {
        expect(normalizePagination(NaN, NaN)).toEqual({
          page: DEFAULT_PAGE,
          limit: DEFAULT_LIMIT,
        });
        expect(normalizePagination(Infinity, Infinity)).toEqual({
          page: DEFAULT_PAGE,
          limit: DEFAULT_LIMIT,
        });
        expect(normalizePagination(-Infinity, -Infinity)).toEqual({
          page: DEFAULT_PAGE,
          limit: DEFAULT_LIMIT,
        });
      });

      it("handles string representations of numbers", () => {
        expect(normalizePagination("5", "25")).toEqual({ page: 5, limit: 25 });
        expect(normalizePagination("-5", "0")).toEqual({
          page: DEFAULT_PAGE,
          limit: DEFAULT_LIMIT,
        });
      });

      it("falls back to defaults on non-numeric strings, null, and undefined", () => {
        expect(normalizePagination("invalid", "not-a-number")).toEqual({
          page: DEFAULT_PAGE,
          limit: DEFAULT_LIMIT,
        });
        expect(normalizePagination(null, null)).toEqual({
          page: DEFAULT_PAGE,
          limit: DEFAULT_LIMIT,
        });
        expect(normalizePagination(undefined, undefined)).toEqual({
          page: DEFAULT_PAGE,
          limit: DEFAULT_LIMIT,
        });
      });
    });
  });

  describe("getPaginationOffset", () => {
    it("calculates 0 offset for page=1", () => {
      expect(getPaginationOffset(1, 20)).toBe(0);
      expect(getPaginationOffset(1, 50)).toBe(0);
    });

    it("calculates correct offset for subsequent pages", () => {
      expect(getPaginationOffset(2, 20)).toBe(20);
      expect(getPaginationOffset(3, 20)).toBe(40);
      expect(getPaginationOffset(10, 50)).toBe(450);
    });

    it("handles zero and negative page boundary values safely", () => {
      expect(getPaginationOffset(0, 20)).toBe(0);
      expect(getPaginationOffset(-1, 20)).toBe(0);
      expect(getPaginationOffset(-100, 20)).toBe(0);
    });

    it("handles zero and negative limit boundary values safely", () => {
      expect(getPaginationOffset(2, 0)).toBe(20); // defaults to limit=20 -> (2-1)*20 = 20
      expect(getPaginationOffset(2, -10)).toBe(20);
    });

    it("handles very large values without overflowing into Infinity or NaN", () => {
      const offset = getPaginationOffset(100_000, 100);
      expect(offset).toBe(99_999 * 100);
      expect(Number.isFinite(offset)).toBe(true);

      const maxOffset = getPaginationOffset(Number.MAX_SAFE_INTEGER, 100);
      expect(maxOffset).toBeLessThanOrEqual(Number.MAX_SAFE_INTEGER);
      expect(Number.isFinite(maxOffset)).toBe(true);
    });
  });

  describe("calculatePaginationMeta", () => {
    it("handles total=0 boundary", () => {
      const meta = calculatePaginationMeta(0, 1, 20);
      expect(meta).toEqual({
        total: 0,
        page: 1,
        limit: 20,
        totalPages: 0,
        hasNextPage: false,
        hasPrevPage: false,
      });
    });

    it("handles negative total boundary by clamping to 0", () => {
      const meta = calculatePaginationMeta(-10, 1, 20);
      expect(meta.total).toBe(0);
      expect(meta.totalPages).toBe(0);
    });

    it("calculates pages correctly when total is an exact multiple of limit", () => {
      const meta = calculatePaginationMeta(100, 2, 20);
      expect(meta.totalPages).toBe(5);
      expect(meta.hasNextPage).toBe(true);
      expect(meta.hasPrevPage).toBe(true);
    });

    it("calculates pages correctly when total has remainder", () => {
      const meta = calculatePaginationMeta(101, 1, 20);
      expect(meta.totalPages).toBe(6);
      expect(meta.hasNextPage).toBe(true);
      expect(meta.hasPrevPage).toBe(false);
    });

    it("handles page beyond totalPages correctly", () => {
      const meta = calculatePaginationMeta(50, 10, 20);
      expect(meta.totalPages).toBe(3);
      expect(meta.page).toBe(10);
      expect(meta.hasNextPage).toBe(false);
      expect(meta.hasPrevPage).toBe(true);
    });

    it("handles very large total values safely", () => {
      const meta = calculatePaginationMeta(1_000_000, 1, 100);
      expect(meta.totalPages).toBe(10_000);
      expect(meta.hasNextPage).toBe(true);
    });
  });

  describe("encodeCursor & decodeCursor boundary values", () => {
    const fixedDate = new Date("2026-11-20T10:00:00.000Z");

    it("encodes and decodes id=0 boundary value", () => {
      const cursor = encodeCursor(0, fixedDate);
      const decoded = decodeCursor(cursor);
      expect(decoded.id).toBe(0);
      expect(decoded.ts).toBe(fixedDate.toISOString());
    });

    it("encodes and decodes very large id (Number.MAX_SAFE_INTEGER)", () => {
      const cursor = encodeCursor(Number.MAX_SAFE_INTEGER, fixedDate);
      const decoded = decodeCursor(cursor);
      expect(decoded.id).toBe(Number.MAX_SAFE_INTEGER);
      expect(decoded.ts).toBe(fixedDate.toISOString());
    });

    it("encodes and decodes standard positive id", () => {
      const cursor = encodeCursor(12345, fixedDate);
      const decoded = decodeCursor(cursor);
      expect(decoded.id).toBe(12345);
      expect(decoded.ts).toBe(fixedDate.toISOString());
    });

    it("throws when encoding negative id (-1, -100)", () => {
      expect(() => encodeCursor(-1, fixedDate)).toThrow(/Invalid id for cursor encoding/);
      expect(() => encodeCursor(-100, fixedDate)).toThrow(/Invalid id for cursor encoding/);
    });

    it("throws when encoding NaN or infinite id", () => {
      expect(() => encodeCursor(NaN, fixedDate)).toThrow(/Invalid id for cursor encoding/);
      expect(() => encodeCursor(Infinity, fixedDate)).toThrow(/Invalid id for cursor encoding/);
    });

    it("throws when encoding with an invalid Date", () => {
      expect(() => encodeCursor(1, new Date("invalid-date"))).toThrow(/Invalid timestamp/);
    });

    it("throws when decoding corrupted or malformed cursor", () => {
      expect(() => decodeCursor("not-base64url-json!")).toThrow(/Invalid or malformed cursor/);
      expect(() => decodeCursor("")).toThrow(/Invalid or malformed cursor/);
    });

    it("throws when decoding cursor with negative id or invalid shape", () => {
      const negativeIdCursor = Buffer.from(
        JSON.stringify({ id: -5, ts: fixedDate.toISOString() }),
      ).toString("base64url");
      expect(() => decodeCursor(negativeIdCursor)).toThrow(/Invalid cursor shape/);

      const invalidDateCursor = Buffer.from(
        JSON.stringify({ id: 10, ts: "not-a-date" }),
      ).toString("base64url");
      expect(() => decodeCursor(invalidDateCursor)).toThrow(/Invalid cursor timestamp/);
    });
  });

  describe("hasCursorPageConflict", () => {
    it("returns true when both page and cursor are present", () => {
      expect(hasCursorPageConflict("1", "cursor123")).toBe(true);
      expect(hasCursorPageConflict("0", "abc")).toBe(true);
    });

    it("returns false when only page is present", () => {
      expect(hasCursorPageConflict("1", undefined)).toBe(false);
    });

    it("returns false when only cursor is present", () => {
      expect(hasCursorPageConflict(undefined, "cursor123")).toBe(false);
    });

    it("returns false when neither is present", () => {
      expect(hasCursorPageConflict(undefined, undefined)).toBe(false);
    });
  });
});
