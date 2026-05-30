"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { toast } from "@/lib/toast";

export interface ContractEvent {
  type:
    | "GrantFunded"
    | "MilestoneSubmitted"
    | "MilestoneApproved"
    | "PayoutReleased"
    | string;
  grantId?: string;
  recipientAddress?: string;
  reviewerAddress?: string;
  data: Record<string, unknown>;
  ledger: number;
  timestamp: Date;
}

export type ConnectionStatus =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";

export interface UseContractEventsResult {
  events: ContractEvent[];
  latestEvent: ContractEvent | null;
  isConnected: boolean;
  connectionStatus: ConnectionStatus;
  error: Error | null;
  clearEvents: () => void;
}

interface UseContractEventsOptions {
  grantId?: string;
  walletAddress?: string;
}

export function useContractEvents({
  grantId,
  walletAddress,
}: UseContractEventsOptions = {}): UseContractEventsResult {
  const [events, setEvents] = useState<ContractEvent[]>([]);
  const [latestEvent, setLatestEvent] = useState<ContractEvent | null>(null);
  const [connectionStatus, setConnectionStatus] =
    useState<ConnectionStatus>("disconnected");
  const [error, setError] = useState<Error | null>(null);

  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Refs let event handlers read the latest values without stale closures.
  const grantIdRef = useRef(grantId);
  const walletAddressRef = useRef(walletAddress);

  useEffect(() => {
    grantIdRef.current = grantId;
  }, [grantId]);

  useEffect(() => {
    walletAddressRef.current = walletAddress;
  }, [walletAddress]);

  const clearEvents = useCallback(() => {
    setEvents([]);
    setLatestEvent(null);
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;

    // Fallback: poll when EventSource is unavailable (old browsers).
    if (typeof EventSource === "undefined") {
      setConnectionStatus("connected");
      const pollInterval = setInterval(() => {
        const currentGrantId = grantIdRef.current;
        if (!currentGrantId) return;
        fetch(`/api/grants/${encodeURIComponent(currentGrantId)}`, {
          cache: "no-store",
        }).catch(() => {
          // polling failure — silently continue
        });
      }, 15000);
      return () => clearInterval(pollInterval);
    }

    let active = true;

    function fireToastIfRelevant(event: ContractEvent) {
      const currentGrantId = grantIdRef.current;
      const currentWallet = walletAddressRef.current;

      const isRelevantGrant =
        !currentGrantId || event.grantId === currentGrantId;
      const isRecipient =
        currentWallet && event.recipientAddress === currentWallet;
      const isReviewer =
        currentWallet && event.reviewerAddress === currentWallet;

      if (!isRelevantGrant && !isRecipient && !isReviewer) return;

      if (event.type === "GrantFunded") {
        toast({ title: "Someone funded a grant you're watching", variant: "info" });
      } else if (event.type === "MilestoneSubmitted" && isReviewer) {
        toast({ title: "Milestone submitted — vote now", variant: "info" });
      } else if (event.type === "MilestoneApproved" && isRecipient) {
        toast({ title: "Milestone approved!", variant: "success" });
      } else if (event.type === "PayoutReleased") {
        toast({ title: "Payout released to contributor", variant: "success" });
      }
    }

    function openConnection() {
      const currentGrantId = grantIdRef.current;
      const url = currentGrantId
        ? `/api/events?grantId=${encodeURIComponent(currentGrantId)}`
        : "/api/events";

      const source = new EventSource(url);
      setConnectionStatus("connecting");

      source.onopen = () => {
        if (!active) {
          source.close();
          return;
        }
        setConnectionStatus("connected");
        setError(null);
      };

      source.onmessage = (e: MessageEvent) => {
        if (!active) return;
        try {
          const event = JSON.parse(e.data as string) as ContractEvent;
          setEvents((prev) => [...prev.slice(-99), event]);
          setLatestEvent(event);
          fireToastIfRelevant(event);
        } catch {
          // malformed event — ignore
        }
      };

      source.addEventListener("ping", () => {
        // keepalive — do nothing
      });

      source.onerror = () => {
        if (!active) return;
        setConnectionStatus("error");
        setError(new Error("SSE connection lost"));
        source.close();
        reconnectTimerRef.current = setTimeout(() => {
          if (active) openConnection();
        }, 5000);
      };

      return source;
    }

    const source = openConnection();

    return () => {
      active = false;
      source.close();
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      setConnectionStatus("disconnected");
    };
  }, [grantId]); // re-connect when the watched grant changes

  return {
    events,
    latestEvent,
    isConnected: connectionStatus === "connected",
    connectionStatus,
    error,
    clearEvents,
  };
}
