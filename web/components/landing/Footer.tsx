"use client";

import React from "react";
import Link from "next/link";

export const Footer = () => {
  return (
    <footer className="bg-bg-secondary border-t border-border-color py-16 px-6 md:px-20">
      <div className="max-w-6xl mx-auto grid grid-cols-1 md:grid-cols-3 gap-12 items-start">
        {/* Left: Brand */}
        <div className="flex flex-col gap-4">
          <h3 className="font-orbitron text-lg font-bold text-text-primary tracking-widest">
            STELLARGRANT PROTOCOL
          </h3>
          <p className="font-mono text-sm text-text-muted max-w-[250px] leading-relaxed">
            The decentralized standard for milestone-based funding on the Stellar network.
          </p>
        </div>

        {/* Center: Navigation */}
        <div className="flex flex-col md:items-center gap-6">
          <div className="grid grid-cols-2 gap-x-12 gap-y-3">
            <Link href="/grants" className="font-mono text-xs text-text-muted hover:text-accent-primary transition-colors">GRANTS</Link>
            <Link href="/docs" className="font-mono text-xs text-text-muted hover:text-accent-primary transition-colors">DOCS</Link>
            <Link href="/sdk" className="font-mono text-xs text-text-muted hover:text-accent-primary transition-colors">SDK</Link>
            <Link href="/api" className="font-mono text-xs text-text-muted hover:text-accent-primary transition-colors">API</Link>
            <Link href="/github" className="font-mono text-xs text-text-muted hover:text-accent-primary transition-colors">GITHUB</Link>
          </div>
        </div>

        {/* Right: Powered by */}
        <div className="flex flex-col md:items-end gap-4">
          <p className="font-mono text-xs text-text-muted">
            Built on Stellar · Powered by Soroban
          </p>
          <div className="w-10 h-10 border border-border-color flex items-center justify-center">
            {/* Simple SVG placeholder for Stellar Logo */}
            <svg viewBox="0 0 24 24" className="w-6 h-6 fill-text-muted" xmlns="http://www.w3.org/2000/svg">
              <path d="M12 2L4.5 20.29L5.21 21L12 18L18.79 21L19.5 20.29L12 2Z" />
            </svg>
          </div>
        </div>
      </div>

      {/* Bottom Strip */}
      <div className="max-w-6xl mx-auto mt-16 pt-8 border-t border-border-color/20 text-center md:text-left">
        <p className="font-mono text-[10px] text-text-muted uppercase tracking-widest">
          © 2024 STELLARGRANT PROTOCOL · NO RIGHTS RESERVED · OPEN SOURCE MISSION
        </p>
      </div>
    </footer>
  );
};
