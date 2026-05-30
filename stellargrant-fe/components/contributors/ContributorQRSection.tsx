"use client";

import { useState } from "react";
import dynamic from "next/dynamic";

const QRCode = dynamic(
  () => import("@/components/ui/QRCode").then((m) => m.QRCode),
  { ssr: false }
);

interface ContributorQRSectionProps {
  address: string;
}

export function ContributorQRSection({ address }: ContributorQRSectionProps) {
  const [open, setOpen] = useState(false);
  const truncated = `${address.slice(0, 6)}…${address.slice(-4)}`;

  return (
    <div className="border border-border-color/30 bg-surface">
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="w-full flex items-center gap-2 px-4 py-3 font-mono text-xs text-text-muted hover:text-text-primary transition-colors"
        aria-expanded={open}
      >
        <span>{open ? "▼" : "▶"}</span>
        <span>Show QR Code for this address</span>
      </button>
      {open && (
        <div className="px-4 pb-4 flex flex-col items-center gap-2">
          <QRCode value={address} size={200} label={truncated} downloadable />
        </div>
      )}
    </div>
  );
}
