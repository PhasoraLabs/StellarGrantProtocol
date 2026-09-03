"use client";

/**
 * RichTextRenderer
 *
 * Safely renders user-supplied Markdown as sanitized HTML.
 * Uses DOMPurify on the client side to prevent XSS.
 * Falls back to a skeleton loader during server-side rendering.
 */

import { useEffect, useState } from "react";
import { markdownToSafeHtml } from "@/lib/utils/sanitize";

interface RichTextRendererProps {
  /** Raw markdown string to render */
  content: string;
  /** Optional additional className for the wrapper */
  className?: string;
  /** Optional fallback element shown while rendering */
  fallback?: React.ReactNode;
}

const LOADING_SENTINEL = "__loading__";

export default function RichTextRenderer({
  content,
  className = "",
  fallback,
}: RichTextRendererProps) {
  // Use a sentinel value to track loading state without a separate boolean.
  // This avoids calling setState synchronously inside the effect body.
  const [safeHtml, setSafeHtml] = useState<string>(LOADING_SENTINEL);

  useEffect(() => {
    let cancelled = false;

    markdownToSafeHtml(content).then((html) => {
      if (!cancelled) {
        setSafeHtml(html);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [content]);

  if (safeHtml === LOADING_SENTINEL) {
    return (
      fallback ?? (
        <div className="animate-pulse space-y-2">
          <div className="h-4 bg-white/10 rounded w-3/4" />
          <div className="h-4 bg-white/10 rounded w-full" />
          <div className="h-4 bg-white/10 rounded w-5/6" />
        </div>
      )
    );
  }

  return (
    <div
      className={`prose prose-invert max-w-none rich-text-content ${className}`}
      // Safe: content is sanitized by DOMPurify before being set
      dangerouslySetInnerHTML={{ __html: safeHtml }}
    />
  );
}
