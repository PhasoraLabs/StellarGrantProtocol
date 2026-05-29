"use client";

import { useEffect, useRef, useState } from "react";
import { Bell } from "lucide-react";
import { useWalletStore } from "@/lib/store/walletStore";
import { useNotifications } from "@/hooks/useNotifications";
import { NotificationDrawer } from "./NotificationDrawer";

export function NotificationBell() {
  const { address } = useWalletStore();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [pulse, setPulse] = useState(false);
  const {
    notifications,
    unreadCount,
    markAsRead,
    markAllAsRead,
    clearAll,
  } = useNotifications();

  const prevUnreadRef = useRef(unreadCount);

  useEffect(() => {
    if (unreadCount > prevUnreadRef.current && unreadCount > 0) {
      const startTimer = setTimeout(() => setPulse(true), 0);
      const stopTimer = setTimeout(() => setPulse(false), 3000);
      prevUnreadRef.current = unreadCount;
      return () => {
        clearTimeout(startTimer);
        clearTimeout(stopTimer);
      };
    }
    prevUnreadRef.current = unreadCount;
  }, [unreadCount]);

  if (!address) return null;

  const badgeLabel = unreadCount > 9 ? "9+" : String(unreadCount);

  return (
    <>
      <button
        type="button"
        onClick={() => setDrawerOpen((o) => !o)}
        className="relative p-2 text-text-muted hover:text-accent-primary transition-colors"
        aria-label={`Notifications${unreadCount ? `, ${unreadCount} unread` : ""}`}
      >
        <Bell size={20} />
        {unreadCount > 0 && (
          <span
            className={[
              "absolute -top-0.5 -right-0.5 min-w-[18px] h-[18px] flex items-center justify-center rounded-full bg-accent-primary text-bg-primary font-mono text-[10px] font-bold px-1",
              pulse ? "animate-pulse" : "",
            ].join(" ")}
          >
            {badgeLabel}
          </span>
        )}
      </button>

      <NotificationDrawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        notifications={notifications}
        onMarkAllRead={markAllAsRead}
        onMarkRead={markAsRead}
        onClearAll={clearAll}
      />
    </>
  );
}
