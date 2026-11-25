import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useGrantDraft } from "@/hooks/useGrantDraft";
import { TOAST_EVENT, type ToastOptions } from "@/lib/toast";

const STORAGE_KEY = "sg-grant-draft";

describe("useGrantDraft discard confirmation and undo", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("opens and cancels discard confirmation without removing the draft", () => {
    const { result } = renderHook(() => useGrantDraft());

    act(() => {
      result.current.saveDraft({ title: "Stellar Grant Draft", currentStep: 2 });
    });

    expect(result.current.hasDraft).toBe(true);
    expect(result.current.isDiscardConfirmOpen).toBe(false);

    act(() => {
      result.current.requestDiscardDraft();
    });

    expect(result.current.isDiscardConfirmOpen).toBe(true);
    expect(result.current.hasDraft).toBe(true);
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull();

    act(() => {
      result.current.cancelDiscardDraft();
    });

    expect(result.current.isDiscardConfirmOpen).toBe(false);
    expect(result.current.hasDraft).toBe(true);
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull();
  });

  it("discards a hydrated draft on confirmation, emits an undo toast, and restores on undo", async () => {
    const dispatchedToasts: ToastOptions[] = [];
    const handleToast = (event: Event) => {
      dispatchedToasts.push((event as CustomEvent<ToastOptions>).detail);
    };
    window.addEventListener(TOAST_EVENT, handleToast);

    const savedAt = Date.now() - 120_000;
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        data: { title: "Important Proposal", currentStep: 3 },
        savedAt,
      })
    );

    const { result } = renderHook(() => useGrantDraft());

    await waitFor(() => {
      expect(result.current.hasDraft).toBe(true);
    });
    expect(result.current.draftAge).toBe("Saved 2 minutes ago");

    act(() => {
      result.current.requestDiscardDraft();
    });

    act(() => {
      result.current.confirmDiscardDraft();
    });

    expect(result.current.isDiscardConfirmOpen).toBe(false);
    expect(result.current.hasDraft).toBe(false);
    expect(result.current.canUndoDiscard).toBe(true);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();

    expect(dispatchedToasts).toHaveLength(1);
    expect(dispatchedToasts[0].title).toBe("Draft discarded");
    expect(dispatchedToasts[0].action?.label).toBe("Undo");
    expect(typeof dispatchedToasts[0].action?.onClick).toBe("function");

    act(() => {
      dispatchedToasts[0].action?.onClick?.();
    });

    expect(result.current.hasDraft).toBe(true);
    expect(result.current.draft).toEqual({
      title: "Important Proposal",
      currentStep: 3,
    });
    expect(result.current.draftAge).toBe("Saved 2 minutes ago");
    expect(result.current.canUndoDiscard).toBe(false);
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull();
    expect(dispatchedToasts[1]?.title).toBe("Draft restored");

    // Repeating undo is a safe no-op
    act(() => {
      result.current.undoDiscardDraft();
    });
    expect(dispatchedToasts).toHaveLength(2);

    window.removeEventListener(TOAST_EVENT, handleToast);
  });
});
