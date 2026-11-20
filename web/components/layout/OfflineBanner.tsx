"use client";

import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useNetworkStatus } from "@/hooks/useNetworkStatus";
import { WifiOff, CheckCircle } from "lucide-react";

export function OfflineBanner() {
  const { isOnline } = useNetworkStatus();
  const [showRestored, setShowRestored] = useState(false);
  const [wasOffline, setWasOffline] = useState(false);

  useEffect(() => {
    if (!isOnline) {
      queueMicrotask(() => setWasOffline(true));
    } else if (isOnline && wasOffline) {
      queueMicrotask(() => setShowRestored(true));
      const timer = setTimeout(() => {
        setShowRestored(false);
        setWasOffline(false);
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [isOnline, wasOffline]);

  return (
    <AnimatePresence>
      {!isOnline && (
        <motion.div
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          className="bg-warning/20 border-b border-warning/40 px-4 py-1.5 flex items-center justify-center gap-2 overflow-hidden"
        >
          <WifiOff className="h-3 w-3 text-warning" />
          <span className="font-mono text-[10px] uppercase tracking-widest text-warning">
            You&apos;re offline — some features may not be available
          </span>
        </motion.div>
      )}

      {showRestored && (
        <motion.div
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          className="bg-success/20 border-b border-success/40 px-4 py-1.5 flex items-center justify-center gap-2 overflow-hidden"
        >
          <CheckCircle className="h-3 w-3 text-success" />
          <span className="font-mono text-[10px] uppercase tracking-widest text-success">
            Connection restored
          </span>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
