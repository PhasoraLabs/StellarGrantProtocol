/**
 * Shared formatter for error messages rendered by React error boundaries.
 *
 * An error boundary must never paint a raw, unbounded `error.message`:
 * severe exceptions can carry multi-kilobyte text (stack dumps, SQL, HTML
 * fragments) that leaks internals and breaks the layout. Both the nested
 * `app/error.tsx` and the root `app/global-error.tsx` boundaries route
 * through this helper so they apply identical rules:
 *
 *   - production  → generic fallback only; the boundary renders the
 *                   `digest` id separately (the digest-only surface Next.js
 *                   intends for production error pages);
 *   - development → the raw message, truncated to `maxLength` (200 chars).
 *
 * @module web/lib/errors/userFacingMessage
 */

/** Maximum length of a raw error message rendered inside a boundary. */
export const MAX_ERROR_MESSAGE_LENGTH = 200;

/** Fallback shown when the error carries no usable message. */
export const GENERIC_ERROR_MESSAGE = "An unexpected error occurred.";

interface UserFacingMessageOptions {
  /** Text to show when the message is missing/empty, or in production. */
  fallback?: string;
  /** Hard cap on the rendered message length. Defaults to 200. */
  maxLength?: number;
}

/**
 * Turn an error boundary's `error` into a safe, bounded display string.
 *
 * Truncation is explicit: a cut message ends with an ellipsis so readers
 * can tell text was omitted rather than the message ending there.
 */
export function toUserFacingErrorMessage(
  error: Pick<Error, "message"> | null | undefined,
  options: UserFacingMessageOptions = {},
): string {
  const {
    fallback = GENERIC_ERROR_MESSAGE,
    maxLength = MAX_ERROR_MESSAGE_LENGTH,
  } = options;

  // Production error surfaces show the digest (rendered by the boundary),
  // never the raw exception text.
  if (process.env.NODE_ENV === "production") {
    return fallback;
  }

  const message = error?.message?.trim();
  if (!message) return fallback;

  if (message.length <= maxLength) return message;
  return `${message.slice(0, maxLength)}…`;
}
