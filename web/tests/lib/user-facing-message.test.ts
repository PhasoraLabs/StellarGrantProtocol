import { describe, it, expect, vi, afterEach } from "vitest";
import {
  toUserFacingErrorMessage,
  MAX_ERROR_MESSAGE_LENGTH,
  GENERIC_ERROR_MESSAGE,
} from "@/lib/errors/userFacingMessage";

describe("toUserFacingErrorMessage", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("returns a short message verbatim", () => {
    expect(toUserFacingErrorMessage(new Error("boom"))).toBe("boom");
  });

  it("never renders an unbounded message — truncates at 200 chars", () => {
    const error = new Error("x".repeat(5_000));
    const message = toUserFacingErrorMessage(error);

    expect(message.length).toBe(MAX_ERROR_MESSAGE_LENGTH + 1);
    expect(message).toBe(`${"x".repeat(MAX_ERROR_MESSAGE_LENGTH)}…`);
  });

  it("honours an explicit maxLength", () => {
    const message = toUserFacingErrorMessage(new Error("abcdefghij"), { maxLength: 4 });
    expect(message).toBe("abcd…");
  });

  it("falls back when the error has no message", () => {
    expect(toUserFacingErrorMessage(null)).toBe(GENERIC_ERROR_MESSAGE);
    expect(toUserFacingErrorMessage(new Error(""))).toBe(GENERIC_ERROR_MESSAGE);
    expect(toUserFacingErrorMessage(new Error("   "))).toBe(GENERIC_ERROR_MESSAGE);
  });

  it("uses a custom fallback when provided", () => {
    const message = toUserFacingErrorMessage(new Error(""), {
      fallback: "The application encountered an unrecoverable error.",
    });
    expect(message).toBe("The application encountered an unrecoverable error.");
  });

  it("shows only the fallback in production (digest-only surface)", () => {
    vi.stubEnv("NODE_ENV", "production");

    expect(toUserFacingErrorMessage(new Error("sensitive internals"))).toBe(
      GENERIC_ERROR_MESSAGE,
    );
    expect(
      toUserFacingErrorMessage(new Error("x".repeat(5_000))).length,
    ).toBeLessThanOrEqual(MAX_ERROR_MESSAGE_LENGTH);
  });
});
