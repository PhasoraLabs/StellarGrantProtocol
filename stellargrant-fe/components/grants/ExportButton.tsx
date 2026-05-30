"use client";

/**
 * ExportButton Component
 *
 * A dropdown-style export button for the Grant Detail page.
 * Allows users to download grant data as JSON or milestones as CSV.
 * Works without a wallet connection (read-only feature).
 *
 * @see https://github.com/StellarGrant/StellarGrant-fe/issues/388
 */

import { useState, useRef, useEffect, useCallback } from "react";
import type { Grant, Milestone } from "@/types";
import {
  exportGrantAsJSON,
  exportGrantAsCSV,
  type FunderRecord,
  exportFundersAsCSV,
} from "@/lib/utils/export";

interface ExportButtonProps {
  grant: Grant;
  milestones: Milestone[];
  funders?: FunderRecord[];
}

export function ExportButton({ grant, milestones, funders }: ExportButtonProps) {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen]);

  const handleExportJSON = useCallback(() => {
    exportGrantAsJSON(grant, milestones);
    setIsOpen(false);
  }, [grant, milestones]);

  const handleExportMilestonesCSV = useCallback(() => {
    exportGrantAsCSV(grant, milestones);
    setIsOpen(false);
  }, [grant, milestones]);

  const handleExportFundersCSV = useCallback(() => {
    if (funders && funders.length > 0) {
      exportFundersAsCSV(funders);
    }
    setIsOpen(false);
  }, [funders]);

  return (
    <div className="relative" ref={menuRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="px-4 py-2 text-sm font-medium rounded-sm border border-accent-secondary text-accent-secondary hover:bg-accent-secondary/10 transition-colors flex items-center gap-2"
        aria-expanded={isOpen}
        aria-haspopup="true"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
        Export
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
          className={`transition-transform ${isOpen ? "rotate-180" : ""}`}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {isOpen && (
        <div
          className="absolute right-0 mt-1 z-50 min-w-[200px] rounded-sm border py-1 shadow-lg"
          style={{
            background: "#111D35",
            borderColor: "#1E3A5F",
          }}
          role="menu"
        >
          <button
            onClick={handleExportJSON}
            className="w-full text-left px-4 py-2 text-sm text-text-secondary hover:bg-accent-secondary/10 hover:text-accent-secondary transition-colors flex items-center gap-2"
            role="menuitem"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
            Export as JSON
          </button>
          <button
            onClick={handleExportMilestonesCSV}
            className="w-full text-left px-4 py-2 text-sm text-text-secondary hover:bg-accent-secondary/10 hover:text-accent-secondary transition-colors flex items-center gap-2"
            role="menuitem"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
              <line x1="3" y1="9" x2="21" y2="9" />
              <line x1="3" y1="15" x2="21" y2="15" />
              <line x1="9" y1="3" x2="9" y2="21" />
            </svg>
            Export milestones as CSV
          </button>
          {funders && funders.length > 0 && (
            <button
              onClick={handleExportFundersCSV}
              className="w-full text-left px-4 py-2 text-sm text-text-secondary hover:bg-accent-secondary/10 hover:text-accent-secondary transition-colors flex items-center gap-2"
              role="menuitem"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
                <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
              </svg>
              Export funders as CSV
            </button>
          )}
        </div>
      )}
    </div>
  );
}
