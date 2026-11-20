import { Repository } from "typeorm";
import { UsedNonce } from "../entities/UsedNonce";

/** How long a consumed nonce stays reserved (well past the 5-minute skew window). */
const NONCE_TTL_MS = 24 * 60 * 60 * 1000;

/**
 * Persist and reject replayed signature nonces (#1239).
 * Call `consume` after signature verification succeeds and before accepting
 * the business write. Throws if the nonce was already used.
 */
export class NonceService {
  constructor(private readonly nonceRepo: Repository<UsedNonce>) {}

  async consume(nonce: string): Promise<void> {
    const expiresAt = new Date(Date.now() + NONCE_TTL_MS);
    try {
      await this.nonceRepo.insert({ nonce, expiresAt });
    } catch (err: any) {
      if (err?.code === "23505" || err?.code === "SQLITE_CONSTRAINT" || /UNIQUE/i.test(String(err?.message))) {
        throw new NonceReplayError(nonce);
      }
      throw err;
    }
  }
}

export class NonceReplayError extends Error {
  constructor(public readonly nonce: string) {
    super("Signature nonce has already been used");
    this.name = "NonceReplayError";
  }
}
