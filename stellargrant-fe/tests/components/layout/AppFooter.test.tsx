import { render, screen, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { AppFooter } from "../../../components/layout/AppFooter";
import { useNetworkStatus } from "../../../hooks/useNetworkStatus";
import { getHorizonClient } from "../../../lib/stellar/client";

vi.mock("../../../hooks/useNetworkStatus", () => ({
  useNetworkStatus: vi.fn(),
}));

vi.mock("../../../lib/stellar/client", () => ({
  getHorizonClient: vi.fn(),
}));

describe("AppFooter", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(useNetworkStatus).mockReturnValue({ isOnline: true } as ReturnType<typeof useNetworkStatus>);
    vi.mocked(getHorizonClient).mockReturnValue({
      ledgers: () => ({
        order: () => ({
          limit: () => ({
            call: vi.fn().mockResolvedValue({
              records: [{ sequence: 1234567 }],
            }),
          }),
        }),
      }),
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.clearAllTimers();
  });

  it("renders logo and tagline", () => {
    render(<AppFooter />);
    expect(screen.getByText("STELLAR·GRANT")).toBeInTheDocument();
    expect(screen.getByText("Decentralized milestone-based grant management on Stellar.")).toBeInTheDocument();
  });

  it("renders footer navigation links", () => {
    render(<AppFooter />);
    expect(screen.getByRole("link", { name: "Explore" })).toHaveAttribute("href", "/grants");
    expect(screen.getByRole("link", { name: "Create" })).toHaveAttribute("href", "/grants/create");
    expect(screen.getByRole("link", { name: "Dashboard" })).toHaveAttribute("href", "/dashboard");
    expect(screen.getByRole("link", { name: "Review" })).toHaveAttribute("href", "/review");
    expect(screen.getByRole("link", { name: "Leaderboard" })).toHaveAttribute("href", "/leaderboard");
    expect(screen.getByRole("link", { name: "Settings" })).toHaveAttribute("href", "/settings");
  });

  it("renders network status and latest ledger", async () => {
    render(<AppFooter />);
    
    // Allow the async fetchLedger to run
    await act(async () => {
      vi.advanceTimersByTime(100);
    });

    expect(screen.getByText(/Block: 1,234,567/)).toBeInTheDocument();
    expect(screen.getByText("Built with ❤ on Stellar")).toBeInTheDocument();
    
    // Expect online dot since useNetworkStatus is mocked to true
    expect(screen.getByLabelText("Online")).toBeInTheDocument();
  });

  it("shows offline status when network is offline", () => {
    vi.mocked(useNetworkStatus).mockReturnValue({ isOnline: false } as ReturnType<typeof useNetworkStatus>);
    render(<AppFooter />);
    expect(screen.getByLabelText("Offline")).toBeInTheDocument();
  });

  it("renders copyright and github links", () => {
    render(<AppFooter />);
    const currentYear = new Date().getFullYear();
    expect(screen.getByText(`© ${currentYear} StellarGrant Protocol`)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "MIT License" })).toHaveAttribute("href", "https://github.com/org/repo/blob/main/LICENSE");
    expect(screen.getByRole("link", { name: "GitHub Repository" })).toHaveAttribute("href", "https://github.com/your-org/stellargrant-fe");
  });
});
