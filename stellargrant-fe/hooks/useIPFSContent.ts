"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { fetchFromIPFS, IPFS_GATEWAYS } from "@/lib/ipfs/gateway";

export interface UseIPFSContentResult {
  content: string | null;
  contentType: string | null;
  isLoading: boolean;
  error: Error | null;
  gatewayUsed: string | null;
  retry: () => void;
}

// Module-level cache so the same CID is never fetched twice across hook instances.
export const contentCache = new Map<
  string,
  { content: string; contentType: string; gatewayUsed: string }
>();

export function useIPFSContent(cid: string | null): UseIPFSContentResult {
  const [content, setContent] = useState<string | null>(null);
  const [contentType, setContentType] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [gatewayUsed, setGatewayUsed] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  const cancelledRef = useRef(false);

  const retry = useCallback(() => {
    setRetryCount((n) => n + 1);
  }, []);

  useEffect(() => {
    if (!cid) return;

    const cached = contentCache.get(cid);
    if (cached) {
      setContent(cached.content);
      setContentType(cached.contentType);
      setGatewayUsed(cached.gatewayUsed);
      setIsLoading(false);
      setError(null);
      return;
    }

    cancelledRef.current = false;
    setIsLoading(true);
    setError(null);
    setContent(null);
    setContentType(null);
    setGatewayUsed(null);

    (async () => {
      try {
        const response = await fetchFromIPFS(cid);

        if (cancelledRef.current) return;

        const ct = response.headers.get("content-type") ?? "text/plain";

        let text = "";
        if (
          ct.includes("text") ||
          ct.includes("markdown") ||
          ct.includes("json") ||
          cid.endsWith(".txt") ||
          cid.endsWith(".md")
        ) {
          text = await response.text();
        }

        // Determine which gateway served the response
        const served =
          (response.url && IPFS_GATEWAYS.find((g) => response.url.startsWith(g))) ||
          (response.url ? new URL(response.url).hostname : "unknown");

        if (cancelledRef.current) return;

        contentCache.set(cid, { content: text, contentType: ct, gatewayUsed: served });
        setContent(text);
        setContentType(ct);
        setGatewayUsed(served);
        setIsLoading(false);
      } catch (err) {
        if (cancelledRef.current) return;
        setError(err instanceof Error ? err : new Error(String(err)));
        setIsLoading(false);
      }
    })();

    return () => {
      cancelledRef.current = true;
    };
  }, [cid, retryCount]);

  return { content, contentType, isLoading, error, gatewayUsed, retry };
}
