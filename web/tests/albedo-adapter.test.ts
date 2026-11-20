/**
 * AlbedoAdapter Tests — #1283
 *
 * Verifies the public key is never cached across signing flows and that a
 * signature from an account other than the authorized one is rejected.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { Account, Asset, BASE_FEE, Keypair, Networks, Operation, TransactionBuilder } from "@stellar/stellar-sdk";
import { AlbedoAdapter, verifySignedByExpectedAccount } from "../lib/wallets/AlbedoAdapter";

const PASSPHRASE = Networks.TESTNET;

function buildTx(source: Keypair) {
  return new TransactionBuilder(new Account(source.publicKey(), "0"), {
    fee: BASE_FEE,
    networkPassphrase: PASSPHRASE,
  })
    .addOperation(Operation.payment({ destination: Keypair.random().publicKey(), asset: Asset.native(), amount: "1" }))
    .setTimeout(30)
    .build();
}

function signedXdr(source: Keypair, signer: Keypair, unsigned = buildTx(source).toXDR()): string {
  const tx = TransactionBuilder.fromXDR(unsigned, PASSPHRASE);
  tx.sign(signer);
  return tx.toXDR();
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("verifySignedByExpectedAccount", () => {
  it("accepts a signature from the expected account", () => {
    const kp = Keypair.random();
    expect(() => verifySignedByExpectedAccount(signedXdr(kp, kp), PASSPHRASE, kp.publicKey())).not.toThrow();
  });

  it("rejects a signature from a different account", () => {
    const authorized = Keypair.random();
    const other = Keypair.random();
    expect(() =>
      verifySignedByExpectedAccount(signedXdr(authorized, other), PASSPHRASE, authorized.publicKey()),
    ).toThrow(/different account/);
  });
});

describe("AlbedoAdapter", () => {
  it("re-queries Albedo on every getPublicKey call", async () => {
    const a = Keypair.random().publicKey();
    const b = Keypair.random().publicKey();
    const publicKey = vi.fn().mockResolvedValueOnce({ pubkey: a }).mockResolvedValueOnce({ pubkey: b });
    vi.stubGlobal("window", { albedo: { publicKey, tx: vi.fn() } });

    const adapter = new AlbedoAdapter(PASSPHRASE);
    expect(await adapter.getPublicKey()).toBe(a);
    expect(await adapter.getPublicKey()).toBe(b);
    expect(publicKey).toHaveBeenCalledTimes(2);
  });

  it("rejects a signature from a different Albedo account than authorized", async () => {
    const authorized = Keypair.random();
    const other = Keypair.random();
    const original = buildTx(authorized);
    const tx = vi.fn().mockResolvedValue({ signed_envelope_xdr: signedXdr(authorized, other, original.toXDR()) });
    vi.stubGlobal("window", {
      albedo: { publicKey: vi.fn().mockResolvedValue({ pubkey: authorized.publicKey() }), tx },
    });

    const adapter = new AlbedoAdapter(PASSPHRASE);
    await adapter.getPublicKey();
    await expect(adapter.signTransaction(original.toXDR(), PASSPHRASE)).rejects.toThrow(/different account/);
    expect(tx).toHaveBeenCalledWith(expect.objectContaining({ pubkey: authorized.publicKey() }));
  });

  it("returns the signed envelope when the authorized account signs", async () => {
    const kp = Keypair.random();
    const unsigned = buildTx(kp).toXDR();
    const signed = signedXdr(kp, kp, unsigned);
    vi.stubGlobal("window", {
      albedo: {
        publicKey: vi.fn().mockResolvedValue({ pubkey: kp.publicKey() }),
        tx: vi.fn().mockResolvedValue({ signed_envelope_xdr: signed }),
      },
    });

    const adapter = new AlbedoAdapter(PASSPHRASE);
    await adapter.getPublicKey();
    await expect(adapter.signTransaction(unsigned, PASSPHRASE)).resolves.toBe(signed);
  });

  it("refuses to sign before an account has been authorized", async () => {
    vi.stubGlobal("window", { albedo: { publicKey: vi.fn(), tx: vi.fn() } });
    const adapter = new AlbedoAdapter(PASSPHRASE);
    await expect(adapter.signTransaction("xdr", PASSPHRASE)).rejects.toThrow(/No Albedo account authorized/);
  });
});
