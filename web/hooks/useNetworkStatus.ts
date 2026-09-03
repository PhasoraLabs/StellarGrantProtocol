"use client";

import { useState, useEffect } from "react";

interface NetworkStatus {
  isOnline: boolean;
  apiReachable: boolean; // can we reach our API?
  rpcReachable: boolean; // can we reach Stellar RPC?
  lastOnline: Date | null;
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

  return { isOnline, apiReachable, rpcReachable, lastOnline };
}
