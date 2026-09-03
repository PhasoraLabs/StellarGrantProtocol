"use client";

import { useCallback, useState } from "react";
import { toast } from "@/lib/toast";

async function copyText(text: string): Promise<void> {
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const el = document.createElement("textarea");
  el.value = text;
  el.style.cssText = "position:fixed;top:-9999px;left:-9999px;opacity:0";
  document.body.appendChild(el);
  el.focus();
  el.select();
  document.execCommand("copy");
  document.body.removeChild(el);
}

export function useCopyToClipboard(resetMs = 2000) {
  const [isCopied, setIsCopied] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const copy = useCallback(
    async (text: string) => {
      try {
        await copyText(text);
        setIsCopied(true);
        setError(null);
        toast({ title: "Copied to clipboard", variant: "success", duration: 2000 });
        setTimeout(() => setIsCopied(false), resetMs);
      } catch (err) {
        const e = err instanceof Error ? err : new Error(String(err));
        setError(e);
        toast({ title: "Copy failed", description: e.message, variant: "error" });
      }
    },
    [resetMs]
  );

  return { copy, isCopied, error };
}