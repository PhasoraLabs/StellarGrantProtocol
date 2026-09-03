"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { STELLAR_NETWORK } from "@/lib/constants";
import { useNetworkStatus } from "@/hooks/useNetworkStatus";
import { getHorizonClient } from "@/lib/stellar/client";

/**
 * AppFooter Component
 * 
 * Footer with links to GitHub, navigation, network status,
 * and ledger information.
 */
export function AppFooter() {
  const { isOnline } = useNetworkStatus();
  const [latestLedger, setLatestLedger] = useState<number | null>(null);

  useEffect(() => {
    const fetchLedger = async () => {
      try {
        const response = await getHorizonClient().ledgers().order("desc").limit(1).call();
        if (response.records && response.records.length > 0) {
          setLatestLedger(response.records[0].sequence);
        }
      } catch (_err) {
        // Silent catch to prevent console errors if network is down
      }
    };

    void fetchLedger();
    const interval = setInterval(fetchLedger, 30000); // Poll every 30 seconds

    return () => clearInterval(interval);
  }, []);

  return (
    <footer className="bg-bg-secondary border-t border-border-color mt-auto">
      <div className="max-w-7xl mx-auto py-12 px-6 md:px-20">
        <div className="flex flex-col md:flex-row md:justify-between gap-8 md:gap-4 mb-8">
          
          {/* Logo Section */}
          <div className="flex flex-col gap-2">
            <Link 
              href="/" 
              className="font-orbitron text-accent-primary font-bold text-xl tracking-wider hover:opacity-80 transition-opacity"
            >
              STELLAR·GRANT
            </Link>
            <p className="font-ibm-plex-mono text-text-muted text-sm max-w-sm">
              Decentralized milestone-based grant management on Stellar.
            </p>
          </div>

          {/* Links Section */}
          <nav className="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-3">
            <Link href="/grants" className="font-mono text-xs text-text-muted hover:text-text-primary transition-colors">
              Explore
            </Link>
            <Link href="/review" className="font-mono text-xs text-text-muted hover:text-text-primary transition-colors">
              Review
            </Link>
            <Link href="/grants/create" className="font-mono text-xs text-text-muted hover:text-text-primary transition-colors">
              Create
            </Link>
            <Link href="/leaderboard" className="font-mono text-xs text-text-muted hover:text-text-primary transition-colors">
              Leaderboard
            </Link>
            <Link href="/dashboard" className="font-mono text-xs text-text-muted hover:text-text-primary transition-colors">
              Dashboard
            </Link>
            <Link href="/settings" className="font-mono text-xs text-text-muted hover:text-text-primary transition-colors">
              Settings
            </Link>
          </nav>

        </div>

        {/* Network Status Bar */}
        <div className="border-t border-border-color/50 pt-6 mb-6 flex flex-col md:flex-row md:items-center md:justify-between gap-4">
          <div className="flex items-center gap-2 font-mono text-xs text-text-muted">
            <span className="flex items-center gap-1.5" aria-label={isOnline ? "Online" : "Offline"}>
              <span 
                className={`w-2 h-2 rounded-full ${isOnline ? 'bg-success' : 'bg-danger'}`}
                aria-hidden="true"
              />
              <span className="sr-only">{isOnline ? "Online" : "Offline"}</span>
              <span className="capitalize">{STELLAR_NETWORK}</span>
            </span>
            <span aria-hidden="true">·</span>
            <span>Block: {latestLedger ? latestLedger.toLocaleString() : "..."}</span>
            <span aria-hidden="true">·</span>
            <span>Built with ❤ on Stellar</span>
          </div>
        </div>

        {/* Copyright Bar */}
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 font-mono text-xs text-text-muted">
          <div>
            © {new Date().getFullYear()} StellarGrant Protocol
          </div>
          <div className="flex items-center gap-6">
            <a 
              href="https://github.com/org/repo/blob/main/LICENSE" 
              target="_blank" 
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              MIT License
            </a>
            <a 
              href="https://github.com/your-org/stellargrant-fe" 
              target="_blank" 
              rel="noopener noreferrer"
              className="flex items-center gap-2 hover:text-text-primary transition-colors"
              aria-label="GitHub Repository"
              title="GitHub"
            >
              GitHub
              <svg 
                xmlns="http://www.w3.org/2000/svg" 
                viewBox="0 0 24 24" 
                fill="none" 
                stroke="currentColor" 
                strokeWidth="2" 
                strokeLinecap="round" 
                strokeLinejoin="round" 
                className="w-4 h-4"
                aria-hidden="true"
              >
                <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.02c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A4.8 4.8 0 0 0 9 18v4" />
                <path d="M9 18c-4.51 2-5-2-7-2" />
              </svg>
            </a>
          </div>
        </div>

      </div>
    </footer>
  );
}
