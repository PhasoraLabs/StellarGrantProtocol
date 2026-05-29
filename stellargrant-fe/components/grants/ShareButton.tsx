"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Share2 } from "lucide-react";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";

const SHARE_BASE = "https://stellargrant.io/grants";

interface ShareButtonProps {
  grantId: string;
  grantTitle: string;
  fundedPercent: number;
}

function buildShareText(title: string, percent: number, url: string): string {
  return `${title} is ${Math.round(percent)}% funded on @StellarGrant — join me in supporting this project 🚀\n${url}`;
}

export function ShareButton({ grantId, grantTitle, fundedPercent }: ShareButtonProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const { copy, copied } = useCopyToClipboard(2000);
  const grantUrl = `${SHARE_BASE}/${grantId}`;
  const shareText = buildShareText(grantTitle, fundedPercent, grantUrl);

  useEffect(() => {
    if (!open) return;
    function handleOutsideClick(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("click", handleOutsideClick);
    return () => document.removeEventListener("click", handleOutsideClick);
  }, [open]);

  const handleShareClick = useCallback(async () => {
    if (typeof navigator !== "undefined" && navigator.share) {
      try {
        await navigator.share({ title: grantTitle, url: grantUrl, text: shareText });
        return;
      } catch {
        /* fall through to popover */
      }
    }
    setOpen((v) => !v);
  }, [grantTitle, grantUrl, shareText]);

  const twitterUrl = `https://twitter.com/intent/tweet?text=${encodeURIComponent(shareText)}`;
  const farcasterUrl = `https://warpcast.com/~/compose?text=${encodeURIComponent(shareText)}`;

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        onClick={() => void handleShareClick()}
        title="Share this grant"
        className="inline-flex items-center justify-center p-1.5 text-text-muted hover:text-accent-primary transition-colors"
        aria-label="Share this grant"
      >
        <Share2 size={18} />
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-2 z-30 min-w-[200px] border border-border-color bg-surface py-1 shadow-lg">
          <button
            type="button"
            onClick={() => {
              void copy(grantUrl);
            }}
            className="flex w-full items-center gap-2 px-3 py-2 font-mono text-xs text-text-primary hover:bg-bg-secondary text-left"
          >
            {copied ? <Check size={14} className="text-success" /> : <span>📋</span>}
            Copy link
          </button>
          <a
            href={twitterUrl}
            target="_blank"
            rel="noopener noreferrer"
            onClick={() => setOpen(false)}
            className="flex w-full items-center gap-2 px-3 py-2 font-mono text-xs text-text-primary hover:bg-bg-secondary"
          >
            <span>𝕏</span> Share on X
          </a>
          <a
            href={farcasterUrl}
            target="_blank"
            rel="noopener noreferrer"
            onClick={() => setOpen(false)}
            className="flex w-full items-center gap-2 px-3 py-2 font-mono text-xs text-text-primary hover:bg-bg-secondary"
          >
            <span>🟣</span> Share on Farcaster
          </a>
        </div>
      )}
    </div>
  );
}
