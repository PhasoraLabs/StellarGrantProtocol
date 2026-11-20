import { describe, expect, it } from "vitest";
import { Account, Asset, BASE_FEE, Keypair, Operation, TransactionBuilder } from "@stellar/stellar-sdk";
import { networkPassphraseConfig } from "../lib/stellar/client";
import { verifySignedTransactionMatchesOriginal } from "../lib/wallets/FreighterAdapter";

describe("verifySignedTransactionMatchesOriginal", () => {
  it("rejects a signed envelope built for a different transaction", () => {
    const originalSource = Keypair.random().publicKey();
    const originalDest = Keypair.random().publicKey();
    const mismatchedSource = Keypair.random().publicKey();

    const originalTx = new TransactionBuilder(new Account(originalSource, "0"), {
      fee: BASE_FEE.toString(),
      networkPassphrase: networkPassphraseConfig,
    })
      .addOperation(
        Operation.payment({
          destination: originalDest,
          asset: Asset.native(),
          amount: "10",
        })
      )
      .setTimeout(30)
      .build();

    const mismatchedTx = new TransactionBuilder(new Account(mismatchedSource, "0"), {
      fee: BASE_FEE.toString(),
      networkPassphrase: networkPassphraseConfig,
    })
      .addOperation(
        Operation.payment({
          destination: originalDest,
          asset: Asset.native(),
          amount: "42",
        })
      )
      .setTimeout(30)
      .build();

    expect(() =>
      verifySignedTransactionMatchesOriginal(
        originalTx.toEnvelope().toXDR("base64"),
        mismatchedTx.toEnvelope().toXDR("base64"),
        networkPassphraseConfig,
      )
    ).toThrow(/does not match|mismatch|different transaction/i);
  });
});
