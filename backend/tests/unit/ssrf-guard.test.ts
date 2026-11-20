import { describe, expect, it } from "vitest";
import { assertSafeWebhookUrl } from "../../src/utils/ssrf-guard";

describe("assertSafeWebhookUrl bypass vectors", () => {
  it("rejects IPv6 loopback addresses", async () => {
    await expect(
      assertSafeWebhookUrl("https://[::1]/hook"),
    ).rejects.toThrow(/private|loopback/i);
  });

  it("rejects IPv6 link-local addresses", async () => {
    await expect(
      assertSafeWebhookUrl("https://[fe80::1]/hook"),
    ).rejects.toThrow(/private|loopback/i);
  });

  it("rejects decimal-encoded loopback IPv4 addresses", async () => {
    const url = "https://2130706433/hook";
    expect(new URL(url).hostname).toBe("127.0.0.1");

    await expect(assertSafeWebhookUrl(url)).rejects.toThrow(
      /private|loopback/i,
    );
  });
});
