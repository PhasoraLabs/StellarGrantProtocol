/**
 * Cursor-based pagination utilities.
 *
 * Cursors are opaque base64url-encoded JSON blobs containing the last-seen
 * record's `id` and `ts` (ISO timestamp). Using both fields gives a stable
 * tie-break when multiple rows share the same timestamp.
 *
 * Encoding uses base64url (no padding, URL-safe) so cursors can be passed
 * directly as query parameters without percent-encoding.
 */

// ── Public types ──────────────────────────────────────────────────────────────

export interface PageParams {
  page: number;
  limit: number;
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

// ── Cursor codec ──────────────────────────────────────────────────────────────

/**
 * Encodes a record's id and timestamp into an opaque cursor string.
 *
 * @param id        The record's numeric primary key.
 * @param timestamp The record's timestamp column (updatedAt / createdAt / paidAt).
 */
export function encodeCursor(id: number, timestamp: Date): string {
  return Buffer.from(
    JSON.stringify({ id, ts: timestamp.toISOString() }),
  ).toString("base64url");
}

/**
 * Decodes a cursor string back into its constituent parts.
 *
 * @throws If the cursor is malformed or cannot be parsed.
 */
export function decodeCursor(cursor: string): { id: number; ts: string } {
  try {
    const raw = Buffer.from(cursor, "base64url").toString("utf8");
    const parsed = JSON.parse(raw) as unknown;
    if (
      typeof parsed !== "object" ||
      parsed === null ||
      typeof (parsed as Record<string, unknown>).id !== "number" ||
      typeof (parsed as Record<string, unknown>).ts !== "string"
    ) {
      throw new Error("Invalid cursor shape");
    }
    return parsed as { id: number; ts: string };
  } catch {
    throw new Error("Invalid or malformed cursor");
  }
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
