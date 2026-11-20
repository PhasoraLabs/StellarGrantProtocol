/**
 * Rich Media Sanitization Utility
 *
 * Converts Markdown to HTML and sanitizes the result using DOMPurify
 * to prevent XSS attacks while preserving safe formatting tags.
 *
 * Server-side safe: DOMPurify is only invoked in browser environments.
 * Testable: an optional sanitizerFn parameter can be injected in tests.
 */

import { marked } from "marked";

// Allowed HTML tags for grant/milestone descriptions
export const ALLOWED_TAGS = [
  "h1", "h2", "h3", "h4", "h5", "h6",
  "p", "br", "hr",
  "strong", "em", "s", "del", "code", "pre", "blockquote",
  "ul", "ol", "li",
  "a",
  "img",
  "table", "thead", "tbody", "tr", "th", "td",
];

// Allowed HTML attributes (blocked: onerror, onload, onclick, etc.)
export const ALLOWED_ATTR = [
  "href", "title", "target", "rel",
  "src", "alt", "width", "height",
  "class",
];

const DOMPURIFY_CONFIG = {
  ALLOWED_TAGS,
  ALLOWED_ATTR,
  ADD_ATTR: ["target"],
  FORBID_ATTR: ["style"],
  KEEP_CONTENT: true,
  FORCE_BODY: false,
};

/**
 * Resolve DOMPurify from the browser environment.
 * Returns null when running in SSR/Node (no window).
 */
function resolveDOMPurify(): { sanitize: (html: string, cfg?: object) => string } | null {
  if (typeof window === "undefined") return null;
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const mod = require("dompurify");
    const purify = mod?.default ?? mod;
    if (typeof purify?.sanitize === "function") return purify;
  } catch {
    console.warn("[sanitize] DOMPurify unavailable; HTML will not be sanitized.");
  }
  return null;
}

/**
 * Sanitize raw HTML.
 *
 * - In production (browser): uses DOMPurify.
 * - In SSR/Node: returns empty string (rendered client-side).
 * - With `sanitizerFn`: uses the injected function (for testing).
 */
export function sanitizeHtml(
  dirtyHtml: string,
  sanitizerFn?: (html: string) => string
): string {
  if (!dirtyHtml) return "";

  // 1. Injected sanitizer (unit tests)
  if (sanitizerFn) return sanitizerFn(dirtyHtml);

  // 2. DOMPurify in browser
  const purify = resolveDOMPurify();
  if (purify) return purify.sanitize(dirtyHtml, DOMPURIFY_CONFIG);

  // 3. SSR fallback
  return "";
}

/**
 * Convert Markdown to sanitized HTML (async).
 * Use for grant/milestone descriptions.
 *
 * @param markdown - Raw user markdown
 * @param sanitizerFn - Optional injected sanitizer (for tests)
 */
export async function markdownToSafeHtml(
  markdown: string,
  sanitizerFn?: (html: string) => string
): Promise<string> {
  if (!markdown || markdown.trim() === "") return "";
  marked.setOptions({ async: false, gfm: true, breaks: true });
  const rawHtml = await marked.parse(markdown);
  return sanitizeHtml(rawHtml, sanitizerFn);
}

/**
 * Convert inline Markdown to sanitized HTML (sync).
 * Use for single-line content like milestone titles.
 *
 * @param markdown - Raw user markdown
 * @param sanitizerFn - Optional injected sanitizer (for tests)
 */
export function inlineMarkdownToSafeHtml(
  markdown: string,
  sanitizerFn?: (html: string) => string
): string {
  if (!markdown || markdown.trim() === "") return "";
  const rawHtml = marked.parseInline(markdown) as string;
  return sanitizeHtml(rawHtml, sanitizerFn);
}

/**
 * Strip all Markdown syntax, returning plain text.
 * Useful for meta descriptions and search indices.
 */
export function stripMarkdown(markdown: string): string {
  if (!markdown) return "";
  return markdown
    .replace(/#{1,6}\s/g, "")
    .replace(/(\*\*|__)(.*?)\1/g, "$2")
    .replace(/(\*|_)(.*?)\1/g, "$2")
    .replace(/`{1,3}[^`]*`{1,3}/g, "")
    .replace(/!\[.*?\]\(.*?\)/g, "")
    .replace(/\[([^\]]+)\]\(.*?\)/g, "$1")
    .replace(/^[-*+]\s/gm, "")
    .replace(/^\d+\.\s/gm, "")
    .replace(/^>\s/gm, "")
    .replace(/\n{2,}/g, " ")
    .trim();
}

/**
 * Validate content length for grant/milestone descriptions.
 *
 * @param content - Raw markdown string
 * @param maxLength - Character limit (default: 5000)
 */
export function validateDescriptionLength(
  content: string,
  maxLength = 5000
): { valid: boolean; message?: string } {
  if (!content || content.trim() === "") {
    return { valid: false, message: "Description cannot be empty." };
  }
  if (content.length > maxLength) {
    return {
      valid: false,
      message: `Description exceeds maximum length of ${maxLength} characters (currently ${content.length}).`,
    };
  }
  return { valid: true };
}
