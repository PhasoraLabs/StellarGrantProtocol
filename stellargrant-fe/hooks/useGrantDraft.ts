"use client";

import { useEffect, useState, useCallback } from "react";
import type { CreateGrantFormValues } from "@/components/grants/CreateGrantForm/types";

const STORAGE_KEY = "sg-grant-draft";
const MAX_DRAFT_AGE_MS = 72 * 60 * 60 * 1000;

interface SavedDraft {
  data: Partial<CreateGrantFormValues> & { currentStep?: number };
  savedAt: number;
}

interface UseGrantDraftResult {
  draft: (Partial<CreateGrantFormValues> & { currentStep?: number }) | null;
  saveDraft: (data: Partial<CreateGrantFormValues> & { currentStep?: number }) => void;
  clearDraft: () => void;
  hasDraft: boolean;
  draftAge: string;
}

function formatDraftAge(savedAt: number): string {
  const diffMs = Date.now() - savedAt;
  const diffSec = Math.floor(diffMs / 1000);
  if (diffSec < 60) return "Saved less than a minute ago";
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `Saved ${diffMin} minute${diffMin === 1 ? "" : "s"} ago`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `Saved ${diffHours} hour${diffHours === 1 ? "" : "s"} ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `Saved ${diffDays} day${diffDays === 1 ? "" : "s"} ago`;
}

export function useGrantDraft(): UseGrantDraftResult {
  const [draft, setDraft] = useState<(Partial<CreateGrantFormValues> & { currentStep?: number }) | null>(null);
  const [draftAge, setDraftAge] = useState("");

  const updateDraftAge = useCallback((savedAt: number) => {
    setDraftAge(formatDraftAge(savedAt));
  }, []);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;

      const saved: SavedDraft = JSON.parse(raw);

      if (Date.now() - saved.savedAt > MAX_DRAFT_AGE_MS) {
        localStorage.removeItem(STORAGE_KEY);
        return;
      }

      setDraft(saved.data);
      updateDraftAge(saved.savedAt);

      const interval = setInterval(() => updateDraftAge(saved.savedAt), 60_000);
      return () => clearInterval(interval);
    } catch {
      localStorage.removeItem(STORAGE_KEY);
    }
  }, [updateDraftAge]);

  const saveDraft = useCallback(
    (data: Partial<CreateGrantFormValues> & { currentStep?: number }) => {
      const saved: SavedDraft = {
        data,
        savedAt: Date.now(),
      };
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(saved));
        setDraft(data);
        updateDraftAge(saved.savedAt);
      } catch {
        // localStorage full or unavailable
      }
    },
    [updateDraftAge]
  );

  const clearDraft = useCallback(() => {
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // ignore
    }
    setDraft(null);
    setDraftAge("");
  }, []);

  return {
    draft,
    saveDraft,
    clearDraft,
    hasDraft: draft !== null,
    draftAge,
  };
}
