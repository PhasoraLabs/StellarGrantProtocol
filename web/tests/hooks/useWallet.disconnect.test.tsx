import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useWallet } from "@/hooks/useWallet";
import { useWalletStore } from "@/lib/store/walletStore";

describe("useWallet disconnect", () => {
  it("clears the wallet state and tears down the adapter", () => {
    const { result } = renderHook(() => useWallet());
    const teardown = vi.fn();

    act(() => {
      useWalletStore.setState({
        address: "GABC",
        walletType: "freighter",
        activeAdapter: { disconnect: teardown } as never,
      });
    });
    expect(result.current.isConnected).toBe(true);

    act(() => result.current.disconnect());

    expect(teardown).toHaveBeenCalledOnce();
    expect(result.current.address).toBeNull();
    expect(result.current.isConnected).toBe(false);
    expect(useWalletStore.getState().walletType).toBeNull();
  });
});
