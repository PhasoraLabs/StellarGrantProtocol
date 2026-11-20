"use client";

import React, { createContext, useContext, useEffect, useState, useRef } from "react";
import io from "socket.io-client";
import { useWalletStore } from "@/lib/store/walletStore";

interface Notification {
  type: string;
  payload: Record<string, unknown>;
  timestamp: string;
}

// Minimal interface to avoid 'any' and complex type resolution issues
interface SocketInstance {
  on: (event: string, callback: (data: Notification | unknown) => void) => void;
  disconnect: () => void;
  io?: { on: (event: string, callback: (data: unknown) => void) => void };
}

export type SocketStatus = "disconnected" | "connecting" | "connected" | "reconnecting";

// Exponential backoff with jitter, capped at 30s; retries until the socket reconnects.
export const RECONNECT_OPTIONS = {
  reconnection: true,
  reconnectionAttempts: Infinity,
  reconnectionDelay: 1000,
  reconnectionDelayMax: 30_000,
  randomizationFactor: 0.5,
} as const;

interface SocketContextType {
  socket: SocketInstance | null;
  connected: boolean;
  status: SocketStatus;
  lastNotification: Notification | null;
}

const SocketContext = createContext<SocketContextType>({
  socket: null,
  connected: false,
  status: "disconnected",
  lastNotification: null,
});

export const useSocket = () => useContext(SocketContext);

export const SocketProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { address } = useWalletStore();
  const [socket, setSocketState] = useState<SocketInstance | null>(null);
  const [connected, setConnected] = useState(false);
  const [status, setStatus] = useState<SocketStatus>("disconnected");
  const [lastNotification, setLastNotification] = useState<Notification | null>(null);
  const socketRef = useRef<SocketInstance | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const scheduleStateUpdate = (fn: () => void) => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      timeoutRef.current = setTimeout(fn, 0);
    };

    if (!address) {
      if (socketRef.current) {
        socketRef.current.disconnect();
        socketRef.current = null;
        scheduleStateUpdate(() => {
          setSocketState(null);
          setConnected(false);
          setStatus("disconnected");
        });
      }
      return;
    }

    const socketUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";

    const newSocket = io(socketUrl, {
      query: { address },
      ...RECONNECT_OPTIONS,
    }) as unknown as SocketInstance;
    scheduleStateUpdate(() => setStatus("connecting"));

    newSocket.on("connect", () => {
      setConnected(true);
      setStatus("connected");
      console.log("WebSocket connected");
    });

    newSocket.on("disconnect", () => {
      setConnected(false);
      setStatus("reconnecting");
      console.log("WebSocket disconnected");
    });

    newSocket.io?.on("reconnect_attempt", () => {
      setStatus("reconnecting");
    });

    newSocket.on("notification", (data: unknown) => {
      setLastNotification(data as Notification);
      console.log("New notification received:", data);
    });

    socketRef.current = newSocket;
    scheduleStateUpdate(() => {
      setSocketState(newSocket);
    });

    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      if (socketRef.current) {
        socketRef.current.disconnect();
        socketRef.current = null;
        scheduleStateUpdate(() => {
          setSocketState(null);
        });
      }
    };
  }, [address]);

  return (
    <SocketContext.Provider value={{ socket, connected, status, lastNotification }}>
      {children}
    </SocketContext.Provider>
  );
};
