import { describe, expect, it } from "vitest";
import {
  escapeHtml,
  sanitizeComment,
} from "../../src/utils/sanitize";

describe("sanitizeComment", () => {
  it("escapes script tags", () => {
    expect(sanitizeComment("<script>alert(1)</script>")).toBe(
      "&lt;script&gt;alert(1)&lt;/script&gt;",
    );
  });

  it("escapes event-handler attributes", () => {
    expect(sanitizeComment('<img src=x onerror="alert(1)">')).toBe(
      "&lt;img src=x onerror=&quot;alert(1)&quot;&gt;",
    );
  });

  it("escapes nested markup", () => {
    expect(
      sanitizeComment(
        '<div><span>comment</span><img src=x onerror="alert(1)"></div>',
      ),
    ).toBe(
      "&lt;div&gt;&lt;span&gt;comment&lt;/span&gt;&lt;img src=x onerror=&quot;alert(1)&quot;&gt;&lt;/div&gt;",
    );
  });
});

describe("escapeHtml", () => {
  it("escapes all HTML metacharacters", () => {
    expect(escapeHtml(`&<>"'`)).toBe("&amp;&lt;&gt;&quot;&#39;");
  });

  it("converts nullish input to an empty string", () => {
    expect(escapeHtml(null)).toBe("");
    expect(escapeHtml(undefined)).toBe("");
  });
});
