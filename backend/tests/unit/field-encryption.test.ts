import { describe, expect, it } from "vitest";
import { decryptField, encryptField } from "../../src/utils/field-encryption";

describe("field encryption", () => {
  it("round-trips unicode values", () => {
    const encrypted = encryptField("PII: café 🔐");
    expect(encrypted).not.toContain("café");
    expect(decryptField(encrypted)).toBe("PII: café 🔐");
  });
  it("does not return plaintext for tampered ciphertext", () => {
    const encrypted = encryptField("secret").replace(/.$/, "A");
    expect(decryptField(encrypted)).not.toBe("secret");
  });
});
