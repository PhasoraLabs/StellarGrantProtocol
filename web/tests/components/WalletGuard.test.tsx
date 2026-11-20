import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { Grant } from "@/types";
import { WalletGuard, isAuthorizedForRole } from "@/components/wallet/WalletGuard";

// --- Mocks -----------------------------------------------------------------

const walletState = {
  address: null as string | null,
  isConnected: false,
  isConnecting: false,
};

vi.mock("@/hooks/useWallet", () => ({
  useWallet: () => walletState,
}));

const grantState = {
  data: null as { grant: Grant } | null,
  isLoading: false,
};

vi.mock("@/hooks/useGrant", () => ({
  useGrant: () => grantState,
}));

vi.mock("@/components/wallet/WalletConnect", () => ({
  WalletConnect: () => <button>Connect Wallet</button>,
}));

function makeGrant(overrides: Partial<Grant> = {}): Grant {
  return {
    id: "1",
    owner: "OWNER_ADDR",
    recipient: "RECIPIENT_ADDR",
    title: "t",
    description: "d",
    budget: 1000n,
    funded: 0n,
    deadline: 0n,
    status: 1,
    milestones: 0,
    reviewers: ["REVIEWER_ADDR"],
    created_at: 0n,
    ...overrides,
  };
}

beforeEach(() => {
  walletState.address = null;
  walletState.isConnected = false;
  walletState.isConnecting = false;
  grantState.data = null;
  grantState.isLoading = false;
});

describe("isAuthorizedForRole", () => {
  const grant = makeGrant();
  it("checks owner / reviewer / contributor", () => {
    expect(isAuthorizedForRole("owner", grant, "OWNER_ADDR")).toBe(true);
    expect(isAuthorizedForRole("owner", grant, "SOMEONE")).toBe(false);
    expect(isAuthorizedForRole("reviewer", grant, "REVIEWER_ADDR")).toBe(true);
    expect(isAuthorizedForRole("reviewer", grant, "SOMEONE")).toBe(false);
    expect(isAuthorizedForRole("contributor", grant, "RECIPIENT_ADDR")).toBe(true);
    expect(isAuthorizedForRole("any", grant, "ANY")).toBe(true);
  });
  it("rejects a null address", () => {
    expect(isAuthorizedForRole("owner", grant, null)).toBe(false);
  });
});

describe("WalletGuard rendering", () => {
  it("shows a shimmer while the session is restoring", () => {
    walletState.isConnecting = true;
    const { container } = render(
      <WalletGuard>
        <p>secret</p>
      </WalletGuard>,
    );
    expect(container.querySelector(".shimmer")).toBeTruthy();
    expect(screen.queryByText("secret")).toBeNull();
  });

  it("shows the connect card with WalletConnect when disconnected", () => {
    render(
      <WalletGuard>
        <p>secret</p>
      </WalletGuard>,
    );
    expect(screen.getByText("Wallet Required")).toBeTruthy();
    expect(screen.getByText("Connect Wallet")).toBeTruthy();
    expect(screen.queryByText("secret")).toBeNull();
  });

  it("renders a custom fallback when provided and disconnected", () => {
    render(
      <WalletGuard fallback={<p>Sign in to fund this grant</p>}>
        <p>secret</p>
      </WalletGuard>,
    );
    expect(screen.getByText("Sign in to fund this grant")).toBeTruthy();
  });

  it("renders children when connected with no role requirement", () => {
    walletState.isConnected = true;
    walletState.address = "ANY_ADDR";
    render(
      <WalletGuard>
        <p>secret</p>
      </WalletGuard>,
    );
    expect(screen.getByText("secret")).toBeTruthy();
  });

  it("blocks non-owners for requiredRole=owner", () => {
    walletState.isConnected = true;
    walletState.address = "NOT_OWNER";
    grantState.data = { grant: makeGrant() };
    render(
      <WalletGuard requiredRole="owner" grantId="1">
        <p>secret</p>
      </WalletGuard>,
    );
    expect(screen.getByText("Access Restricted")).toBeTruthy();
    expect(screen.queryByText("secret")).toBeNull();
  });

  it("allows the owner for requiredRole=owner", () => {
    walletState.isConnected = true;
    walletState.address = "OWNER_ADDR";
    grantState.data = { grant: makeGrant() };
    render(
      <WalletGuard requiredRole="owner" grantId="1">
        <p>secret</p>
      </WalletGuard>,
    );
    expect(screen.getByText("secret")).toBeTruthy();
  });

  it("blocks non-reviewers for requiredRole=reviewer", () => {
    walletState.isConnected = true;
    walletState.address = "NOT_REVIEWER";
    grantState.data = { grant: makeGrant() };
    render(
      <WalletGuard requiredRole="reviewer" grantId="1">
        <p>secret</p>
      </WalletGuard>,
    );
    expect(screen.getByText("Access Restricted")).toBeTruthy();
  });

  it("shows a shimmer while grant data loads for a role check", () => {
    walletState.isConnected = true;
    walletState.address = "OWNER_ADDR";
    grantState.isLoading = true;
    const { container } = render(
      <WalletGuard requiredRole="owner" grantId="1">
        <p>secret</p>
      </WalletGuard>,
    );
    expect(container.querySelector(".shimmer")).toBeTruthy();
  });
});
