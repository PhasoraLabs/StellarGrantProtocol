import { StellarGrantsSDK } from "../src/StellarGrantsSDK";
import type { WalletAdapter } from "../src/types";

/**
 * Example: Vote on a milestone (approve/reject).
 *
 * Notes:
 * - Replace the signer implementation with a real wallet integration.
 */
async function main() {
  const rpcUrl = process.env.RPC_URL;
  const networkPassphrase = process.env.NETWORK_PASSPHRASE;
  const contractId = process.env.CONTRACT_ID;

  if (!rpcUrl || !networkPassphrase || !contractId) {
    throw new Error("Missing env: RPC_URL, NETWORK_PASSPHRASE, CONTRACT_ID");
  }

  const signer: WalletAdapter = {
    async getPublicKey() {
      // TODO: return the wallet public key (G...)
      return "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
    },
    async signTransaction(txXdr: string) {
      // TODO: sign with a wallet and return signed XDR
      return txXdr;
    },
  };

  const sdk = new StellarGrantsSDK({
    rpcUrl,
    networkPassphrase,
    contractId,
    signer,
  });

  const result = await sdk.milestoneVote({
    grantId: 1,
    milestoneIdx: 0,
    approve: true,
  });

  console.log("milestoneVote result:", result);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

