"use client";

import { useState } from "react";
import { useUserPreferences } from "@/hooks/useUserPreferences";
import { useAddressFormat } from "@/hooks/useAddressFormat";
import { useWallet } from "@/hooks/useWallet";
import { ToggleSwitch } from "@/components/settings/ToggleSwitch";
import { RadioGroup } from "@/components/settings/RadioGroup";

const ADDRESS_FORMAT_OPTIONS = [
  { value: "short" as const, label: "Short" },
  { value: "medium" as const, label: "Medium" },
  { value: "full" as const, label: "Full" },
];

const XLM_DECIMAL_OPTIONS = [
  { value: 2 as const, label: "2" },
  { value: 4 as const, label: "4" },
  { value: 7 as const, label: "7" },
];

const DATE_FORMAT_OPTIONS = [
  { value: "relative" as const, label: "Relative" },
  { value: "absolute" as const, label: "Absolute" },
];

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="border border-border-color bg-surface">
      <header className="border-b border-border-color px-6 py-3">
        <h2 className="font-orbitron text-sm font-bold uppercase tracking-wider text-text-primary">
          {title}
        </h2>
      </header>
      <div className="divide-y divide-border-color px-6">{children}</div>
    </section>
  );
}

export function SettingsPageClient() {
  const { preferences, setPreferences, reset } = useUserPreferences();
  const formatAddress = useAddressFormat();
  const { address, isConnected, network, disconnect } = useWallet();

  const [confirmingReset, setConfirmingReset] = useState(false);

  return (
    <main className="mx-auto max-w-3xl px-6 py-12 space-y-6">
      <header>
        <h1 className="font-orbitron text-2xl font-bold uppercase tracking-wider text-text-primary">
          Settings
        </h1>
      </header>

      <Section title="Notifications">
        <ToggleSwitch
          label="Notify when a watched grant receives funding"
          checked={preferences.notifyOnFunding}
          onChange={(v) => setPreferences({ notifyOnFunding: v })}
        />
        <ToggleSwitch
          label="Notify when a milestone I review is submitted"
          checked={preferences.notifyOnMilestoneSubmit}
          onChange={(v) => setPreferences({ notifyOnMilestoneSubmit: v })}
        />
        <ToggleSwitch
          label="Notify when my grant receives a vote"
          checked={preferences.notifyOnVote}
          onChange={(v) => setPreferences({ notifyOnVote: v })}
        />
        <ToggleSwitch
          label="Notify when a payout is released on my grant"
          checked={preferences.notifyOnPayout}
          onChange={(v) => setPreferences({ notifyOnPayout: v })}
        />
      </Section>

      <Section title="Display">
        <RadioGroup
          label="Address format"
          value={preferences.addressFormat}
          onChange={(v) => setPreferences({ addressFormat: v })}
          options={ADDRESS_FORMAT_OPTIONS}
        />
        <RadioGroup
          label="XLM decimals"
          value={preferences.xlmDecimals}
          onChange={(v) => setPreferences({ xlmDecimals: v })}
          options={XLM_DECIMAL_OPTIONS}
        />
        <RadioGroup
          label="Dates"
          value={preferences.dateFormat}
          onChange={(v) => setPreferences({ dateFormat: v })}
          options={DATE_FORMAT_OPTIONS}
        />
      </Section>

      <Section title="Developer">
        <ToggleSwitch
          label="Show transaction hashes inline"
          checked={preferences.showTxHashes}
          onChange={(v) => setPreferences({ showTxHashes: v })}
        />
        <ToggleSwitch
          label="Debug mode (verbose errors)"
          checked={preferences.debugMode}
          onChange={(v) => setPreferences({ debugMode: v })}
        />
      </Section>

      <Section title="Wallet">
        <div className="flex items-center justify-between py-3">
          <span className="font-mono text-sm text-text-primary">Connected as</span>
          <span className="font-mono text-sm text-text-muted">
            {isConnected && address ? formatAddress(address) : "Not connected"}
          </span>
        </div>
        <div className="flex items-center justify-between py-3">
          <span className="font-mono text-sm text-text-primary">Network</span>
          <span className="font-mono text-sm text-text-muted capitalize">{network}</span>
        </div>
        {isConnected && (
          <div className="flex justify-end py-3">
            <button
              type="button"
              onClick={disconnect}
              className="border border-accent-primary bg-transparent px-4 py-2 font-orbitron text-xs font-bold uppercase tracking-wider text-accent-primary transition-colors hover:bg-accent-primary hover:text-bg-primary"
            >
              Disconnect
            </button>
          </div>
        )}
      </Section>

      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => setConfirmingReset(true)}
          className="border border-danger bg-transparent px-4 py-2 font-orbitron text-xs font-bold uppercase tracking-wider text-danger transition-colors hover:bg-danger hover:text-bg-primary"
        >
          Reset all settings to defaults
        </button>
      </div>

      {confirmingReset && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label="Reset settings"
          className="fixed inset-0 z-50 flex items-center justify-center p-4"
        >
          <div
            aria-hidden="true"
            onClick={() => setConfirmingReset(false)}
            className="absolute inset-0 bg-black/60"
          />
          <div className="relative w-full max-w-md border border-border-color bg-surface p-6">
            <h3 className="font-orbitron text-lg font-medium text-text-primary">
              Reset all settings to defaults?
            </h3>
            <p className="mt-2 font-mono text-sm text-text-muted">
              Your stored preferences will be cleared and the defaults will be restored.
            </p>
            <div className="mt-6 flex justify-end gap-3">
              <button
                type="button"
                onClick={() => setConfirmingReset(false)}
                className="border border-accent-primary bg-transparent px-4 py-2 font-orbitron text-xs font-bold uppercase tracking-wider text-accent-primary transition-colors hover:bg-accent-primary hover:text-bg-primary"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  reset();
                  setConfirmingReset(false);
                }}
                className="bg-danger px-4 py-2 font-orbitron text-xs font-bold uppercase tracking-wider text-bg-primary transition-colors hover:bg-opacity-90"
              >
                Reset
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
