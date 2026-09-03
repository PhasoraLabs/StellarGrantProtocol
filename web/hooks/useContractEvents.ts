"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { API_URL } from "@/lib/constants";

export interface ContractEvent {
  type:
    | "GrantFunded"
    | "MilestoneSubmitted"
    | "MilestoneApproved"
    | "MilestoneRejected"
    | "PayoutReleased"
    | string;
  data: Record<string, unknown>;
  ledger: number;
  timestamp: Date;
}

export type ConnectionStatus =
  "connecting" | "connected" | "disconnected" | "error";

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
}

export function useContractEvents({
  grantId,
}: UseContractEventsOptions = {}): UseContractEventsResult {
  const [events, setEvents] = useState<ContractEvent[]>([]);
  const [latestEvent, setLatestEvent] = useState<ContractEvent | null>(null);
  const [connectionStatus, setConnectionStatus] =
    useState<ConnectionStatus>("disconnected");
  const [error, setError] = useState<Error | null>(null);

  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const grantIdRef = useRef(grantId);
  const prevDataRef = useRef<string>("");

  useEffect(() => {
    grantIdRef.current = grantId;
  }, [grantId]);

  const clearEvents = useCallback(() => {
    setEvents([]);
    setLatestEvent(null);
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;
    if (!grantId) return;

    let active = true;

    async function poll() {
      if (!active) return;
      setConnectionStatus("connecting");
      try {
        const res = await fetch(`${API_URL}/grants/${grantId}`, {
          cache: "no-store",
        });
        if (!res.ok) throw new Error(`Poll failed: ${res.statusText}`);
        const data = await res.json();
        const dataStr = JSON.stringify(data);

        if (dataStr !== prevDataRef.current) {
          prevDataRef.current = dataStr;
          const event: ContractEvent = {
            type: "MilestoneApproved",
            data: { grant_id: grantId },
            ledger: 0,
            timestamp: new Date(),
          };
          setEvents((prev) => [...prev.slice(-99), event]);
          setLatestEvent(event);
        }

        setConnectionStatus("connected");
        setError(null);
      } catch (err) {
        if (!active) return;
        setConnectionStatus("error");
        setError(err instanceof Error ? err : new Error("Poll failed"));
      }
    }

    poll();
    const pollTimer = setInterval(poll, 10_000);

    return () => {
      active = false;
      clearInterval(pollTimer);
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
      }
      setConnectionStatus("disconnected");
    };
  }, [grantId]);

  return {
    events,
    latestEvent,
    isConnected: connectionStatus === "connected",
    connectionStatus,
    error,
    clearEvents,
  };
}
