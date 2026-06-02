/**
 * AppHeader
 *
 * Global sticky header rendered on every page.
 * Desktop  (≥768 px): Logo | Nav links | Testnet chip | WalletConnect
 * Mobile   (<768 px): Logo | Hamburger ↔ × | slide-down menu with nav + wallet
 *
 * Design-system tokens:
 *  - bg-bg-primary/95 backdrop-blur-sm sticky top-0 z-50
 *  - Logo: Orbitron, text-accent-primary
 *  - Nav: IBM Plex Mono, text-text-muted → text-text-primary on hover,
 *         text-accent-primary for active route
 *  - Testnet chip: reads STELLAR_NETWORK from lib/constants (never hardcoded)
 */

"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Badge } from "@/components/ui/Badge";
import { WalletConnect } from "@/components/wallet/WalletConnect";
import { STELLAR_NETWORK } from "@/lib/constants";

// ── Nav links definition ──────────────────────────────────────────────────────

const NAV_LINKS = [
  { href: "/grants", label: "Explore" },
  { href: "/grants/create", label: "Create" },
  { href: "/leaderboard", label: "Leaderboard" },
  { href: "/review", label: "Review" },
] as const;

// ── Hamburger / Close icon ────────────────────────────────────────────────────

function HamburgerIcon({ isOpen }: { isOpen: boolean }) {
  return (
    <svg
      aria-hidden="true"
      width="22"
      height="22"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{
        transform: isOpen ? "rotate(90deg)" : "rotate(0deg)",
        transition: "transform 0.25s ease",
      }}
    >
      {isOpen ? (
        <>
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </>
      ) : (
        <>
          <line x1="3" y1="7" x2="21" y2="7" />
          <line x1="3" y1="12" x2="21" y2="12" />
          <line x1="3" y1="17" x2="21" y2="17" />
        </>
      )}
    </svg>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export function AppHeader() {
  const pathname = usePathname();
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  // Close mobile menu on route change
  useEffect(() => {
    setIsMobileMenuOpen(false);
  }, [pathname]);

  const isActive = (href: string) => pathname === href || pathname.startsWith(href + "/");

  const navLinkClass = (href: string) =>
    [
      "font-mono text-xs uppercase tracking-wider transition-colors duration-150 no-underline",
      isActive(href)
        ? "text-accent-primary"
        : "text-text-muted hover:text-text-primary",
    ].join(" ");

  return (
    <header
      className="sticky top-0 z-50 border-b border-border-color bg-bg-primary/95 backdrop-blur-sm"
      style={{ backgroundColor: "rgba(5, 10, 20, 0.95)" }}
    >
      {/* ── Desktop bar ───────────────────────────────────────────────── */}
      <div className="container mx-auto px-4 py-3 flex items-center justify-between">
        {/* Logo */}
        <Link
          href="/"
          className="font-orbitron text-base font-bold tracking-widest text-accent-primary hover:opacity-80 transition-opacity no-underline"
          aria-label="StellarGrant home"
        >
          STELLAR·GRANT
        </Link>

        {/* Desktop nav */}
        <nav className="hidden md:flex items-center gap-6" aria-label="Primary navigation">
          {NAV_LINKS.map(({ href, label }) => (
            <Link key={href} href={href} className={navLinkClass(href)}>
              {label}
            </Link>
          ))}
        </nav>

        {/* Right-side controls */}
        <div className="flex items-center gap-3">
          {/* Testnet chip — reads from constant, never hardcoded */}
          <span className="hidden md:inline">
            <Badge variant="warning" size="sm">
              {STELLAR_NETWORK}
            </Badge>
          </span>

          {/* WalletConnect — desktop */}
          <div className="hidden md:flex">
            <WalletConnect />
          </div>

          {/* Hamburger — mobile only */}
          <button
            type="button"
            className="inline-flex items-center justify-center p-2 text-text-muted hover:text-text-primary transition-colors md:hidden"
            aria-expanded={isMobileMenuOpen}
            aria-label="Toggle navigation"
            onClick={() => setIsMobileMenuOpen((prev) => !prev)}
          >
            <HamburgerIcon isOpen={isMobileMenuOpen} />
          </button>
        </div>
      </div>

      {/* ── Mobile slide-down menu ─────────────────────────────────────── */}
      {isMobileMenuOpen && (
        <div
          role="dialog"
          aria-label="Mobile navigation"
          className="md:hidden border-t border-border-color bg-bg-secondary"
        >
          <nav
            className="container mx-auto px-4 py-4 flex flex-col gap-1"
            aria-label="Mobile primary navigation"
          >
            {NAV_LINKS.map(({ href, label }) => (
              <Link
                key={href}
                href={href}
                onClick={() => setIsMobileMenuOpen(false)}
                className={[
                  navLinkClass(href),
                  "block py-2.5 border-b border-border-color/50 last:border-b-0",
                ].join(" ")}
              >
                {label}
              </Link>
            ))}
          </nav>

          {/* Network badge + wallet inside mobile menu */}
          <div className="container mx-auto px-4 pb-4 flex items-center gap-3">
            <Badge variant="warning" size="sm">
              {STELLAR_NETWORK}
            </Badge>
            <WalletConnect />
          </div>
        </div>
      )}
    </header>
  );
}
