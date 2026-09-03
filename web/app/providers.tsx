"use client";

/**
 * Providers Component
 *
 * Client-side provider wrapper for QueryClient, StellarGrantsProvider,
 * and SocketProvider. This component is imported into the root layout.
 */

import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { queryClient } from "@/lib/queryClient";
import { StellarGrantsProvider } from "@/components/StellarGrantsProvider";
import { SocketProvider } from "@/hooks/useSocket";
import { NotificationToast } from "@/components/ui/NotificationToast";

import { OfflineBanner } from "@/components/layout/OfflineBanner";
import { useNetworkStatus } from "@/hooks/useNetworkStatus";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { KeyboardShortcutsOverlay } from "@/components/ui/KeyboardShortcutsOverlay";

export function Providers({ children }: { children: React.ReactNode }) {
  const { isOnline } = useNetworkStatus();
  const router = useRouter();
  const [isShortcutsOpen, setIsShortcutsOpen] = useState(false);

  useEffect(() => {
    if (isOnline) {
      void queryClient.refetchQueries({ type: "active" });
    }
  }, [isOnline]);

  useKeyboardShortcuts([
    { key: "g g", description: "Browse Grants", action: () => router.push("/grants") },
    { key: "g d", description: "Dashboard", action: () => router.push("/dashboard") },
    { key: "g r", description: "Reviewer Page", action: () => router.push("/review") },
    { key: "g l", description: "Leaderboard", action: () => router.push("/leaderboard") },
    { key: "g c", description: "Create Grant", action: () => router.push("/grants/create") },
    { key: "g s", description: "Settings", action: () => router.push("/settings") },
    {
      key: "?",
      description: "Keyboard Shortcuts",
      action: () => setIsShortcutsOpen((o) => !o),
    },
    {
      key: "/",
      description: "Search",
      action: (e) => {
        const searchInput = document.querySelector('input[type="search"]') as HTMLInputElement | null;
        if (searchInput) {
          e?.preventDefault();
          searchInput.focus();
        } else {
          router.push("/search");
        }
      },
    },
  ]);

  return (
    <QueryClientProvider client={queryClient}>
      <StellarGrantsProvider>
        <SocketProvider>
          <OfflineBanner />
          {children}
          <NotificationToast />
        </SocketProvider>
      </StellarGrantsProvider>
      {process.env.NODE_ENV === "development" && (
        <ReactQueryDevtools initialIsOpen={false} />
      )}
      <KeyboardShortcutsOverlay open={isShortcutsOpen} onOpenChange={setIsShortcutsOpen} />
    </QueryClientProvider>
  );
}
