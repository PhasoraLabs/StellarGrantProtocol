"use client";

import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { useAddressFormat } from "@/hooks/useAddressFormat";

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

interface WalletAddressProps {
  address: string;
  showCopyIcon?: boolean;
  showAvatar?: boolean;
  /** @deprecated use showCopyIcon */
  showCopy?: boolean;
}

export function WalletAddress({
  address,
  showCopyIcon = true,
  showAvatar = false,
  showCopy,
}: WalletAddressProps) {
  const { copy, isCopied } = useCopyToClipboard();
  const formatAddress = useAddressFormat();
  const iconVisible = showCopy !== undefined ? showCopy : showCopyIcon;

  return (
    <div className="inline-flex items-center gap-2">
      {showAvatar && (
        <div
          className="w-6 h-6 rounded-full bg-accent-secondary shrink-0"
          aria-hidden="true"
        />
      )}

      <span className="font-mono text-sm" title={address}>
        {formatAddress(address)}
      </span>

      {iconVisible && (
        <button
          type="button"
          onClick={() => void copy(address)}
          title="Click to copy full address"
          aria-label={`Copy address ${address}`}
          className={[
            "cursor-pointer transition-colors shrink-0",
            isCopied
              ? "text-accent-secondary"
              : "text-text-muted hover:text-accent-secondary",
          ].join(" ")}
        >
          {isCopied ? <CheckIcon /> : <CopyIcon />}
        </button>
      )}
    </div>
  );
}
