"use client";

import React, { useEffect, useRef, useState, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useSocket } from "@/hooks/useSocket";
import {
  Bell,
  CheckCircle,
  Info,
  Rocket,
  X,
  AlertCircle,
  AlertTriangle,
} from "lucide-react";
import Link from "next/link";
import { TOAST_EVENT, type ToastOptions, type InternalToast } from "@/lib/toast";

export interface ToastEventDetail {
  type: string;
  title: string;
  message: string;
  href?: string;
}

interface ToastIconProps {
  type: string;
  variant: "success" | "error" | "warning" | "info";
}

function ToastIcon({ type, variant }: ToastIconProps) {
  if (type.startsWith("grant_")) {
    switch (type) {
      case "grant_created":
        return <Info className="text-blue-400" size={20} />;
      case "grant_updated":
        return <CheckCircle className="text-green-400" size={20} />;
      case "milestone_submitted":
        return <Rocket className="text-orange-400" size={20} />;
      case "vote_recorded":
        return <CheckCircle className="text-green-400" size={20} />;
      case "vote_error":
        return <AlertCircle className="text-red-400" size={20} />;
      default:
        return <Bell className="text-purple-400" size={20} />;
    }
  }
  switch (variant) {
    case "success":
      return <CheckCircle className="text-green-400" size={20} />;
    case "error":
      return <XCircleIcon className="text-red-400" size={20} />;
    case "warning":
      return <AlertTriangle className="text-yellow-400" size={20} />;
    case "info":
      return <Info className="text-blue-400" size={20} />;
    default:
      return <Bell className="text-purple-400" size={20} />;
  }
}

function XCircleIcon({ className, size }: { className?: string; size?: number }) {
  return (
    <svg
      width={size ?? 20}
      height={size ?? 20}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <circle cx="12" cy="12" r="10" />
      <line x1="15" y1="9" x2="9" y2="15" />
      <line x1="9" y1="9" x2="15" y2="15" />
    </svg>
  );
}

interface Notification {
  type: string;
  payload: Record<string, unknown>;
  timestamp: string;
}

function toastDetailFromNotification(n: Notification): ToastEventDetail {
  const { type, payload } = n;
  switch (type) {
    case "grant_created":
      return {
        type,
        title: "New Grant Created",
        message: `Grant "${payload.title}" has been successfully registered on-chain.`,
      };
    case "grant_updated":
      return {
        type,
        title: "Grant Updated",
        message: `Grant "${payload.title}" status changed from ${payload.oldStatus} to ${payload.newStatus}.`,
      };
    case "milestone_submitted":
      return {
        type,
        title: "Milestone Submitted",
        message: `A new milestone proof has been submitted for Grant #${payload.grantId} (Milestone ${Number(payload.milestoneIdx) + 1}).`,
      };
    default:
      return { type, title: "Notification", message: "You have a new update." };
  }
}

const variantStyles: Record<string, { border: string; bg: string }> = {
  success: { border: "border-success/40", bg: "bg-success/10" },
  error: { border: "border-danger/40", bg: "bg-danger/10" },
  warning: { border: "border-warning/40", bg: "bg-warning/10" },
  info: { border: "border-accent-secondary/40", bg: "bg-accent-secondary/10" },
};

const MAX_TOASTS = 3;

export const NotificationToast: React.FC = () => {
  const { lastNotification } = useSocket();
  const [toasts, setToasts] = useState<InternalToast[]>([]);
  const timersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(
    new Map()
  );

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
    const timer = timersRef.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timersRef.current.delete(id);
    }
  }, []);

  const addToast = useCallback(
    (options: ToastOptions) => {
      const id = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
      const toast: InternalToast = {
        id,
        title: options.title,
        description: options.description ?? "",
        variant: options.variant ?? "info",
        duration: options.duration ?? 5000,
        action: options.action,
      };

      setToasts((prev) => {
        const next = [...prev, toast];
        return next.length > MAX_TOASTS
          ? next.slice(next.length - MAX_TOASTS)
          : next;
      });

      const timer = setTimeout(() => removeToast(id), toast.duration);
      timersRef.current.set(id, timer);
    },
    [removeToast]
  );

  useEffect(() => {
    if (!lastNotification) return;
    const detail = toastDetailFromNotification(lastNotification);
    addToast({
      title: detail.title,
      description: detail.message,
      variant: "info",
      action: detail.href ? { label: "View", href: detail.href } : undefined,
    });
  }, [lastNotification, addToast]);

  useEffect(() => {
    function handleCustom(e: Event) {
      const detail = (e as CustomEvent<ToastOptions>).detail;
      addToast(detail);
    }

    window.addEventListener(TOAST_EVENT, handleCustom);
    return () => window.removeEventListener(TOAST_EVENT, handleCustom);
  }, [addToast]);

  useEffect(() => {
    function handleLegacy(e: Event) {
      const detail = (e as CustomEvent<ToastEventDetail>).detail;
      addToast({
        title: detail.title,
        description: detail.message,
        variant:
          detail.type === "address_copied"
            ? "success"
            : detail.type === "success"
              ? "success"
              : detail.type === "error"
                ? "error"
                : "info",
        action: detail.href
          ? { label: "View on Explorer", href: detail.href }
          : undefined,
        duration: detail.type === "address_copied" ? 2000 : 5000,
      });
    }

    window.addEventListener("stellar:toast", handleLegacy);
    return () => window.removeEventListener("stellar:toast", handleLegacy);
  }, [addToast]);

  // Cleanup all timers on unmount
  useEffect(() => {
    return () => {
      timersRef.current.forEach((timer) => clearTimeout(timer));
    };
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed top-4 right-4 z-[9999] flex flex-col gap-2 max-w-sm w-full pointer-events-none">
      <AnimatePresence initial={false}>
        {toasts.map((t) => {
          const styles = variantStyles[t.variant] ?? variantStyles.info;
          return (
            <motion.div
              key={t.id}
              initial={{ opacity: 0, x: 120 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 120, transition: { duration: 0.2 } }}
              className="pointer-events-auto"
            >
              <div
                className={`bg-surface border ${styles.border} ${styles.bg} p-4 flex items-start gap-3`}
              >
                <div className="shrink-0 mt-0.5">
                  <ToastIcon type="" variant={t.variant} />
                </div>

                <div className="flex-1 min-w-0">
                  <h4 className="text-sm font-bold text-text-primary">
                    {t.title}
                  </h4>
                  {t.description && (
                    <p className="text-xs text-text-muted mt-1 leading-relaxed">
                      {t.description}
                    </p>
                  )}
                  {t.action && (
                    <div className="mt-2">
                      {t.action.href ? (
                        <Link
                          href={t.action.href}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-xs text-accent-secondary hover:underline font-mono"
                        >
                          {t.action.label} →
                        </Link>
                      ) : (
                        <button
                          type="button"
                          onClick={t.action.onClick}
                          className="text-xs text-accent-secondary hover:underline font-mono"
                        >
                          {t.action.label}
                        </button>
                      )}
                    </div>
                  )}
                </div>

                <button
                  type="button"
                  onClick={() => removeToast(t.id)}
                  className="text-text-muted hover:text-text-primary transition-colors shrink-0"
                  aria-label="Dismiss"
                >
                  <X size={16} />
                </button>
              </div>
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}
