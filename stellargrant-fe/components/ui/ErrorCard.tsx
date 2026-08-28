"use client";

import { useState } from "react";
import { AlertTriangle } from "lucide-react";

export interface ErrorCardProps {
  title?: string;
  message?: string;
  onRetry?: () => void;
  compact?: boolean;
  type?: "network" | "api" | "rpc" | "generic";
}

const ERROR_MAP = {
  network: {
    title: "Network Connection Lost",
    message: "You appear to be offline. Check your connection and try again.",
  },
  api: {
    title: "API Unavailable",
    message: "The StellarGrant API is temporarily unavailable.",
  },
  rpc: {
    title: "Stellar Network Error",
    message:
      "Cannot reach the Stellar network. The Stellar testnet may be undergoing maintenance.",
  },
  generic: {
    title: "Something went wrong",
    message: "Something went wrong. Please try again.",
  },
};

export function ErrorCard({
  title,
  message,
  onRetry,
  compact = false,
  type = "generic",
}: ErrorCardProps) {
  const [expanded, setExpanded] = useState(false);
  
  const finalTitle = title || ERROR_MAP[type].title;
  const finalMessage = message || ERROR_MAP[type].message;
  
  const needsTruncation = finalMessage.length > 200;
  const displayMessage = needsTruncation && !expanded ? finalMessage.slice(0, 200) + "…" : finalMessage;

  if (compact) {
    return (
      <div className="flex items-center gap-2">
        <AlertTriangle className="h-4 w-4 shrink-0 text-danger" />
        <span className="font-mono text-xs text-text-muted">
          {displayMessage}
          {needsTruncation && (
            <button
              onClick={() => setExpanded(!expanded)}
              className="ml-1 text-danger underline"
            >
              {expanded ? "less" : "more"}
            </button>
          )}
        </span>
      </div>
    );
  }

  return (
    <div className="border border-danger/40 bg-danger/5 p-4">
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-danger" />
        <div className="min-w-0 flex-1">
          <p className="font-orbitron text-sm font-bold text-danger">{finalTitle}</p>
          <p className="mt-1 font-mono text-xs text-text-muted">
            {displayMessage}
            {needsTruncation && (
              <button
                onClick={() => setExpanded(!expanded)}
                className="ml-1 text-danger underline"
              >
                {expanded ? "less" : "more"}
              </button>
            )}
          </p>
          {onRetry && (
            <button
              onClick={onRetry}
              className="mt-3 inline-flex items-center justify-center border border-danger/40 px-4 py-1.5 font-orbitron text-xs font-bold uppercase tracking-wider text-danger transition-all duration-300 hover:bg-danger/10"
            >
              Try again
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
