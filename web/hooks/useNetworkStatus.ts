"use client";

import { useState, useEffect } from "react";
import { STELLAR_NETWORK } from "@/lib/constants";

interface NetworkStatus {
  isOnline: boolean;
  apiReachable: boolean; // can we reach our API?
  rpcReachable: boolean; // can we reach Stellar RPC?
  lastOnline: Date | null;
  walletNetwork: "testnet" | "mainnet" | "futurenet" | null; // wallet's current network
  appNetwork: "testnet" | "mainnet"; // app's configured network
  isNetworkMismatch: boolean; // true if wallet is on a different network than the app
}

export function useNetworkStatus(): NetworkStatus {
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== "undefined" ? navigator.onLine : true
  );
  const [apiReachable, setApiReachable] = useState(true);
  const [rpcReachable, setRpcReachable] = useState(true);
  const [lastOnline, setLastOnline] = useState<Date | null>(
    typeof navigator !== "undefined" && navigator.onLine ? new Date() : null
  );
  const [walletNetwork, setWalletNetwork] = useState<"testnet" | "mainnet" | "futurenet" | null>(null);

  useEffect(() => {
    if (typeof window === "undefined") return;

    const handleOnline = () => {
      setIsOnline(true);
      setLastOnline(new Date());
    };
    const handleOffline = () => {
      setIsOnline(false);
    };

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  // API reachability check (every 30s when offline or periodically)
  useEffect(() => {
    const checkApi = async () => {
      try {
        // Use a lightweight endpoint for reachability check
        const res = await fetch("/api/stats", { 
          signal: AbortSignal.timeout(5000),
          cache: 'no-store'
        });
        setApiReachable(res.ok);
      } catch {
        setApiReachable(false);
      }
    };

    const checkRpc = async () => {
      try {
        // Horizon Testnet status check
        const res = await fetch("https://horizon-testnet.stellar.org", {
          signal: AbortSignal.timeout(5000),
          cache: 'no-store'
        });
        setRpcReachable(res.ok);
      } catch {
        setRpcReachable(false);
      }
    };

    // Initial check
    void checkApi();
    void checkRpc();

    const interval = setInterval(() => {
      void checkApi();
      void checkRpc();
    }, 30000);

    return () => clearInterval(interval);
  }, []);

  // Detect wallet network mismatch
  useEffect(() => {
    if (typeof window === "undefined") return;

    const detectWalletNetwork = async () => {
      try {
        // Try Freighter first (most common)
        const freighter = (window as any).stellar;
        if (freighter?.freighter) {
          const { getNetworkDetails } = await import("@stellar/freighter-api");
          const networkDetails = await getNetworkDetails();
          setWalletNetwork(networkDetails.network as "testnet" | "mainnet" | "futurenet");
          return;
        }

        // Try to detect from wallet store if Freighter didn't work
        const walletStore = localStorage.getItem("wallet-store");
        if (walletStore) {
          try {
            const store = JSON.parse(walletStore);
            if (store.state?.network) {
              setWalletNetwork(store.state.network);
            }
          } catch {
            // ignore parse errors
          }
        }
      } catch (err) {
        // wallet detection failed, continue gracefully
      }
    };

    detectWalletNetwork();
  }, []);

  const isNetworkMismatch =
    walletNetwork !== null &&
    walletNetwork !== STELLAR_NETWORK &&
    !(walletNetwork === "futurenet"); // futurenet doesn't mismatch with testnet in some cases

  return {
    isOnline,
    apiReachable,
    rpcReachable,
    lastOnline,
    walletNetwork,
    appNetwork: STELLAR_NETWORK,
    isNetworkMismatch,
  };
}
