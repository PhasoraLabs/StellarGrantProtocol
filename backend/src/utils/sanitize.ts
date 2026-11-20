/**
 * HTML/script sanitization helpers for user-supplied content (#1228).
 *
 * Milestone comment bodies are treated as plain text. Any markup is escaped so
 * that a stored comment can never inject HTML or execute script when rendered
 * by a client.
 */

const HTML_ESCAPE_MAP: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/**
 * Escape the HTML metacharacters in a value so it is safe to embed verbatim.
 */
export function escapeHtml(value: unknown): string {
  const s = value == null ? "" : String(value);
  return s.replace(/[&<>"']/g, (char) => HTML_ESCAPE_MAP[char] ?? char);
}

/**
 * Sanitize comment content before it is persisted.
 */
export function sanitizeComment(content: unknown): string {
  return escapeHtml(content);
}
