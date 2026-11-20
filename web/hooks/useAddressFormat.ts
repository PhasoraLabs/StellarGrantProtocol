/**
 * useAddressFormat — Issue #386.
 *
 * Returns a truncation function derived from the user's `addressFormat`
 * preference:
 *   short  → first-6 … last-4   (default)
 *   medium → first-8 … last-6
 *   full   → no truncation
 */

"use client";

import { useCallback } from "react";
import { useUserPreferences, type AddressFormat } from "@/hooks/useUserPreferences";

/** Pure truncator — exposed for non-React callers and easy testing. */
export function formatAddressFor(format: AddressFormat, address: string): string {
  if (!address) return address;
  if (format === "full") return address;
  if (format === "medium") {
    return address.length > 14 ? `${address.slice(0, 8)}…${address.slice(-6)}` : address;
  }
  return address.length > 10 ? `${address.slice(0, 6)}…${address.slice(-4)}` : address;
}

export function useAddressFormat(): (address: string) => string {
  const { preferences } = useUserPreferences();
  return useCallback(
    (address: string) => formatAddressFor(preferences.addressFormat, address),
    [preferences.addressFormat],
  );
}
