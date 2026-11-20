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

function toBigInt(value: unknown): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "number" && Number.isFinite(value)) return BigInt(Math.trunc(value));
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) return 0n;
    return BigInt(trimmed);
  }
  return 0n;
}

function normalizeStatus(value: unknown): string {
  return String(value ?? "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "");
}

function buildEventFromSnapshot(
  prevData: unknown,
  nextData: unknown,
  grantId: string,
): ContractEvent | null {
  const prevGrant = (prevData as { grant?: Record<string, unknown> } | null)?.grant ?? (prevData as Record<string, unknown> | null);
  const nextGrant = (nextData as { grant?: Record<string, unknown> } | null)?.grant ?? (nextData as Record<string, unknown> | null);

  if (!prevGrant || !nextGrant) return null;

  const prevFunded = toBigInt(prevGrant.funded);
  const nextFunded = toBigInt(nextGrant.funded);
  if (nextFunded > prevFunded) {
    return {
      type: "GrantFunded",
      data: {
        grant_id: grantId,
        amount: (nextFunded - prevFunded).toString(),
        previous_funded: prevFunded.toString(),
        new_funded: nextFunded.toString(),
      },
      ledger: Date.now(),
      timestamp: new Date(),
    };
  }

  const prevMilestones = Array.isArray((prevData as { milestones?: unknown[] } | null)?.milestones)
    ? (prevData as { milestones: unknown[] }).milestones
    : [];
  const nextMilestones = Array.isArray((nextData as { milestones?: unknown[] } | null)?.milestones)
    ? (nextData as { milestones: unknown[] }).milestones
    : [];

  const maxLength = Math.max(prevMilestones.length, nextMilestones.length);
  for (let index = 0; index < maxLength; index += 1) {
    const prevMilestone = prevMilestones[index] as Record<string, unknown> | undefined;
    const nextMilestone = nextMilestones[index] as Record<string, unknown> | undefined;

    const prevStatus = normalizeStatus(prevMilestone?.status ?? prevMilestone?.state ?? prevMilestone?.approval_status);
    const nextStatus = normalizeStatus(nextMilestone?.status ?? nextMilestone?.state ?? nextMilestone?.approval_status);

    if (prevStatus === nextStatus) continue;

    if (nextStatus.includes("approved")) {
      return {
        type: "MilestoneApproved",
        data: { grant_id: grantId, milestone_index: index },
        ledger: Date.now(),
        timestamp: new Date(),
      };
    }

    if (nextStatus.includes("rejected")) {
      return {
        type: "MilestoneRejected",
        data: { grant_id: grantId, milestone_index: index },
        ledger: Date.now(),
        timestamp: new Date(),
      };
    }

    if (nextStatus.includes("submitted") || nextStatus.includes("pendingreview") || nextStatus.includes("inreview")) {
      return {
        type: "MilestoneSubmitted",
        data: { grant_id: grantId, milestone_index: index },
        ledger: Date.now(),
        timestamp: new Date(),
      };
    }

    if (nextStatus.includes("payout") || nextStatus.includes("released")) {
      return {
        type: "PayoutReleased",
        data: { grant_id: grantId, milestone_index: index },
        ledger: Date.now(),
        timestamp: new Date(),
      };
    }
  }

  return null;
}

export function useContractEvents({
  grantId,
}: UseContractEventsOptions = {}): UseContractEventsResult {
  const [events, setEvents] = useState<ContractEvent[]>([]);
  const [latestEvent, setLatestEvent] = useState<ContractEvent | null>(null);
  const [connectionStatus, setConnectionStatus] =
    useState<ConnectionStatus>("disconnected");
  const [error, setError] = useState<Error | null>(null);

  const grantIdRef = useRef(grantId);
  const prevDataRef = useRef<string>("");
  const eventSourceRef = useRef<EventSource | null>(null);

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
    let pollTimer: ReturnType<typeof setInterval> | null = null;

    const pushEvent = (event: ContractEvent) => {
      if (!active) return;
      setEvents((prev) => [...prev.slice(-99), event]);
      setLatestEvent(event);
    };

    const poll = async () => {
      if (!active) return;
      setConnectionStatus("connecting");
      try {
        const res = await fetch(`${API_URL}/grants/${grantId}`, {
          cache: "no-store",
        });
        if (!res.ok) throw new Error(`Poll failed: ${res.statusText}`);

        const data = await res.json();
        const dataStr = JSON.stringify(data);

        if (dataStr !== prevDataRef.current && prevDataRef.current) {
          const event = buildEventFromSnapshot(
            JSON.parse(prevDataRef.current),
            data,
            grantId,
          );
          if (event) pushEvent(event);
        }

        prevDataRef.current = dataStr;
        setConnectionStatus("connected");
        setError(null);
      } catch (err) {
        if (!active) return;
        setConnectionStatus("error");
        setError(err instanceof Error ? err : new Error("Poll failed"));
      }
    };

    const connectSse = () => {
      if (typeof EventSource === "undefined") {
        void poll();
        pollTimer = setInterval(() => {
          void poll();
        }, 10_000);
        return;
      }

      try {
        const source = new EventSource(`/api/events?grantId=${grantId}`);
        eventSourceRef.current = source;

        source.addEventListener("contract_event", (event) => {
          try {
            const payload = JSON.parse((event as MessageEvent).data ?? "{}");
            if (payload?.type) pushEvent(payload as ContractEvent);
          } catch {
            setConnectionStatus("error");
          }
        });

        source.onopen = () => {
          setConnectionStatus("connected");
          setError(null);
        };

        source.onerror = () => {
          if (active) {
            setConnectionStatus("error");
            source.close();
            void poll();
            pollTimer = setInterval(() => {
              void poll();
            }, 10_000);
          }
        };
      } catch (err) {
        setConnectionStatus("error");
        setError(err instanceof Error ? err : new Error("SSE setup failed"));
        void poll();
        pollTimer = setInterval(() => {
          void poll();
        }, 10_000);
      }
    };

    void poll();
    connectSse();

    return () => {
      active = false;
      if (pollTimer) clearInterval(pollTimer);
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
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
