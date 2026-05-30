/**
 * useUserPreferences — Issue #386.
 *
 * Typed access to the user's personal preferences (notifications, display,
 * developer options) persisted to `localStorage` under `sg-preferences`.
 *
 * Reads are SSR-safe (returns defaults on the server) and writes are
 * partial — pass only the keys you want to change.
 */

"use client";

import { useCallback, useSyncExternalStore } from "react";

export const STORAGE_KEY = "sg-preferences";

export type AddressFormat = "short" | "medium" | "full";
export type XlmDecimals = 2 | 4 | 7;
export type DateFormat = "relative" | "absolute";

export interface UserPreferences {
  // Notifications
  notifyOnFunding: boolean;
  notifyOnMilestoneSubmit: boolean;
  notifyOnVote: boolean;
  notifyOnPayout: boolean;

  // Display
  addressFormat: AddressFormat;
  xlmDecimals: XlmDecimals;
  dateFormat: DateFormat;

  // Developer
  showTxHashes: boolean;
  debugMode: boolean;
}

export const DEFAULT_PREFERENCES: UserPreferences = {
  notifyOnFunding: true,
  notifyOnMilestoneSubmit: true,
  notifyOnVote: true,
  notifyOnPayout: true,

  addressFormat: "short",
  xlmDecimals: 7,
  dateFormat: "relative",

  showTxHashes: false,
  debugMode: false,
};

const PREF_CHANGED_EVENT = "sg:preferences-changed";

/**
 * Coerce arbitrary JSON to a valid `UserPreferences` by falling back to defaults
 * for any missing or out-of-range value.
 */
export function mergePreferences(raw: unknown): UserPreferences {
  if (!raw || typeof raw !== "object") return { ...DEFAULT_PREFERENCES };
  const r = raw as Partial<Record<keyof UserPreferences, unknown>>;
  const bool = (v: unknown, fallback: boolean): boolean =>
    typeof v === "boolean" ? v : fallback;

  const addressFormat: AddressFormat =
    r.addressFormat === "short" || r.addressFormat === "medium" || r.addressFormat === "full"
      ? r.addressFormat
      : DEFAULT_PREFERENCES.addressFormat;

  const xlmDecimals: XlmDecimals =
    r.xlmDecimals === 2 || r.xlmDecimals === 4 || r.xlmDecimals === 7
      ? r.xlmDecimals
      : DEFAULT_PREFERENCES.xlmDecimals;

  const dateFormat: DateFormat =
    r.dateFormat === "relative" || r.dateFormat === "absolute"
      ? r.dateFormat
      : DEFAULT_PREFERENCES.dateFormat;

  return {
    notifyOnFunding: bool(r.notifyOnFunding, DEFAULT_PREFERENCES.notifyOnFunding),
    notifyOnMilestoneSubmit: bool(
      r.notifyOnMilestoneSubmit,
      DEFAULT_PREFERENCES.notifyOnMilestoneSubmit,
    ),
    notifyOnVote: bool(r.notifyOnVote, DEFAULT_PREFERENCES.notifyOnVote),
    notifyOnPayout: bool(r.notifyOnPayout, DEFAULT_PREFERENCES.notifyOnPayout),
    addressFormat,
    xlmDecimals,
    dateFormat,
    showTxHashes: bool(r.showTxHashes, DEFAULT_PREFERENCES.showTxHashes),
    debugMode: bool(r.debugMode, DEFAULT_PREFERENCES.debugMode),
  };
}

/** SSR-safe read from localStorage. */
export function readPreferences(): UserPreferences {
  if (typeof window === "undefined") return { ...DEFAULT_PREFERENCES };
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_PREFERENCES };
    return mergePreferences(JSON.parse(raw));
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

function persistPreferences(prefs: UserPreferences): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
    window.dispatchEvent(new CustomEvent(PREF_CHANGED_EVENT));
  } catch {
    // Storage may be unavailable (Safari private mode, quota); silently degrade.
  }
}

export interface UseUserPreferencesResult {
  preferences: UserPreferences;
  /** Update one or more keys. */
  setPreferences: (patch: Partial<UserPreferences>) => void;
  /** Wipe all custom preferences and revert to the defaults. */
  reset: () => void;
}

/** Memoised snapshot so React's bail-out comparison is stable. */
let cachedSnapshot: UserPreferences = DEFAULT_PREFERENCES;
let cachedSnapshotRaw: string | null | undefined = undefined;

function getSnapshot(): UserPreferences {
  if (typeof window === "undefined") return DEFAULT_PREFERENCES;
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (raw === cachedSnapshotRaw) return cachedSnapshot;
  cachedSnapshotRaw = raw;
  cachedSnapshot = readPreferences();
  return cachedSnapshot;
}

function subscribe(callback: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  const onChange = () => {
    // Invalidate the snapshot cache so the next getSnapshot reads fresh.
    cachedSnapshotRaw = undefined;
    callback();
  };
  window.addEventListener(PREF_CHANGED_EVENT, onChange);
  window.addEventListener("storage", onChange);
  return () => {
    window.removeEventListener(PREF_CHANGED_EVENT, onChange);
    window.removeEventListener("storage", onChange);
  };
}

export function useUserPreferences(): UseUserPreferencesResult {
  const preferences = useSyncExternalStore(
    subscribe,
    getSnapshot,
    () => DEFAULT_PREFERENCES,
  );

  const setPreferences = useCallback((patch: Partial<UserPreferences>) => {
    const current = typeof window !== "undefined" ? readPreferences() : DEFAULT_PREFERENCES;
    persistPreferences({ ...current, ...patch });
  }, []);

  const reset = useCallback(() => {
    if (typeof window !== "undefined") {
      try {
        window.localStorage.removeItem(STORAGE_KEY);
      } catch {
        // ignore
      }
      window.dispatchEvent(new CustomEvent(PREF_CHANGED_EVENT));
    }
  }, []);

  return { preferences, setPreferences, reset };
}
