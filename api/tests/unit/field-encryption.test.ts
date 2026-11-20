import { describe, expect, it } from "vitest";
import { decryptField, encryptField } from "../../src/utils/field-encryption";

describe("field encryption", () => {
  it("round-trips unicode values", () => {
    const encrypted = encryptField("PII: café 🔐", "test-key");
    expect(encrypted).not.toContain("café");
    expect(decryptField(encrypted, "test-key")).toBe("PII: café 🔐");
  });
  it("rejects tampered ciphertext", () => {
    const encrypted = encryptField("secret", "test-key").replace(/.$/, "A");
    expect(() => decryptField(encrypted, "test-key")).toThrow();
  });
});
