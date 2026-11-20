/**
 * /settings — Issue #386.
 *
 * Personal preferences (notifications, display, developer) plus a wallet
 * panel. All state lives in `useUserPreferences`, which persists to
 * localStorage. "Reset to defaults" wipes the stored preferences after a
 * confirmation dialog.
 */

import type { Metadata } from "next";
import { SettingsPageClient } from "@/components/settings/SettingsPageClient";

export const metadata: Metadata = {
  title: "Settings — StellarGrant Protocol",
};

export default function SettingsPage() {
  return <SettingsPageClient />;
}
