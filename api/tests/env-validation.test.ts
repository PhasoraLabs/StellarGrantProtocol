import { describe, expect, it } from "vitest";
import { parseEnv } from "../src/config/env";

describe("environment validation", () => {
  it("uses safe defaults when optional variables are absent", () => {
    expect(parseEnv({}).port).toBe(4000);
  });
  it("rejects invalid ports and empty database URLs", () => {
    expect(() => parseEnv({ PORT: "0" })).toThrow("PORT");
    expect(() => parseEnv({ DATABASE_URL: " " })).toThrow("DATABASE_URL");
  });
});
