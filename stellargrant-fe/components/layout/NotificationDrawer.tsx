"use client";

import { useRouter } from "next/navigation";
import { motion, AnimatePresence } from "framer-motion";
import { Bell, CheckCircle, ChevronRight, Rocket, X } from "lucide-react";
import { useRelativeTime } from "@/hooks/useRelativeTime";
import type { Notification } from "@/hooks/useNotifications";

interface NotificationDrawerProps {
  open: boolean;
  onClose: () => void;
  notifications: Notification[];
  onMarkAllRead: () => void;
  onMarkRead: (id: string) => void;
  onClearAll: () => void;
}

function RowIcon({ type }: { type: string }) {
  switch (type) {
    case "milestone_approved":
    case "grant_updated":
    case "vote_recorded":
      return <CheckCircle size={16} className="text-success shrink-0" />;
    case "milestone_submitted":
      return <Rocket size={16} className="text-warning shrink-0" />;
    default:
      return <Bell size={16} className="text-accent-secondary shrink-0" />;
  }
}

function NotificationRow({
  item,
  onNavigate,
}: {
  item: Notification;
  onNavigate: (n: Notification) => void;
}) {
  const relative = useRelativeTime(item.timestamp);

  return (
    <button
      type="button"
      onClick={() => onNavigate(item)}
      className={[
        "w-full text-left px-4 py-3 flex gap-3 items-start border-b border-border-color hover:bg-bg-secondary/80 transition-colors",
        !item.read ? "border-l-2 border-l-accent-primary" : "border-l-2 border-l-transparent",
      ].join(" ")}
    >
      <RowIcon type={item.type} />
      <div className="flex-1 min-w-0">
        <p className="font-mono text-xs text-text-primary font-medium truncate">
          {item.title}
        </p>
        <p className="font-mono text-[11px] text-text-muted mt-0.5 line-clamp-2">
          {item.description}
        </p>
        <p className="font-mono text-[10px] text-text-muted mt-1">{relative}</p>
      </div>
      {item.grantId && (
        <ChevronRight size={14} className="text-text-muted shrink-0 mt-1" />
      )}
    </button>
  );
}

export function NotificationDrawer({
  open,
  onClose,
  notifications,
  onMarkAllRead,
  onMarkRead,
  onClearAll,
}: NotificationDrawerProps) {
  const router = useRouter();

  const handleNavigate = (n: Notification) => {
    onMarkRead(n.id);
    onClose();
    if (n.grantId) {
      if (n.milestoneIdx != null) {
        router.push(`/grants/${n.grantId}/milestones/${n.milestoneIdx}`);
      } else {
        router.push(`/grants/${n.grantId}`);
      }
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-40 bg-bg-primary/60"
            onClick={onClose}
            aria-hidden
          />
          <motion.aside
            initial={{ x: "100%" }}
            animate={{ x: 0 }}
            exit={{ x: "100%" }}
            transition={{ type: "tween", duration: 0.25, ease: "easeOut" }}
            className="fixed right-0 top-0 z-50 flex h-full w-full max-w-full flex-col border-l border-border-color bg-bg-secondary sm:w-96"
          >
            <header className="flex items-center justify-between gap-2 border-b border-border-color px-4 py-4">
              <h2 className="font-orbitron text-sm uppercase tracking-wider text-text-primary">
                Notifications
              </h2>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={onMarkAllRead}
                  className="font-mono text-[10px] uppercase tracking-wider text-accent-secondary hover:underline"
                >
                  Mark all as read
                </button>
                <button
                  type="button"
                  onClick={onClose}
                  className="text-text-muted hover:text-text-primary p-1"
                  aria-label="Close notifications"
                >
                  <X size={18} />
                </button>
              </div>
            </header>

            <div className="flex-1 overflow-y-auto">
              {notifications.length === 0 ? (
                <p className="px-4 py-12 text-center font-mono text-sm text-text-muted">
                  No notifications yet
                </p>
              ) : (
                notifications.map((n) => (
                  <NotificationRow key={n.id} item={n} onNavigate={handleNavigate} />
                ))
              )}
            </div>

            <footer className="border-t border-border-color p-4">
              <button
                type="button"
                onClick={onClearAll}
                className="w-full font-mono text-xs uppercase tracking-wider text-text-muted hover:text-danger transition-colors"
              >
                Clear all
              </button>
            </footer>
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  );
}
