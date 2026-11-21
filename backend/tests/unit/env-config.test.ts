import { afterEach, describe, expect, it, vi } from "vitest";

describe("environment config", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it("exposes runtime settings through the typed config", async () => {
    vi.stubEnv("SENDGRID_API_KEY", "sendgrid-test-key");
    vi.stubEnv("SENDGRID_FROM_EMAIL", "grants@example.test");
    vi.stubEnv("FRONTEND_URL", "https://grants.example.test");
    vi.stubEnv("USE_MOCK_SOROBAN", "true");
    vi.resetModules();

    const { env } = await import("../../src/config/env");

    expect(env.sendgridApiKey).toBe("sendgrid-test-key");
    expect(env.sendgridFromEmail).toBe("grants@example.test");
    expect(env.frontendUrl).toBe("https://grants.example.test");
    expect(env.useMockSoroban).toBe(true);
  });

  it("keeps the existing defaults for optional runtime settings", async () => {
    vi.stubEnv("FRONTEND_URL", "");
    vi.stubEnv("SENDGRID_FROM_EMAIL", "");
    vi.stubEnv("USE_MOCK_SOROBAN", "false");
    vi.resetModules();

    const { env } = await import("../../src/config/env");

    expect(env.frontendUrl).toBe("http://localhost:3000");
    expect(env.sendgridFromEmail).toBe("");
    expect(env.useMockSoroban).toBe(false);
  });
});
