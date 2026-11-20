"use client";

/**
 * useWallet Hook
 *
 * Adapter-agnostic wallet state hook. Delegates all signing and key retrieval
 * to the active WalletAdapter stored in walletStore, so switching wallet
 * providers requires changing only the `connect(type)` call.
 *
 * Supported types: "freighter" | "albedo"
 * Coming soon:     "xbull" | "passkey"
 */

import { useState, useEffect } from "react";
import { useWalletStore, type WalletType } from "@/lib/store/walletStore";
import { networkPassphraseConfig } from "@/lib/stellar/client";

export type SupportedWalletType = "freighter" | "albedo" | "xbull" | "passkey";

export interface WalletState {
  address: string | null;
  isConnected: boolean;
  isConnecting: boolean;
  network: "testnet" | "mainnet" | "futurenet";
  walletType: WalletType;
  connect: (type: SupportedWalletType) => Promise<void>;
  disconnect: () => void;
  signTransaction: (xdr: string) => Promise<string>;
  error: string | null;
}

export function useWallet(): WalletState {
  const {
    address,
    network,
    walletType,
    activeAdapter,
    setAddress,
    setNetwork,
    setWalletType,
    setActiveAdapter,
    reset,
  } = useWalletStore();

  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ── Session restore on mount ─────────────────────────────────────────────────
  // Re-validate the persisted session. Freighter can be re-checked via the API;
  // Albedo has no persistent session so we clear the address if it was persisted.
  useEffect(() => {
    if (typeof window === "undefined") return;

    async function restoreSession() {
      try {
        if (walletType === "freighter") {
          const { isConnected, getAddress, getNetworkDetails } = await import(
            "@stellar/freighter-api"
          );
          const connected = await isConnected();
          if (connected) {
            const addressResult = await getAddress();
            if (addressResult.error) {
              console.debug("Failed to restore Freighter session:", addressResult.error);
              reset();
              return;
            }
            const networkResult = await getNetworkDetails();
            setAddress(addressResult.address);
            setNetwork(networkResult.network as "testnet" | "mainnet" | "futurenet");

            // Re-instantiate the adapter so signing works after a page reload
            const { FreighterAdapter } = await import("@stellar/freighter-api").then(
              () => import("@/lib/wallets/FreighterAdapter"),
            );
            setActiveAdapter(new FreighterAdapter());
          } else {
            reset();
          }
        } else if (walletType === "albedo") {
          // Albedo has no persistent session — clear the stale address
          reset();
        }
      } catch (err) {
        console.debug("Session restore failed:", err);
      }
    }

    restoreSession();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // run once on mount only

  // ── connect ──────────────────────────────────────────────────────────────────

  const connect = async (type: SupportedWalletType): Promise<void> => {
    setIsConnecting(true);
    setError(null);

    try {
      if (type === "freighter") {
        const { FreighterAdapter } = await import("@/lib/wallets/FreighterAdapter");
        const adapter = new FreighterAdapter();
        const pubkey = await adapter.getPublicKey();

        const { getNetworkDetails } = await import("@stellar/freighter-api");
        const networkResult = await getNetworkDetails();

        setAddress(pubkey);
        setNetwork(networkResult.network as "testnet" | "mainnet" | "futurenet");
        setWalletType("freighter");
        setActiveAdapter(adapter);
      } else if (type === "albedo") {
        const { AlbedoAdapter } = await import("@/lib/wallets/AlbedoAdapter");
        const adapter = new AlbedoAdapter(networkPassphraseConfig);
        // getPublicKey triggers the Albedo popup
        const pubkey = await adapter.getPublicKey();

        setAddress(pubkey);
        setWalletType("albedo");
        setActiveAdapter(adapter);
      } else {
        throw new Error(`${type} wallet is not supported yet`);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to connect wallet";
      setError(message);
      console.error("Wallet connection error:", err);
    } finally {
      setIsConnecting(false);
    }
  };

  // ── disconnect ───────────────────────────────────────────────────────────────

  const disconnect = (): void => {
    // Call optional adapter teardown (e.g. WalletConnect session close)
    void activeAdapter?.disconnect?.();
    reset();
    setError(null);
  };

  // ── signTransaction ──────────────────────────────────────────────────────────

  const sign = async (xdr: string): Promise<string> => {
    if (typeof window === "undefined") {
      throw new Error("Signing is only available in the browser");
    }

    if (!activeAdapter) {
      throw new Error("No wallet connected. Call connect() first.");
    }

    try {
      return await activeAdapter.signTransaction(xdr, networkPassphraseConfig);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to sign transaction";
      setError(message);
      throw new Error(message);
    }
  };

  return {
    address,
    isConnected: !!address,
    isConnecting,
    network,
    walletType,
    connect,
    disconnect,
    signTransaction: sign,
    error,
  };
}
