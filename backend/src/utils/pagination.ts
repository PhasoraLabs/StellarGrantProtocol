/**
 * Cursor-based and offset-based pagination utilities.
 *
 * Cursors are opaque base64url-encoded JSON blobs containing the last-seen
 * record's `id` and `ts` (ISO timestamp). Using both fields gives a stable
 * tie-break when multiple rows share the same timestamp.
 *
 * Encoding uses base64url (no padding, URL-safe) so cursors can be passed
 * directly as query parameters without percent-encoding.
 */

// ── Public constants & types ──────────────────────────────────────────────────

export const DEFAULT_PAGE = 1;
export const DEFAULT_LIMIT = 20;
export const MIN_LIMIT = 1;
export const MAX_LIMIT = 100;
export const MAX_PAGE = Number.MAX_SAFE_INTEGER;

export interface PageParams {
  page: number;
  limit: number;
}

export interface PaginationOptions {
  defaultPage?: number;
  defaultLimit?: number;
  minLimit?: number;
  maxLimit?: number;
}

export interface PaginationMeta {
  total: number;
  page: number;
  limit: number;
  totalPages: number;
  hasNextPage: boolean;
  hasPrevPage: boolean;
}

export interface CursorParams {
  /** Opaque base64url-encoded cursor returned by a previous response. */
  cursor?: string;
  limit: number;
  direction: "forward" | "backward";
}

export interface CursorResult<T> {
  items: T[];
  nextCursor: string | null;
  prevCursor: string | null;
  hasMore: boolean;
}

// ── Offset pagination helpers ─────────────────────────────────────────────────

/**
 * Normalizes raw page and limit input values, safeguarding against:
 * - Zero (coerced to default or minimum)
 * - Negative numbers (coerced to default or minimum)
 * - Non-integer numbers (floored to integer)
 * - Very large values (clamped to max limits or safe integer bounds)
 * - Non-numeric / NaN / Infinity / null / undefined (falling back to defaults)
 */
export function normalizePagination(
  rawPage: unknown,
  rawLimit: unknown,
  options: PaginationOptions = {},
): PageParams {
  const defaultPage = options.defaultPage ?? DEFAULT_PAGE;
  const defaultLimit = options.defaultLimit ?? DEFAULT_LIMIT;
  const minLimit = options.minLimit ?? MIN_LIMIT;
  const maxLimit = options.maxLimit ?? MAX_LIMIT;

  const numPage = typeof rawPage === "number" ? rawPage : Number(rawPage);
  const numLimit = typeof rawLimit === "number" ? rawLimit : Number(rawLimit);

  // Normalize page: must be a finite integer >= 1
  let page: number;
  if (!Number.isFinite(numPage) || numPage < 1) {
    page = defaultPage;
  } else {
    page = Math.min(Math.floor(numPage), MAX_PAGE);
  }

  // Normalize limit: must be a finite integer between minLimit and maxLimit
  let limit: number;
  if (!Number.isFinite(numLimit) || numLimit < minLimit) {
    limit = defaultLimit;
  } else {
    limit = Math.min(Math.max(Math.floor(numLimit), minLimit), maxLimit);
  }

  return { page, limit };
}

/**
 * Calculates SQL offset (skip) for a given page and limit.
 * Guaranteed to return a non-negative integer capped at MAX_SAFE_INTEGER.
 */
export function getPaginationOffset(page: number, limit: number): number {
  const safePage = Number.isFinite(page) && page >= 1 ? Math.floor(page) : DEFAULT_PAGE;
  const safeLimit = Number.isFinite(limit) && limit >= 1 ? Math.floor(limit) : DEFAULT_LIMIT;

  const offset = (safePage - 1) * safeLimit;
  if (!Number.isFinite(offset) || offset < 0) {
    return 0;
  }
  return Math.min(offset, Number.MAX_SAFE_INTEGER);
}

/**
 * Computes pagination metadata including totalPages, hasNextPage, and hasPrevPage.
 */
export function calculatePaginationMeta(
  total: number,
  page: number,
  limit: number,
): PaginationMeta {
  const safeTotal = Number.isFinite(total) && total >= 0 ? Math.floor(total) : 0;
  const safeLimit = Number.isFinite(limit) && limit >= 1 ? Math.floor(limit) : DEFAULT_LIMIT;
  const safePage = Number.isFinite(page) && page >= 1 ? Math.floor(page) : DEFAULT_PAGE;

  const totalPages = safeLimit > 0 ? Math.ceil(safeTotal / safeLimit) : 0;

  return {
    total: safeTotal,
    page: safePage,
    limit: safeLimit,
    totalPages,
    hasNextPage: safePage < totalPages,
    hasPrevPage: safePage > 1 && totalPages > 0,
  };
}

// ── Cursor codec ──────────────────────────────────────────────────────────────

/**
 * Encodes a record's id and timestamp into an opaque cursor string.
 *
 * @param id        The record's numeric primary key (must be a non-negative finite integer).
 * @param timestamp The record's timestamp column (updatedAt / createdAt / paidAt).
 */
export function encodeCursor(id: number, timestamp: Date): string {
  if (!Number.isFinite(id) || id < 0) {
    throw new Error("Invalid id for cursor encoding: must be a non-negative finite number");
  }
  if (!(timestamp instanceof Date) || Number.isNaN(timestamp.getTime())) {
    throw new Error("Invalid timestamp for cursor encoding");
  }

  return Buffer.from(
    JSON.stringify({ id: Math.floor(id), ts: timestamp.toISOString() }),
  ).toString("base64url");
}

/**
 * Decodes a cursor string back into its constituent parts.
 *
 * @throws If the cursor is malformed or cannot be parsed.
 */
export function decodeCursor(cursor: string): { id: number; ts: string } {
  let parsed: unknown;
  try {
    const raw = Buffer.from(cursor, "base64url").toString("utf8");
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("Invalid or malformed cursor");
  }

  if (typeof parsed !== "object" || parsed === null) {
    throw new Error("Invalid cursor shape");
  }

  const record = parsed as Record<string, unknown>;
  const id = record.id;
  const ts = record.ts;

  if (
    typeof id !== "number" ||
    !Number.isFinite(id) ||
    id < 0 ||
    typeof ts !== "string"
  ) {
    throw new Error("Invalid cursor shape");
  }

  // Validate that ts can be parsed as a valid date
  const date = new Date(ts);
  if (Number.isNaN(date.getTime())) {
    throw new Error("Invalid cursor timestamp");
  }

  return { id, ts };
}

// ── Guard ─────────────────────────────────────────────────────────────────────

/**
 * Returns true when the request supplies both `page` and `cursor` params,
 * which is not allowed. Callers should respond with HTTP 400.
 */
export function hasCursorPageConflict(
  page: string | undefined,
  cursor: string | undefined,
): boolean {
  return page !== undefined && cursor !== undefined;
}
