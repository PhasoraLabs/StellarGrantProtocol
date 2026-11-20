/**
 * Stellar RPC & Horizon Client Singletons
 *
 * Centralized network client configuration for Stellar interactions.
 * - Soroban RPC: contract invocations and ledger state
 * - Horizon: account balances, trustlines, and transaction history
 */

import { rpc, Horizon } from "@stellar/stellar-sdk";
import { STELLAR_RPC_URL, NETWORK_PASSPHRASE, HORIZON_URL } from "@/lib/constants";

const rpcUrl = STELLAR_RPC_URL;
const networkPassphrase = NETWORK_PASSPHRASE;
const horizonUrl = HORIZON_URL;

// Soroban RPC client singleton
export const rpcClient = new rpc.Server(rpcUrl, {
  allowHttp: rpcUrl.startsWith("http://"),
});

// Horizon client singleton (provides account balances and trustlines)
export const horizonClient = new Horizon.Server(horizonUrl, {
  allowHttp: horizonUrl.startsWith("http://"),
});

export const networkPassphraseConfig = networkPassphrase;

export function getRpcClient(): rpc.Server {
  return rpcClient;
}

export function getHorizonClient(): Horizon.Server {
  return horizonClient;
}
