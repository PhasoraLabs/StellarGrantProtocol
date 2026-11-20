import { createCipheriv, createDecipheriv, createHash, randomBytes } from "node:crypto";

const algorithm = "aes-256-gcm";
const keyFrom = (key: string | Buffer) => createHash("sha256").update(key).digest();

export function encryptField(value: string, key: string | Buffer): string {
  const iv = randomBytes(12);
  const cipher = createCipheriv(algorithm, keyFrom(key), iv);
  const ciphertext = Buffer.concat([cipher.update(value, "utf8"), cipher.final()]);
  return [iv, cipher.getAuthTag(), ciphertext].map((part) => part.toString("base64url")).join(".");
}

export function decryptField(encoded: string, key: string | Buffer): string {
  const [ivText, tagText, ciphertextText] = encoded.split(".");
  if (!ivText || !tagText || !ciphertextText) throw new Error("Invalid encrypted field");
  const decipher = createDecipheriv(algorithm, keyFrom(key), Buffer.from(ivText, "base64url"));
  decipher.setAuthTag(Buffer.from(tagText, "base64url"));
  return Buffer.concat([decipher.update(Buffer.from(ciphertextText, "base64url")), decipher.final()]).toString("utf8");
}
