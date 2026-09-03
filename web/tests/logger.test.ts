/**
 * Logger tests (#271)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { Logger } from "@/lib/logger";

describe("Logger", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("logs at or above the configured minimum level", () => {
    const log = new Logger({ level: "info" });
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});

    log.warn("hello warn");
    log.debug("should be silent");

    expect(warnSpy).toHaveBeenCalledOnce();
    expect(debugSpy).not.toHaveBeenCalled();
  });

  it("is silent below the minimum level", () => {
    const log = new Logger({ level: "error" });
    const infoSpy = vi.spyOn(console, "info").mockImplementation(() => {});
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    log.info("ignored");
    log.warn("ignored");

    expect(infoSpy).not.toHaveBeenCalled();
    expect(warnSpy).not.toHaveBeenCalled();
  });

  it("masks sensitive keys in context", () => {
    const log = new Logger({ level: "debug" });
    const spy = vi.spyOn(console, "debug").mockImplementation(() => {});

    log.debug("tx", { privateKey: "super-secret", amount: "100" });

    const [, context] = spy.mock.calls[0];
    expect((context as Record<string, unknown>).privateKey).toBe("***");
    expect((context as Record<string, unknown>).amount).toBe("100");
  });

  it("includes the configured prefix in output", () => {
    const log = new Logger({ level: "info", prefix: "[MyApp]" });
    const spy = vi.spyOn(console, "info").mockImplementation(() => {});

    log.info("test message");

    expect(spy.mock.calls[0][0]).toContain("[MyApp]");
  });

  it("child logger inherits parent level and extends prefix", () => {
    const parent = new Logger({ level: "debug", prefix: "[Root]" });
    const child = parent.child("Child");
    const spy = vi.spyOn(console, "debug").mockImplementation(() => {});

    child.debug("child message");

    expect(spy.mock.calls[0][0]).toContain("[Root]:Child");
  });

  it("setLevel changes the minimum level at runtime", () => {
    const log = new Logger({ level: "error" });
    const spy = vi.spyOn(console, "info").mockImplementation(() => {});

    log.info("before change — should be silent");
    expect(spy).not.toHaveBeenCalled();

    log.setLevel("info");
    log.info("after change — should log");
    expect(spy).toHaveBeenCalledOnce();
  });
});
