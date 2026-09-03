"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useSocket } from "@/hooks/useSocket";
import { useWalletStore } from "@/lib/store/walletStore";
import type { ToastEventDetail } from "@/components/ui/NotificationToast";

const STORAGE_KEY = "sg-notifications";
const MAX_NOTIFICATIONS = 50;

export interface Notification {
  id: string;
  type: string;
  title: string;
  description: string;
  grantId?: string;
  milestoneIdx?: number;
  txHash?: string;
  timestamp: Date;
  read: boolean;
}

export interface UseNotificationsResult {
  notifications: Notification[];
  unreadCount: number;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  clearAll: () => void;
}

type StoredNotification = Omit<Notification, "timestamp"> & { timestamp: string };

function loadStored(): Notification[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as StoredNotification[];
    return parsed.map((n) => ({ ...n, timestamp: new Date(n.timestamp) }));
  } catch {
    return [];
  }
}

function saveStored(list: Notification[]) {
  const payload: StoredNotification[] = list.map((n) => ({
    ...n,
    timestamp: n.timestamp.toISOString(),
  }));
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

function councilAddresses(): Set<string> {
  const raw = process.env.NEXT_PUBLIC_COUNCIL_ADDRESSES ?? "";
  return new Set(
    raw
      .split(",")
      .map((a) => a.trim())
      .filter(Boolean)
  );
}

function isRelevant(payload: Record<string, unknown>, wallet: string): boolean {
  if (councilAddresses().has(wallet)) return true;

  const actors = [
    payload.owner,
    payload.recipient,
    payload.grantOwner,
    payload.funder,
    payload.funderAddress,
    payload.actorAddress,
  ]
    .filter(Boolean)
    .map(String);

  if (actors.some((a) => a === wallet)) return true;

  const reviewers = Array.isArray(payload.reviewers)
    ? (payload.reviewers as string[])
    : [];
  if (reviewers.includes(wallet)) return true;

  return false;
}

function fromSocket(
  type: string,
  payload: Record<string, unknown>,
  timestamp: string
): Notification {
  const grantId = payload.grantId != null ? String(payload.grantId) : undefined;
  const milestoneIdx =
    payload.milestoneIdx != null ? Number(payload.milestoneIdx) : undefined;

  let title = "Notification";
  let description = "You have a new update.";

  switch (type) {
    case "grant_created":
      title = "New Grant Created";
      description = `Grant "${String(payload.title ?? "")}" was registered on-chain.`;
      break;
    case "grant_updated":
      title = "Grant Updated";
      description = `Grant "${String(payload.title ?? "")}" status changed.`;
      break;
    case "grant_funded":
      title = "Grant Funded";
      description = `Grant #${grantId ?? "?"} received funding.`;
      break;
    case "milestone_submitted":
      title = "Milestone Submitted";
      description = `Milestone ${(milestoneIdx ?? 0) + 1} on grant #${grantId ?? "?"}.`;
      break;
    case "milestone_approved":
      title = "Milestone Approved";
      description = `Milestone approved on grant #${grantId ?? "?"}.`;
      break;
    default:
      break;
  }

  return {
    id: `${type}-${timestamp}-${grantId ?? Date.now()}`,
    type,
    title,
    description,
    grantId,
    milestoneIdx,
    txHash: payload.txHash != null ? String(payload.txHash) : undefined,
    timestamp: new Date(timestamp),
    read: false,
  };
}

function fromToast(detail: ToastEventDetail): Notification {
  const grantMatch = detail.href?.match(/\/grants\/([^/]+)/);
  return {
    id: `toast-${detail.type}-${Date.now()}`,
    type: detail.type,
    title: detail.title,
    description: detail.message,
    grantId: grantMatch?.[1],
    timestamp: new Date(),
    read: false,
  };
}

export function useNotifications(): UseNotificationsResult {
  const { address } = useWalletStore();
  const { lastNotification } = useSocket();
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [hydratedFor, setHydratedFor] = useState<string | null>(null);

  if (address !== hydratedFor) {
    setHydratedFor(address);
    setNotifications(address ? loadStored() : []);
  }

  const updateList = useCallback((updater: (prev: Notification[]) => Notification[]) => {
    setNotifications((prev) => {
      const next = updater(prev).slice(0, MAX_NOTIFICATIONS);
      if (address) saveStored(next);
      return next;
    });
  }, [address]);

  useEffect(() => {
    if (!address || !lastNotification) return;
    if (!isRelevant(lastNotification.payload, address)) return;

    const item = fromSocket(
      lastNotification.type,
      lastNotification.payload,
      lastNotification.timestamp
    );
    const timer = setTimeout(() => {
      updateList((prev) => [item, ...prev.filter((n) => n.id !== item.id)]);
    }, 0);
    return () => clearTimeout(timer);
  }, [lastNotification, address, updateList]);

  useEffect(() => {
    if (!address) return;

    function onToast(e: Event) {
      const detail = (e as CustomEvent<ToastEventDetail>).detail;
      if (detail.type === "address_copied") return;
      const item = fromToast(detail);
      updateList((prev) => [item, ...prev]);
    }

    window.addEventListener("stellar:toast", onToast);
    return () => window.removeEventListener("stellar:toast", onToast);
  }, [address, updateList]);

  const unreadCount = useMemo(
    () => notifications.filter((n) => !n.read).length,
    [notifications]
  );

  const markAsRead = useCallback(
    (id: string) => {
      updateList((prev) =>
        prev.map((n) => (n.id === id ? { ...n, read: true } : n))
      );
    },
    [updateList]
  );

  const markAllAsRead = useCallback(() => {
    updateList((prev) => prev.map((n) => ({ ...n, read: true })));
  }, [updateList]);

  const clearAll = useCallback(() => {
    updateList(() => []);
  }, [updateList]);

  return {
    notifications,
    unreadCount,
    markAsRead,
    markAllAsRead,
    clearAll,
  };
}
