/**
 * Sanitization Utility Tests
 *
 * Tests for stripMarkdown, validateDescriptionLength, sanitizeHtml,
 * and inlineMarkdownToSafeHtml using the injectable sanitizer pattern.
 * No DOMPurify mocking required.
 */

import { describe, it, expect } from "vitest";
import {
  stripMarkdown,
  validateDescriptionLength,
  sanitizeHtml,
  inlineMarkdownToSafeHtml,
  markdownToSafeHtml,
} from "../lib/utils/sanitize";

// ── Shared test sanitizer ────────────────────────────────────────────────────
// A lightweight sanitizer injected in place of DOMPurify for tests.
// Strips <script> tags and dangerous event-handler attributes.
function testSanitizer(html: string): string {
  if (!html) return "";
  return html
    .replace(/<script[\s\S]*?>[\s\S]*?<\/script>/gi, "")
    .replace(/\s(onerror|onclick|onload|javascript)=[^\s>]*/gi, "")
    .replace(/href="javascript:[^"]*"/gi, 'href="#"');
}

// ── sanitizeHtml ─────────────────────────────────────────────────────────────
describe("sanitizeHtml", () => {
  it("returns empty string for empty input", () => {
    expect(sanitizeHtml("", testSanitizer)).toBe("");
  });

  it("strips script tags", () => {
    const dirty = '<p>Safe</p><script>alert("xss")</script>';
    const clean = sanitizeHtml(dirty, testSanitizer);
    expect(clean).not.toContain("<script>");
    expect(clean).toContain("<p>Safe</p>");
  });

  it("strips onerror event handlers", () => {
    const dirty = '<img src="x" onerror="alert(1)">';
    const clean = sanitizeHtml(dirty, testSanitizer);
    expect(clean).not.toContain("onerror");
  });

  it("strips onclick event handlers", () => {
    const dirty = '<a href="#" onclick="steal()">click</a>';
    const clean = sanitizeHtml(dirty, testSanitizer);
    expect(clean).not.toContain("onclick");
  });

  it("preserves safe HTML", () => {
    const safe = "<p>Hello <strong>world</strong></p>";
    expect(sanitizeHtml(safe, testSanitizer)).toBe(safe);
  });
});

// ── inlineMarkdownToSafeHtml ─────────────────────────────────────────────────
describe("inlineMarkdownToSafeHtml", () => {
  it("returns empty string for blank input", () => {
    expect(inlineMarkdownToSafeHtml("", testSanitizer)).toBe("");
  });

  it("returns empty string for whitespace-only input", () => {
    expect(inlineMarkdownToSafeHtml("   ", testSanitizer)).toBe("");
  });

  it("converts bold markdown to <strong> tag", () => {
    const result = inlineMarkdownToSafeHtml("**bold**", testSanitizer);
    expect(result).toContain("strong");
  });

  it("converts italic markdown to <em> tag", () => {
    const result = inlineMarkdownToSafeHtml("*italic*", testSanitizer);
    expect(result).toContain("em");
  });
});

// ── markdownToSafeHtml ───────────────────────────────────────────────────────
describe("markdownToSafeHtml", () => {
  it("returns empty string for empty input", async () => {
    expect(await markdownToSafeHtml("", testSanitizer)).toBe("");
  });

  it("converts a heading to an <h1> tag", async () => {
    const result = await markdownToSafeHtml("# Hello", testSanitizer);
    expect(result).toContain("<h1>");
    expect(result).toContain("Hello");
  });

  it("converts unordered list to <ul>/<li>", async () => {
    const result = await markdownToSafeHtml("- item one\n- item two", testSanitizer);
    expect(result).toContain("<ul>");
    expect(result).toContain("<li>");
  });

  it("strips injected script from markdown output", async () => {
    const malicious = '<script>alert("xss")</script>\n\nSome content';
    const result = await markdownToSafeHtml(malicious, testSanitizer);
    expect(result).not.toContain("<script>");
    expect(result).toContain("Some content");
  });
});

// ── stripMarkdown ────────────────────────────────────────────────────────────
describe("stripMarkdown", () => {
  it("strips headings", () => {
    expect(stripMarkdown("## Hello World")).toBe("Hello World");
  });

  it("strips bold syntax", () => {
    expect(stripMarkdown("This is **bold** text")).toBe("This is bold text");
  });

  it("strips italic syntax", () => {
    expect(stripMarkdown("This is *italic*")).toBe("This is italic");
  });

  it("strips links and keeps link text", () => {
    expect(stripMarkdown("[Click here](https://example.com)")).toBe("Click here");
  });

  it("strips images entirely", () => {
    expect(stripMarkdown("![alt text](https://example.com/img.png)")).toBe("");
  });

  it("strips inline code blocks", () => {
    expect(stripMarkdown("`const x = 1`")).toBe("");
  });

  it("returns empty string for empty input", () => {
    expect(stripMarkdown("")).toBe("");
  });

  it("collapses multiple blank lines into a single space", () => {
    expect(stripMarkdown("Hello\n\n\nWorld")).toBe("Hello World");
  });
});

// ── XSS audit: IPFS / user-supplied HTML ────────────────────────────────────
describe("sanitizeHtml — XSS audit", () => {
  it("strips javascript: href from anchor tags", () => {
    const dirty = '<a href="javascript:alert(1)">click</a>';
    const clean = sanitizeHtml(dirty, testSanitizer);
    expect(clean).not.toContain("javascript:");
  });

  it("strips onload attribute from img tags", () => {
    const dirty = '<img src="x" onload="steal()">';
    const clean = sanitizeHtml(dirty, testSanitizer);
    expect(clean).not.toContain("onload");
  });

  it("does not strip allowed safe tags", () => {
    const safe = "<p><strong>Bold</strong> and <em>italic</em></p>";
    expect(sanitizeHtml(safe, testSanitizer)).toContain("strong");
    expect(sanitizeHtml(safe, testSanitizer)).toContain("em");
  });

  it("handles nested script injection in markdown output", async () => {
    const payload = "**safe**<script>document.cookie</script>";
    const result = await markdownToSafeHtml(payload, testSanitizer);
    expect(result).not.toContain("<script>");
    expect(result).toContain("safe");
  });
});

// ── validateDescriptionLength ────────────────────────────────────────────────
describe("validateDescriptionLength", () => {
  it("passes a valid description", () => {
    const result = validateDescriptionLength("A valid grant description.");
    expect(result.valid).toBe(true);
    expect(result.message).toBeUndefined();
  });

  it("fails on empty string", () => {
    const result = validateDescriptionLength("");
    expect(result.valid).toBe(false);
    expect(result.message).toMatch(/empty/i);
  });

  it("fails on whitespace-only string", () => {
    expect(validateDescriptionLength("   ").valid).toBe(false);
  });

  it("fails when content exceeds maxLength", () => {
    const long = "a".repeat(5001);
    const result = validateDescriptionLength(long, 5000);
    expect(result.valid).toBe(false);
    expect(result.message).toMatch(/5000/);
  });

  it("passes when content equals maxLength exactly", () => {
    expect(validateDescriptionLength("a".repeat(5000), 5000).valid).toBe(true);
  });

  it("respects a custom maxLength parameter", () => {
    expect(validateDescriptionLength("hello world!", 10).valid).toBe(false);
  });
});
