"use client";

import { useCallback, useState } from "react";
import { motion, useReducedMotion } from "framer-motion";
import type { ToastEventDetail } from "@/components/ui/NotificationToast";
import { useWatchlist } from "@/hooks/useWatchlist";

interface WatchButtonProps {
  grantId: string;
}

export function WatchButton({ grantId }: WatchButtonProps) {
  const { isWatched, toggle } = useWatchlist();
  const watched = isWatched(grantId);
  const prefersReduced = useReducedMotion();
  const [pulseKey, setPulseKey] = useState(0);

  const handleClick = useCallback(() => {
    const wasWatched = watched;
    toggle(grantId);
    setPulseKey((key) => key + 1);

    if (!wasWatched) {
      window.dispatchEvent(
        new CustomEvent<ToastEventDetail>("stellar:toast", {
          detail: {
            type: "watchlist_added",
            title: "Grant added to watchlist",
            message: "You can find it under Dashboard → Watching.",
          },
        }),
      );
    }
  }, [grantId, toggle, watched]);

  return (
    <motion.button
      key={pulseKey}
      type="button"
      onClick={handleClick}
      title={watched ? "Stop watching this grant" : "Watch this grant"}
      initial={prefersReduced ? false : { scale: 1 }}
      animate={prefersReduced ? {} : { scale: [1, 1.06, 1] }}
      transition={{ duration: 0.35, ease: "easeOut" }}
      className={[
        "inline-flex items-center gap-1.5 rounded-none border px-3 py-1.5 font-mono text-xs uppercase tracking-wider transition-colors",
        watched
          ? "border-accent-primary/40 bg-accent-primary/20 text-accent-primary"
          : "border-border-color bg-transparent text-text-muted hover:border-accent-primary/50 hover:text-text-primary",
      ].join(" ")}
    >
      <span aria-hidden="true">{watched ? "★" : "☆"}</span>
      {watched ? "Watching" : "Watch Grant"}
    </motion.button>
  );
}
