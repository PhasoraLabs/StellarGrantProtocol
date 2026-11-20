import { describe, it, expect } from "vitest";
import { getContentSecurityPolicy, getSecurityHeaders } from "../next.config";

describe("Content Security Policy", () => {
  it("does not include unsafe-eval or unsafe-inline in script-src for production builds", () => {
    const csp = getContentSecurityPolicy(true);

    // Extract the script-src directive
    const match = csp.match(/script-src\s+([^;]+);/);
    expect(match).not.toBeNull();
    const scriptSrcDirective = match![1];

    expect(scriptSrcDirective).toBe("'self'");
    expect(scriptSrcDirective).not.toContain("unsafe-eval");
    expect(scriptSrcDirective).not.toContain("unsafe-inline");
  });

  it("includes unsafe-eval and unsafe-inline in development mode for Fast Refresh / HMR", () => {
    const csp = getContentSecurityPolicy(false);

    const match = csp.match(/script-src\s+([^;]+);/);
    expect(match).not.toBeNull();
    const scriptSrcDirective = match![1];

    expect(scriptSrcDirective).toContain("'unsafe-eval'");
    expect(scriptSrcDirective).toContain("'unsafe-inline'");
  });

  it("retains all essential security directives in production CSP", () => {
    const csp = getContentSecurityPolicy(true);

    expect(csp).toContain("default-src 'self'");
    expect(csp).toContain("object-src 'none'");
    expect(csp).toContain("base-uri 'self'");
    expect(csp).toContain("form-action 'self'");
    expect(csp).toContain("upgrade-insecure-requests");
  });

  it("getSecurityHeaders returns configured headers with production CSP", () => {
    const headers = getSecurityHeaders(true);
    const cspHeader = headers.find(
      (h) => h.key === "Content-Security-Policy" || h.key === "Content-Security-Policy-Report-Only",
    );
    expect(cspHeader).toBeDefined();
    expect(cspHeader!.value).toContain("script-src 'self';");
    expect(cspHeader!.value).not.toContain("script-src 'self' 'unsafe-eval'");
  });
});
