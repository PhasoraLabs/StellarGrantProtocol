/**
 * WalletConnect Component
 *
 * Primary connect button. Detects installed wallet extensions and shows
 * a modal to select Freighter, xBull, or Passkey.
 *
 * Render states:
 *   1. Freighter not installed → "Install Freighter" button
 *   2. Installed, not connected → "Connect Wallet" button → opens selection modal
 *   3. Connected → truncated address + XLM balance + green dot + disconnect dropdown
 */

"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Wallet } from "lucide-react";
import { useWallet } from "@/hooks/useWallet";
import { StatusDot } from "@/components/ui/StatusDot";
import { getHorizonClient } from "@/lib/stellar/client";
import { WalletSelectModal } from "./WalletSelectModal";
import Link from "next/link";

// ── Helpers ────────────────────────────────────────────────────────────────────

function truncateAddress(address: string): string {
  if (address.length <= 10) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

async function checkFreighterInstalled(): Promise<boolean> {
  try {
    const { isConnected } = await import("@stellar/freighter-api");
    await isConnected();
    return true;
  } catch {
    return false;
  }
}

async function fetchXlmBalance(address: string): Promise<string | null> {
  try {
    const horizon = getHorizonClient();
    const account = await horizon.loadAccount(address);
    const native = account.balances.find((b) => b.asset_type === "native");
    if (!native) return null;
    // Format to 2 decimal places for display
    return parseFloat(native.balance).toFixed(2);
  } catch {
    return null;
  }
}

// ── Component ──────────────────────────────────────────────────────────────────

interface WalletConnectProps {
  variant?: "button" | "icon";
  onConnect?: (address: string) => void;
}

export function WalletConnect({
  variant: _variant = "button",
  onConnect,
}: WalletConnectProps) {
  const { address, isConnected, disconnect } = useWallet();

  const [freighterInstalled, setFreighterInstalled] = useState<boolean | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [xlmBalance, setXlmBalance] = useState<string | null>(null);
  const [copyLabel, setCopyLabel] = useState("Copy Address");

  const dropdownRef = useRef<HTMLDivElement>(null);

  // ── Detect Freighter on mount ────────────────────────────────────────────────
  useEffect(() => {
    checkFreighterInstalled().then(setFreighterInstalled);
  }, []);

  // ── Fetch XLM balance when connected ────────────────────────────────────────
  // Never call setState synchronously in an effect body (react-hooks/set-state-in-effect).
  // setState is only called inside the async .then() callback. When address is
  // absent the component renders State 1/2, so the stale balance value is never
  // displayed — no synchronous reset needed.
  useEffect(() => {
    if (!address) return;
    let cancelled = false;
    void fetchXlmBalance(address).then((bal) => {
      if (!cancelled) setXlmBalance(bal);
    });
    onConnect?.(address);
    return () => { cancelled = true; };
  }, [address, onConnect]);

  // ── Close dropdown on outside click ─────────────────────────────────────────
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    }
    if (dropdownOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [dropdownOpen]);

  // ── Copy address helper ──────────────────────────────────────────────────────
  const handleCopyAddress = useCallback(async () => {
    if (!address) return;
    try {
      await navigator.clipboard.writeText(address);
      setCopyLabel("Copied!");
      setTimeout(() => setCopyLabel("Copy Address"), 2000);
    } catch {
      // silently fail
    }
    setDropdownOpen(false);
  }, [address]);

  // ── State 1: Freighter not installed ────────────────────────────────────────
  if (freighterInstalled === false) {
    return (
      <a
        id="wallet-install-freighter"
        href="https://freighter.app"
        target="_blank"
        rel="noopener noreferrer"
        className="wallet-connect-btn wallet-connect-btn--install"
        aria-label="Install Freighter wallet extension"
      >
        Install Freighter
      </a>
    );
  }

  // ── State 3: Connected ───────────────────────────────────────────────────────
  if (isConnected && address) {
    return (
      <div className="wallet-connect-connected" ref={dropdownRef}>
        <button
          id="wallet-connected-btn"
          className="wallet-connect-btn wallet-connect-btn--connected"
          onClick={() => setDropdownOpen((prev) => !prev)}
          aria-haspopup="true"
          aria-expanded={dropdownOpen}
          aria-label="Wallet options"
        >
          <StatusDot status="active" label="" />
          <span className="wallet-connect-address">{truncateAddress(address)}</span>
          {xlmBalance !== null && (
            <span className="wallet-connect-balance">{xlmBalance} XLM</span>
          )}
          <svg
            className={`wallet-connect-chevron ${dropdownOpen ? "wallet-connect-chevron--open" : ""}`}
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>

        <AnimatePresence>
          {dropdownOpen && (
            <motion.div
              className="wallet-dropdown"
              initial={{ opacity: 0, y: -6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.15 }}
              role="menu"
              aria-label="Wallet menu"
            >
              <Link
                id="wallet-dropdown-dashboard"
                href="/profile"
                className="wallet-dropdown-item"
                role="menuitem"
                onClick={() => setDropdownOpen(false)}
              >
                Dashboard →
              </Link>
              <button
                id="wallet-dropdown-copy"
                className="wallet-dropdown-item"
                role="menuitem"
                onClick={() => void handleCopyAddress()}
              >
                {copyLabel}
              </button>
              <button
                id="wallet-dropdown-disconnect"
                className="wallet-dropdown-item wallet-dropdown-item--danger"
                role="menuitem"
                onClick={() => {
                  disconnect();
                  setDropdownOpen(false);
                }}
              >
                Disconnect
              </button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  }

  // ── State 2: Installed but not connected (also shown while detecting) ────────
  return (
    <>
      <button
        id="wallet-connect-btn"
        className="wallet-connect-btn wallet-connect-btn--default"
        onClick={() => setModalOpen(true)}
        disabled={freighterInstalled === null}
        aria-label="Connect wallet"
      >
        <Wallet size={16} aria-hidden="true" />
        Connect Wallet
      </button>

      <WalletSelectModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
      />
    </>
  );
}
