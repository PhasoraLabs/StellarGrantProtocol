"use client";

import React, { useEffect } from "react";

export type KeyboardShortcutsOverlayProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function KeyboardShortcutsOverlay({
  open,
  onOpenChange,
}: KeyboardShortcutsOverlayProps) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (open && e.key === "Escape") {
        onOpenChange(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onOpenChange]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Keyboard Shortcuts"
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
    >
      <div
        aria-hidden="true"
        onClick={() => onOpenChange(false)}
        className="absolute inset-0 bg-black/60"
      />
      <div className="relative w-full max-w-2xl bg-surface border border-border-color rounded-none p-6 shadow-xl max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between border-b border-border-color/30 pb-4 mb-4">
          <h2 className="font-orbitron text-lg font-medium text-text-primary">
            Keyboard Shortcuts
          </h2>
          <button
            type="button"
            onClick={() => onOpenChange(false)}
            className="text-text-muted hover:text-text-primary transition-colors font-mono text-sm"
            aria-label="Close"
          >
            Esc
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
          {/* Navigation */}
          <div>
            <h3 className="font-mono text-xs uppercase tracking-widest text-text-muted mb-3">
              Navigation
            </h3>
            <ul className="space-y-2">
              <ShortcutRow keys={["g", "g"]} description="Browse Grants" />
              <ShortcutRow keys={["g", "d"]} description="Dashboard" />
              <ShortcutRow keys={["g", "r"]} description="Reviewer Page" />
              <ShortcutRow keys={["g", "l"]} description="Leaderboard" />
              <ShortcutRow keys={["g", "c"]} description="Create Grant" />
              <ShortcutRow keys={["g", "s"]} description="Settings" />
              <ShortcutRow keys={["/"]} description="Search" />
              <ShortcutRow keys={["?"]} description="Keyboard Shortcuts" />
            </ul>
          </div>

          <div className="space-y-8">
            {/* Grants List */}
            <div>
              <h3 className="font-mono text-xs uppercase tracking-widest text-text-muted mb-3">
                Grants List
              </h3>
              <ul className="space-y-2">
                <ShortcutRow keys={["f"]} description="Focus Filters" />
                <ShortcutRow keys={["j"]} description="Next Grant" />
                <ShortcutRow keys={["k"]} description="Previous Grant" />
                <ShortcutRow keys={["Enter"]} description="Open Grant" />
              </ul>
            </div>

            {/* Grant Detail */}
            <div>
              <h3 className="font-mono text-xs uppercase tracking-widest text-text-muted mb-3">
                Grant Detail
              </h3>
              <ul className="space-y-2">
                <ShortcutRow keys={["F"]} description="Fund Grant" />
                <ShortcutRow keys={["h"]} description="Grant History" />
              </ul>
            </div>

            {/* Milestone Detail */}
            <div>
              <h3 className="font-mono text-xs uppercase tracking-widest text-text-muted mb-3">
                Milestone Detail
              </h3>
              <ul className="space-y-2">
                <ShortcutRow keys={["a"]} description="Approve Milestone" />
                <ShortcutRow keys={["r"]} description="Reject Milestone" />
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function ShortcutRow({ keys, description }: { keys: string[]; description: string }) {
  return (
    <li className="flex items-center justify-between text-sm">
      <span className="font-mono text-text-primary">{description}</span>
      <div className="flex gap-1">
        {keys.map((k, i) => (
          <kbd
            key={i}
            className="flex min-w-[24px] items-center justify-center rounded-none border border-border-color bg-bg-secondary px-1.5 py-0.5 font-mono text-xs text-text-muted"
          >
            {k}
          </kbd>
        ))}
      </div>
    </li>
  );
}
