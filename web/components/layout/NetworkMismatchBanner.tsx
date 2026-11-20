"use client";

import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useNetworkStatus } from "@/hooks/useNetworkStatus";
import { AlertTriangle } from "lucide-react";

export function NetworkMismatchBanner() {
  const { isNetworkMismatch, walletNetwork, appNetwork } = useNetworkStatus();
  const [showBanner, setShowBanner] = useState(false);

  useEffect(() => {
    if (isNetworkMismatch) {
      queueMicrotask(() => setShowBanner(true));
    } else {
      setShowBanner(false);
    }
  }, [isNetworkMismatch]);

  const getNetworkName = (network: string | null): string => {
    switch (network) {
      case "testnet":
        return "Testnet";
      case "mainnet":
        return "Mainnet";
      case "futurenet":
        return "Futurenet";
      default:
        return "Unknown";
    }
  };

  return (
    <AnimatePresence>
      {showBanner && (
        <motion.div
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          className="bg-error/20 border-b border-error/40 px-4 py-2 flex items-center justify-center gap-2 overflow-hidden"
        >
          <AlertTriangle className="h-4 w-4 text-error flex-shrink-0" />
          <span className="font-mono text-xs uppercase tracking-widest text-error">
            Network mismatch — your wallet is on {getNetworkName(walletNetwork)} but
            this app is configured for {getNetworkName(appNetwork)}
          </span>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
