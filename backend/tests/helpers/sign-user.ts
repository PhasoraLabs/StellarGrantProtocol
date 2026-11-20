import { Keypair } from "@stellar/stellar-sdk";
import { randomUUID } from "node:crypto";
import { SignatureService } from "../../src/services/signature-service";

const signatureService = new SignatureService();

/** Builds the x-stellar-* headers proving `keypair` controls its address for METHOD:path. */
export const signedUserHeaders = (
  keypair: Keypair,
  method: string,
  path: string,
  overrides: { nonce?: string; timestamp?: number } = {},
) => {
  const address = keypair.publicKey();
  const nonce = overrides.nonce ?? randomUUID();
  const timestamp = overrides.timestamp ?? Date.now();
  const message = signatureService.buildUserIntentMessage({
    address,
    nonce,
    timestamp,
    action: `${method.toUpperCase()}:${path}`,
  });
  return {
    "x-stellar-address": address,
    "x-stellar-nonce": nonce,
    "x-stellar-timestamp": String(timestamp),
    "x-stellar-signature": keypair.sign(Buffer.from(message, "utf8")).toString("base64"),
  };
};
