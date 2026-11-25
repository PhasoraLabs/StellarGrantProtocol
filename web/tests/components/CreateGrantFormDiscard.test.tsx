import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within, act } from "@testing-library/react";
import { CreateGrantForm } from "@/components/grants/CreateGrantForm";
import { TOAST_EVENT, type ToastOptions } from "@/lib/toast";

vi.mock("@/lib/store", () => ({
  useWalletStore: vi.fn(<T,>(selector: (s: { address: string }) => T) =>
    selector({ address: "GASUAQLFSMVODTPT4GP55RASDBE6ZCN5J76XLO5LRQTNJDNTTNK2NRQK" })
  ),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
  }),
}));

vi.mock("@/hooks/useContractTransaction", () => ({
  useContractTransaction: () => ({
    execute: vi.fn(),
    isPending: false,
    isSimulating: false,
    isSuccess: false,
    txHash: null,
    error: null,
    reset: vi.fn(),
  }),
}));

vi.mock("@/lib/stellar/contract", () => ({
  contractClient: {
    grantCreate: vi.fn().mockResolvedValue({
      method: "grant_create",
      args: [],
    }),
  },
}));

const STORAGE_KEY = "sg-grant-draft";

describe("CreateGrantForm draft discard confirmation", () => {
  beforeEach(() => {
    localStorage.clear();
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        data: { title: "Saved Grant Draft Title", currentStep: 2 },
        savedAt: Date.now() - 60_000,
      })
    );
  });

  it("shows confirmation dialog before discarding and supports undo via toast", async () => {
    const toasts: ToastOptions[] = [];
    const onToast = (event: Event) => {
      toasts.push((event as CustomEvent<ToastOptions>).detail);
    };
    window.addEventListener(TOAST_EVENT, onToast);

    render(<CreateGrantForm />);

    await waitFor(() => {
      expect(screen.getByText(/You have a saved draft from/i)).toBeInTheDocument();
    });

    // Click Discard Draft on banner -> opens confirmation dialog without deleting draft
    fireEvent.click(screen.getByRole("button", { name: "Discard Draft" }));
    expect(screen.getByRole("dialog", { name: "Discard saved draft?" })).toBeInTheDocument();
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull();

    // Cancel keeps draft intact
    fireEvent.click(screen.getByRole("button", { name: "Keep Draft" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByText(/You have a saved draft from/i)).toBeInTheDocument();

    // Confirm discard removes draft and emits undo toast
    fireEvent.click(screen.getByRole("button", { name: "Discard Draft" }));
    const dialog = screen.getByRole("dialog", { name: "Discard saved draft?" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Discard Draft" }));

    await waitFor(() => {
      expect(screen.queryByText(/You have a saved draft from/i)).toBeNull();
    });
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
    expect(toasts).toHaveLength(1);
    expect(toasts[0].action?.label).toBe("Undo");

    // Triggering toast Undo action brings the draft banner back
    act(() => {
      toasts[0].action?.onClick?.();
    });

    await waitFor(() => {
      expect(screen.getByText(/You have a saved draft from/i)).toBeInTheDocument();
    });
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull();

    window.removeEventListener(TOAST_EVENT, onToast);
  });
});
