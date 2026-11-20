import { afterEach, describe, expect, it } from "vitest";
import { validateEnv } from "../src/config/env-validation";

describe("environment validation", () => {
  const originalEnv = { ...process.env };

  afterEach(() => {
    process.env = { ...originalEnv };
  });

  it("reports every missing required variable", () => {
    process.env = { NODE_ENV: "test" };
    const result = validateEnv();

    expect(result.valid).toBe(false);
    expect(result.errors).toEqual(expect.arrayContaining([
      expect.stringContaining("JWT_SECRET"),
      expect.stringContaining("CONTRACT_ID"),
      expect.stringContaining("RPC_URL"),
      expect.stringContaining("NETWORK_PASSPHRASE"),
    ]));
  });

  it("rejects blank required values", () => {
    process.env = {
      NODE_ENV: "test",
      DATABASE_URL: "postgres://localhost/db",
      JWT_SECRET: " ",
      CONTRACT_ID: "contract",
      RPC_URL: "https://rpc.example",
      NETWORK_PASSPHRASE: "passphrase",
    };

    expect(validateEnv().errors).toEqual([
      expect.stringContaining("JWT_SECRET"),
    ]);
  });
});
