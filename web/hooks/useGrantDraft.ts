"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import type { GrantFormData } from "@/lib/schemas/grant";
import { toast, toastSuccess } from "@/lib/toast";

const STORAGE_KEY = "sg-grant-draft";
const MAX_DRAFT_AGE_MS = 72 * 60 * 60 * 1000;

export type GrantDraftData = Partial<GrantFormData> & { currentStep?: number };

interface SavedDraft {
  data: GrantDraftData;
  savedAt: number;
}

export interface UseGrantDraftResult {
  draft: GrantDraftData | null;
  saveDraft: (data: GrantDraftData) => void;
  clearDraft: () => void;
  requestDiscardDraft: () => void;
  confirmDiscardDraft: () => void;
  cancelDiscardDraft: () => void;
  undoDiscardDraft: () => void;
  isDiscardConfirmOpen: boolean;
  canUndoDiscard: boolean;
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
  const [draft, setDraft] = useState<GrantDraftData | null>(null);
  const [draftAge, setDraftAge] = useState("");
  const [isDiscardConfirmOpen, setIsDiscardConfirmOpen] = useState(false);
  const [canUndoDiscard, setCanUndoDiscard] = useState(false);
  const lastDiscardedRef = useRef<SavedDraft | null>(null);

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

      queueMicrotask(() => {
        setDraft(saved.data);
        updateDraftAge(saved.savedAt);
      });

      const interval = setInterval(() => updateDraftAge(saved.savedAt), 60_000);
      return () => clearInterval(interval);
    } catch {
      localStorage.removeItem(STORAGE_KEY);
    }
  }, [updateDraftAge]);

  const saveDraft = useCallback(
    (data: GrantDraftData) => {
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
    setIsDiscardConfirmOpen(false);
  }, []);

  const undoDiscardDraft = useCallback(() => {
    const discarded = lastDiscardedRef.current;
    if (!discarded) return;

    lastDiscardedRef.current = null;
    setCanUndoDiscard(false);

    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(discarded));
    } catch {
      // localStorage full or unavailable
    }

    setDraft(discarded.data);
    updateDraftAge(discarded.savedAt);
    toastSuccess("Draft restored", {
      description: "Your discarded grant draft has been recovered.",
    });
  }, [updateDraftAge]);

  const requestDiscardDraft = useCallback(() => {
    setIsDiscardConfirmOpen(true);
  }, []);

  const cancelDiscardDraft = useCallback(() => {
    setIsDiscardConfirmOpen(false);
  }, []);

  const confirmDiscardDraft = useCallback(() => {
    let snapshot: SavedDraft | null = null;
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        snapshot = JSON.parse(raw) as SavedDraft;
      }
    } catch {
      // ignore parse or storage errors
    }

    if (!snapshot && draft !== null) {
      snapshot = { data: draft, savedAt: Date.now() };
    }

    if (snapshot) {
      lastDiscardedRef.current = snapshot;
      setCanUndoDiscard(true);
    }

    clearDraft();

    if (snapshot) {
      toast({
        title: "Draft discarded",
        description: "Your saved grant draft was removed.",
        variant: "info",
        duration: 8000,
        action: {
          label: "Undo",
          onClick: undoDiscardDraft,
        },
      });
    }
  }, [clearDraft, draft, undoDiscardDraft]);

  return {
    draft,
    saveDraft,
    clearDraft,
    requestDiscardDraft,
    confirmDiscardDraft,
    cancelDiscardDraft,
    undoDiscardDraft,
    isDiscardConfirmOpen,
    canUndoDiscard,
    hasDraft: draft !== null,
    draftAge,
  };
}
