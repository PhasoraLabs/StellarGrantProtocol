"use client";

/**
 * WalletAddress Component
 *
 * Displays a truncated Stellar address in GABC12…XYZ9 format.
 * Click to copy the full address to the clipboard, with:
 *   - A "Address copied" toast dispatched via the stellar:toast event bus
 *   - A 2-second checkmark icon swap on the copy button
 *   - Graceful fallback to document.execCommand('copy') on non-HTTPS
 *
 * Props:
 *   address       — full Stellar address
 *   showCopyIcon  — show/hide the copy icon (default: true)
 *   showAvatar    — show a coloured avatar circle (default: false)
 */

import { useState, useCallback } from "react";
import { toast } from "@/lib/toast";

// ── Inline SVG icons (no icon-library dependency) ─────────────────────────────

function CopyIcon() {
  return (
    <svg
      aria-hidden="true"
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg
      aria-hidden="true"
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}

// ── Clipboard helper ──────────────────────────────────────────────────────────

async function copyToClipboard(text: string): Promise<void> {
  // Modern Clipboard API (requires HTTPS or localhost)
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  // Fallback: document.execCommand (deprecated but works on HTTP)
  const el = document.createElement("textarea");
  el.value = text;
  el.style.cssText = "position:fixed;top:-9999px;left:-9999px;opacity:0";
  document.body.appendChild(el);
  el.focus();
  el.select();
  document.execCommand("copy");
  document.body.removeChild(el);
}

// ── Component ─────────────────────────────────────────────────────────────────

interface WalletAddressProps {
  address: string;
  /** Show or hide the copy icon. Default: true */
  showCopyIcon?: boolean;
  /** Show a coloured avatar circle. Default: false */
  showAvatar?: boolean;
  /** @deprecated use showCopyIcon — kept for back-compat */
  showCopy?: boolean;
}

const CHECKMARK_DURATION_MS = 2000;

/** Truncate to first-6 … last-4 per spec: GABC12…XYZ9 */
function truncateAddress(address: string): string {
  if (address.length <= 10) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

export function WalletAddress({
  address,
  showCopyIcon = true,
  showAvatar = false,
  showCopy, // back-compat alias
}: WalletAddressProps) {
  const [copied, setCopied] = useState(false);

  // Resolve alias: if old showCopy prop is explicitly passed, honour it
  const iconVisible = showCopy !== undefined ? showCopy : showCopyIcon;

  const handleCopy = useCallback(async () => {
    if (!address) return;
    try {
      await copyToClipboard(address);
    } catch {
      // If even the fallback fails, bail silently — nothing to show
      return;
    }

    toast({
      title: "Address copied",
      variant: "success",
      duration: 2000,
    });

    // Swap icon to checkmark for 2 s
    setCopied(true);
    setTimeout(() => setCopied(false), CHECKMARK_DURATION_MS);
  }, [address]);

  return (
    <div className="inline-flex items-center gap-2">
      {showAvatar && (
        <div
          className="w-6 h-6 rounded-full bg-accent-secondary shrink-0"
          aria-hidden="true"
        />
      )}

      <span
        className="font-mono text-sm"
        title={address}
      >
        {truncateAddress(address)}
      </span>

      {iconVisible && (
        <button
          type="button"
          onClick={() => void handleCopy()}
          title="Click to copy full address"
          aria-label={`Copy address ${address}`}
          className={[
            "cursor-pointer transition-colors shrink-0",
            copied
              ? "text-accent-secondary"
              : "text-text-muted hover:text-accent-secondary",
          ].join(" ")}
        >
          {copied ? <CheckIcon /> : <CopyIcon />}
        </button>
      )}
    </div>
  );
}
