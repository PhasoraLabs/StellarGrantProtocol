/**
 * Tests for the three webhook security fixes:
 *
 *  Issue #27 — IDOR: spoofed x-user-address header must not access another
 *              user's webhook subscriptions.
 *  Issue #28 — SSRF: private/internal/metadata target URLs must be blocked.
 *  Issue #29 — Error handler: errorHandler is now wired in app.ts; raw
 *              internal error messages must not leak to clients in production.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { assertSafeWebhookUrl } from "../../src/utils/ssrf-guard";
import * as fs from "node:fs";
import * as path from "node:path";

// ---------------------------------------------------------------------------
// Issue #28 – SSRF guard (unit tests for assertSafeWebhookUrl)
// ---------------------------------------------------------------------------

describe("assertSafeWebhookUrl (Issue #28 – SSRF)", () => {
  it("accepts a valid public HTTPS URL", async () => {
    // Stub DNS so the test is self-contained.
    vi.mock("node:dns", () => ({
      promises: {
        lookup: vi
          .fn()
          .mockResolvedValue([{ address: "93.184.216.34", family: 4 }]),
      },
    }));
    await expect(
      assertSafeWebhookUrl("https://example.com/hook"),
    ).resolves.toBeUndefined();
  });

  it("rejects plain HTTP", async () => {
    await expect(
      assertSafeWebhookUrl("http://example.com/hook"),
    ).rejects.toThrow(/HTTPS/i);
  });

  it("rejects a localhost URL", async () => {
    // 127.0.0.1 is a bare IP — DNS lookup is skipped and it is blocked directly.
    await expect(
      assertSafeWebhookUrl("https://127.0.0.1/hook"),
    ).rejects.toThrow(/private|loopback/i);
  });

  it("rejects 0.0.0.0", async () => {
    await expect(assertSafeWebhookUrl("https://0.0.0.0/hook")).rejects.toThrow(
      /private|loopback/i,
    );
  });

  it("rejects the AWS metadata endpoint 169.254.169.254", async () => {
    await expect(
      assertSafeWebhookUrl("https://169.254.169.254/latest/meta-data/"),
    ).rejects.toThrow(/private|loopback/i);
  });

  it("rejects a private RFC-1918 address 192.168.1.1", async () => {
    await expect(
      assertSafeWebhookUrl("https://192.168.1.1/hook"),
    ).rejects.toThrow(/private|loopback/i);
  });

  it("rejects a private RFC-1918 address 10.0.0.1", async () => {
    await expect(assertSafeWebhookUrl("https://10.0.0.1/hook")).rejects.toThrow(
      /private|loopback/i,
    );
  });

  it("rejects 172.16.0.1 (RFC-1918)", async () => {
    await expect(
      assertSafeWebhookUrl("https://172.16.0.1/hook"),
    ).rejects.toThrow(/private|loopback/i);
  });

  it("rejects the IPv6 loopback ::1", async () => {
    await expect(assertSafeWebhookUrl("https://[::1]/hook")).rejects.toThrow(
      /private|loopback/i,
    );
  });

  it("rejects a hostname that resolves to an internal IP", async () => {
    // Override the module-level vi.mock so DNS returns an internal address.
    const { promises: dns } = await import("node:dns");
    vi.spyOn(dns, "lookup").mockResolvedValue([
      { address: "10.0.0.5", family: 4 },
    ] as any);

    await expect(
      assertSafeWebhookUrl("https://internal.corp/hook"),
    ).rejects.toThrow(/private|loopback/i);
  });

  it("rejects when DNS lookup fails (fail-closed)", async () => {
    const { promises: dns } = await import("node:dns");
    vi.spyOn(dns, "lookup").mockRejectedValue(new Error("NXDOMAIN"));

    await expect(
      assertSafeWebhookUrl("https://nonexistent.invalid/hook"),
    ).rejects.toThrow(/private|loopback/i);
  });
});

// ---------------------------------------------------------------------------
// Issue #27 – IDOR: getCallerAddress no longer reads spoofable headers
// ---------------------------------------------------------------------------

describe("Webhook IDOR fix (Issue #27)", () => {
  /**
   * The router now calls `router.use(signedUserAuth)` before any handler, so
   * `req.user.stellarAddress` is the *only* identity source.  We test the
   * helper in isolation by verifying that the old spoofable-header paths are
   * gone from the source.
   */
  it("getCallerAddress reads only req.user.stellarAddress, not x-user-address or x-admin-address", async () => {
    const { buildWebhooksRouter } = await import("../../src/routes/webhooks");

    // Inspect the stringified factory to ensure the fallback headers are gone.
    const src = buildWebhooksRouter.toString();
    expect(src).not.toContain("x-user-address");
    expect(src).not.toContain("x-admin-address");
    expect(src).toContain("req.user?.stellarAddress");
  });

  it("a request with only a spoofed x-user-address header gets 401, not the victim's data", async () => {
    // Build a minimal router with mocked dependencies.
    const { buildWebhooksRouter } = await import("../../src/routes/webhooks");
    const express = (await import("express")).default;
    const supertest = (await import("supertest")).default;

    const userRepo = {
      findOne: vi.fn(),
      create: vi.fn(),
      save: vi.fn(),
    } as any;
    const subscriptionRepo = { find: vi.fn(), findOne: vi.fn() } as any;
    const webhookDispatcher = {} as any;
    const rbacService = {
      getUserPermissions: vi.fn().mockResolvedValue([]),
      hasPermission: vi.fn().mockResolvedValue(false),
    } as any;

    // A signedUserAuth stub that always rejects — simulating a missing/invalid
    // signature (which is what a pure header-spoofing attack would produce).
    const strictAuth = (_req: any, res: any, _next: any) => {
      res.status(401).json({ error: "Signed authentication required" });
    };

    const app = express();
    app.use(express.json());
    app.use(
      "/webhooks",
      buildWebhooksRouter(
        userRepo,
        subscriptionRepo,
        webhookDispatcher,
        rbacService,
        strictAuth,
      ),
    );

    // Attacker sends *only* a spoofed header — no valid Stellar signature.
    const response = await supertest(app)
      .get("/webhooks")
      .set("x-user-address", "VICTIM_ADDRESS_GABCDEFG");

    expect(response.status).toBe(401);
    expect(subscriptionRepo.find).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Issue #29 – errorHandler is wired and redacts messages in production
// ---------------------------------------------------------------------------

describe("errorHandler wiring (Issue #29)", () => {
  it("app.ts imports errorHandler from middlewares/error-handler", () => {
    const appSrc = fs.readFileSync(
      path.resolve(__dirname, "../../src/app.ts"),
      "utf8",
    );
    expect(appSrc).toContain("errorHandler");
    expect(appSrc).toContain("middlewares/error-handler");
  });

  it("errorHandler returns a generic message and does NOT leak raw errors in production", async () => {
    const { errorHandler } =
      await import("../../src/middlewares/error-handler");
    const envModule = await import("../../src/config/env");

    const req = { headers: {}, path: "/test", method: "GET" } as any;
    const res = {
      status: vi.fn().mockReturnThis(),
      json: vi.fn(),
    } as any;
    const next = vi.fn();

    // Patch the already-imported env object to simulate production.
    // We cannot rely on process.env.NODE_ENV because env.nodeEnv is
    // evaluated at module load time and cached.
    const originalNodeEnv = envModule.env.nodeEnv;
    (envModule.env as any).nodeEnv = "production";

    const internalError = new Error(
      "SELECT * FROM users WHERE id = 1; -- SQL detail",
    );
    errorHandler(internalError, req, res, next);

    // Restore so other tests are not affected.
    (envModule.env as any).nodeEnv = originalNodeEnv;

    expect(res.status).toHaveBeenCalledWith(500);
    const jsonArg = res.json.mock.calls[0][0];
    expect(jsonArg.message).not.toContain("SQL");
    expect(jsonArg.message).toBe("Internal server error");
  });
});
