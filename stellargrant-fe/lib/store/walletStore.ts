/**
 * Wallet Store (Zustand)
 *
 * Global state management for wallet connection and network settings.
 *
 * `activeAdapter` holds the live WalletAdapter instance in memory.
 * It is intentionally excluded from localStorage persistence because class
 * instances cannot be serialised — only `address`, `walletType`, and `network`
 * are persisted so the UI can restore the connected state label on reload.
 */

import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { WalletAdapter } from "@/lib/wallets/types";

export type WalletType = "freighter" | "albedo" | "xbull" | "walletconnect" | "passkey" | null;
type Network = "testnet" | "mainnet" | "futurenet";

interface WalletStore {
  address: string | null;
  walletType: WalletType;
  network: Network;
  /** In-memory only — not persisted to localStorage. */
  activeAdapter: WalletAdapter | null;
  setAddress: (address: string | null) => void;
  setWalletType: (type: WalletType) => void;
  setNetwork: (network: Network) => void;
  setActiveAdapter: (adapter: WalletAdapter | null) => void;
  reset: () => void;
}

export const useWalletStore = create<WalletStore>()(
  persist(
    (set) => ({
      address: null,
      walletType: null,
      network: "testnet",
      activeAdapter: null,
      setAddress: (address) => set({ address }),
      setWalletType: (walletType) => set({ walletType }),
      setNetwork: (network) => set({ network }),
      setActiveAdapter: (activeAdapter) => set({ activeAdapter }),
      reset: () => set({ address: null, walletType: null, network: "testnet", activeAdapter: null }),
    }),
    {
      name: "stellar-grants-wallet",
      // Exclude activeAdapter — class instances cannot be serialised
      partialize: (state) => ({
        address: state.address,
        walletType: state.walletType,
        network: state.network,
      }),
    },
  ),
);
