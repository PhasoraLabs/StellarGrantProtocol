import { describe, expect, it, vi, beforeEach } from "vitest";
import { escapeHtml, getEmailTemplate } from "../../src/services/email-service";
import { NonceService, NonceReplayError } from "../../src/services/nonce-service";

describe("escapeHtml", () => {
  it("escapes HTML metacharacters", () => {
    expect(escapeHtml(`<img src=x onerror="alert('1')">`)).toBe(
      "&lt;img src=x onerror=&quot;alert(&#39;1&#39;)&quot;&gt;",
    );
  });
});

describe("email templates (#1237)", () => {
  it("renders crafted grant titles as literal text", () => {
    const { html, subject } = getEmailTemplate("milestone_approved", {
      grantTitle: `<script>alert(1)</script>`,
      milestoneTitle: `</b><img src=x onerror=alert(2)>`,
    });
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;script&gt;");
    expect(subject).not.toContain("<script>");
  });
});

describe("NonceService (#1239)", () => {
  const insert = vi.fn();
  const repo = { insert } as any;

  beforeEach(() => {
    insert.mockReset();
  });

  it("persists a fresh nonce", async () => {
    insert.mockResolvedValue({});
    await new NonceService(repo).consume("nonce-1");
    expect(insert).toHaveBeenCalledOnce();
  });

  it("rejects a replayed nonce independently of business rules", async () => {
    insert.mockRejectedValue({ code: "23505" });
    await expect(new NonceService(repo).consume("nonce-1")).rejects.toBeInstanceOf(
      NonceReplayError,
    );
  });
});
