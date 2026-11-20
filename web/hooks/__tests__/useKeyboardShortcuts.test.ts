import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useKeyboardShortcuts } from "../useKeyboardShortcuts";

describe("useKeyboardShortcuts", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function triggerKeyDown(key: string, target?: HTMLElement) {
    const event = new KeyboardEvent("keydown", { key, bubbles: true });
    if (target) {
      target.dispatchEvent(event);
    } else {
      window.dispatchEvent(event);
    }
  }

  it("fires action on single key shortcut", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "?", description: "Help", action }])
    );

    act(() => {
      triggerKeyDown("?");
    });

    expect(action).toHaveBeenCalledTimes(1);
  });

  it("fires action on multi-key shortcut", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "g g", description: "Grants", action }])
    );

    act(() => {
      triggerKeyDown("g");
      triggerKeyDown("g");
    });

    expect(action).toHaveBeenCalledTimes(1);
  });

  it("resets sequence on invalid key", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "g g", description: "Grants", action }])
    );

    act(() => {
      triggerKeyDown("g");
      triggerKeyDown("x"); // Invalid key
      triggerKeyDown("g");
      triggerKeyDown("g"); // This should match now since it reset
    });

    expect(action).toHaveBeenCalledTimes(1);
  });

  it("resets sequence after 1000ms timeout", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "g g", description: "Grants", action }])
    );

    act(() => {
      triggerKeyDown("g");
      vi.advanceTimersByTime(1050);
      triggerKeyDown("g"); // Too late
    });

    expect(action).not.toHaveBeenCalled();

    act(() => {
      triggerKeyDown("g"); // First g of new sequence
    });

    expect(action).not.toHaveBeenCalled();
  });

  it("does not trigger shortcuts inside input elements", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "f", description: "Focus", action }])
    );

    const input = document.createElement("input");
    document.body.appendChild(input);

    act(() => {
      triggerKeyDown("f", input);
    });

    expect(action).not.toHaveBeenCalled();
    document.body.removeChild(input);
  });

  it("does not trigger shortcuts inside textarea elements", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "a", description: "Action", action }])
    );

    const textarea = document.createElement("textarea");
    document.body.appendChild(textarea);

    act(() => {
      triggerKeyDown("a", textarea);
    });

    expect(action).not.toHaveBeenCalled();
    document.body.removeChild(textarea);
  });

  it("does not trigger shortcuts inside contenteditable elements", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "c", description: "Content", action }])
    );

    const div = document.createElement("div");
    div.contentEditable = "true";
    document.body.appendChild(div);

    act(() => {
      triggerKeyDown("c", div);
    });

    expect(action).not.toHaveBeenCalled();
    document.body.removeChild(div);
  });

  it("does not trigger shortcuts inside select elements", () => {
    const action = vi.fn();
    renderHook(() =>
      useKeyboardShortcuts([{ key: "s", description: "Select", action }])
    );

    const select = document.createElement("select");
    document.body.appendChild(select);

    act(() => {
      triggerKeyDown("s", select);
    });

    expect(action).not.toHaveBeenCalled();
    document.body.removeChild(select);
  });
});
