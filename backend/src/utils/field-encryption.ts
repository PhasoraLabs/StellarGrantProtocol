import { createCipheriv, createDecipheriv, createHash, randomBytes } from "node:crypto";
import { env } from "../config/env";

/**
 * Transparent at-rest encryption for sensitive columns (#1229).
 *
 * Values are encrypted with AES-256-GCM. The stored representation is
 * `enc:v1:<iv>:<authTag>:<ciphertext>` (all base64) so it round-trips through a
 * plain varchar column. Plaintext values written before encryption was
 * introduced are returned unchanged by `decryptField`, which keeps historical
 * rows readable.
 */

const PREFIX = "enc:v1:";
const ALGORITHM = "aes-256-gcm";
const IV_LENGTH = 12;

const FALLBACK_KEY = "stellargrant-dev-field-encryption-key";

let warnedAboutFallback = false;

function getKey(): Buffer {
  const secret = env.fieldEncryptionKey || env.jwtSecret || FALLBACK_KEY;
  if (!env.fieldEncryptionKey && !env.jwtSecret && !warnedAboutFallback) {
    warnedAboutFallback = true;
    if (env.nodeEnv === "production") {
      console.warn(
        "FIELD_ENCRYPTION_KEY is not set; falling back to an insecure default. Set FIELD_ENCRYPTION_KEY in production.",
      );
    }
  }
  return createHash("sha256").update(secret).digest();
}

export function encryptField(plaintext: string): string {
  if (plaintext == null) return plaintext;
  if (plaintext.startsWith(PREFIX)) return plaintext;

  const iv = randomBytes(IV_LENGTH);
  const cipher = createCipheriv(ALGORITHM, getKey(), iv);
  const encrypted = Buffer.concat([cipher.update(plaintext, "utf8"), cipher.final()]);
  const authTag = cipher.getAuthTag();

  return `${PREFIX}${iv.toString("base64")}:${authTag.toString("base64")}:${encrypted.toString("base64")}`;
}

export function decryptField(value: string): string {
  if (value == null) return value;
  if (!value.startsWith(PREFIX)) return value;

  const [, , ivB64, authTagB64, dataB64] = value.split(":");
  if (!ivB64 || !authTagB64 || !dataB64) {
    return value;
  }

  try {
    const decipher = createDecipheriv(ALGORITHM, getKey(), Buffer.from(ivB64, "base64"));
    decipher.setAuthTag(Buffer.from(authTagB64, "base64"));
    const decrypted = Buffer.concat([
      decipher.update(Buffer.from(dataB64, "base64")),
      decipher.final(),
    ]);
    return decrypted.toString("utf8");
  } catch {
    return value;
  }
}

export const encryptedFieldTransformer = {
  to: (value: string): string => encryptField(value),
  from: (value: string): string => decryptField(value),
};
