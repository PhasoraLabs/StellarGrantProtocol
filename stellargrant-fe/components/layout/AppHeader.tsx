"use client";

import Link from "next/link";
import { NotificationBell } from "@/components/layout/NotificationBell";
import { WalletConnect } from "@/components/wallet/WalletConnect";

function HeaderSearchIcon() {
  return (
    <svg
      aria-hidden="true"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  );
}

export function AppHeader() {
  return (
    <header className="border-b border-border-color bg-bg-secondary/80 backdrop-blur-sm sticky top-0 z-30">
      <div className="container mx-auto px-4 py-4 flex items-center justify-between">
        <div className="flex items-center gap-4 sm:gap-8">
          <Link
            href="/"
            className="font-orbitron text-lg font-bold text-text-primary hover:text-accent-primary transition-colors"
          >
            StellarGrants
          </Link>
          <nav className="hidden sm:flex gap-4 font-mono text-sm">
            <Link href="/grants" className="text-text-muted hover:text-accent-secondary transition-colors">
              Explore
            </Link>
            <Link
              href="/grants/create"
              className="text-text-muted hover:text-accent-secondary transition-colors"
            >
              Create
            </Link>
            <Link href="/profile" className="text-text-muted hover:text-accent-secondary transition-colors">
              Profile
            </Link>
          </nav>
          <Link
            href="/search"
            className="inline-flex items-center justify-center p-2 text-text-muted transition-colors hover:text-accent-secondary"
            aria-label="Search"
            title="Search"
          >
            <HeaderSearchIcon />
          </Link>
        </div>
        <div className="flex items-center gap-3">
          <span className="hidden sm:inline font-mono text-[10px] uppercase tracking-wider px-2 py-1 border border-warning/40 text-warning">
            Testnet
          </span>
          <NotificationBell />
          <WalletConnect />
        </div>
      </div>
    </header>
  );
}
