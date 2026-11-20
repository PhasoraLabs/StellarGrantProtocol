import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import DisputePanelClient from "@/components/dispute/DisputePanelClient";
import { DisputePanel } from "@/components/dispute/DisputePanel";
import { DisputeHistory } from "@/components/dispute/DisputeHistory";

// 1. Mocks
const mockExecute = vi.fn();
const mockReset = vi.fn();

vi.mock("@/hooks/useContractTransaction", () => ({
  useContractTransaction: () => ({
    execute: mockExecute,
    isPending: false,
    isSimulating: false,
    error: null,
    reset: mockReset,
  }),
}));

const mockIsCouncilMember = vi.fn();
const mockResolveDispute = vi.fn();

vi.mock("@/lib/stellar/contract", () => ({
  contractClient: {
    isCouncilMember: (args: unknown) => mockIsCouncilMember(args),
    resolveDispute: (args: unknown) => mockResolveDispute(args),
  },
}));

vi.mock("@/components/wallet/WalletGuard", () => ({
  WalletGuard: ({ children }: { children: React.ReactNode }) => <div data-testid="wallet-guard">{children}</div>,
}));

vi.mock("@/components/milestones/ProofViewer", () => ({
  ProofViewer: ({ proofHash }: { proofHash: string }) => <div data-testid="proof-viewer">Proof: {proofHash}</div>,
}));

const mockApiGet = vi.fn();
vi.mock("@/lib/api", () => ({
  apiGet: (url: string, params?: unknown) => mockApiGet(url, params),
}));

// Mock wallet address
const mockAddress = "GBCOUNCIL1234567890";
vi.mock("@/lib/store/walletStore", () => ({
  useWalletStore: () => ({
    address: mockAddress,
  }),
}));

describe("Dispute Resolution Panel Components", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("DisputeHistory", () => {
    it("renders empty state correctly", () => {
      render(<DisputeHistory history={[]} />);
      expect(screen.getByText("Resolved Disputes (0)")).toBeTruthy();
      expect(screen.getByText("No resolved disputes in history.")).toBeTruthy();
    });

    it("renders resolved disputes table", () => {
      const history = [
        {
          id: "1-0",
          grantTitle: "Project Alpha",
          milestoneIdx: 0,
          milestoneTitle: "Setup Architecture",
          resolution: "payout" as const,
          resolvedAt: "2026-05-29T12:00:00Z",
          fundedAmount: 10000000n,
          token: "native",
        },
      ];

      render(<DisputeHistory history={history} />);
      expect(screen.getByText("Project Alpha")).toBeTruthy();
      expect(screen.getByText("Milestone #1: Setup Architecture")).toBeTruthy();
      expect(screen.getByText("Payout Approved")).toBeTruthy();
    });
  });

  describe("DisputePanel", () => {
    const defaultProps = {
      grantId: "42",
      grantTitle: "DeFi Aggregator",
      milestoneIdx: 1,
      milestoneTitle: "Smart Contract Audit",
      proofHash: "QmHash123",
      contributorArgument: "I completed the audit successfully.",
      funderArgument: "The audit has critical gaps.",
      fundedAmount: 50000000n,
      token: "native",
      priorVotes: { approved: 2, rejected: 1 },
      onResolved: vi.fn(),
    };

    it("renders all dispute information correctly", async () => {
      render(<DisputePanel {...defaultProps} />);
      
      expect(screen.getByText("DeFi Aggregator")).toBeTruthy();
      expect(screen.getByText(/Milestone #2:/)).toBeTruthy();
      expect(screen.getByText("Smart Contract Audit")).toBeTruthy();
      expect(screen.getByTestId("proof-viewer")).toBeTruthy();

      await waitFor(() => {
        expect(screen.getByText("I completed the audit successfully.")).toBeTruthy();
        expect(screen.getByText("The audit has critical gaps.")).toBeTruthy();
      });
    });

    it("requires confirmation prompt before resolving", async () => {
      render(<DisputePanel {...defaultProps} />);
      
      const approveBtn = screen.getByText("✓ Approve Payout to Contributor");
      fireEvent.click(approveBtn);

      expect(screen.getByText("Confirm Resolution Plan")).toBeTruthy();
      expect(screen.getByText(/You are resolving this dispute in favor of the/)).toBeTruthy();
      expect(screen.getByText("Contributor")).toBeTruthy();

      const confirmBtn = screen.getByText("Confirm");
      
      mockResolveDispute.mockResolvedValue({
        method: "milestone_resolve_dispute",
        args: [],
      });
      mockExecute.mockResolvedValue("mock_tx_hash");

      fireEvent.click(confirmBtn);

      await waitFor(() => {
        expect(mockResolveDispute).toHaveBeenCalledWith({
          grantId: "42",
          milestoneIdx: 1,
          approvePayout: true,
          councilAddress: mockAddress,
        });
        expect(mockExecute).toHaveBeenCalled();
      });
    });

    it("handles refund confirmation flow correctly", async () => {
      render(<DisputePanel {...defaultProps} />);
      
      const refundBtn = screen.getByText("✗ Refund Funders");
      fireEvent.click(refundBtn);

      expect(screen.getByText("Confirm Resolution Plan")).toBeTruthy();
      expect(screen.getByText("Funders")).toBeTruthy();

      const confirmBtn = screen.getByText("Confirm");
      
      mockResolveDispute.mockResolvedValue({
        method: "milestone_resolve_dispute",
        args: [],
      });
      mockExecute.mockResolvedValue("mock_tx_hash");

      fireEvent.click(confirmBtn);

      await waitFor(() => {
        expect(mockResolveDispute).toHaveBeenCalledWith({
          grantId: "42",
          milestoneIdx: 1,
          approvePayout: false,
          councilAddress: mockAddress,
        });
        expect(mockExecute).toHaveBeenCalled();
      });
    });
  });

  describe("DisputePanelClient", () => {
    it("renders Restricted Access screen for non-council members", async () => {
      mockIsCouncilMember.mockResolvedValue(false);

      render(<DisputePanelClient />);

      await waitFor(() => {
        expect(screen.getByText("Restricted Access")).toBeTruthy();
        expect(screen.getByText("This panel is only accessible to StellarGrant Council members.")).toBeTruthy();
      });
    });

    it("renders dashboard and active disputes list for council members", async () => {
      mockIsCouncilMember.mockResolvedValue(true);
      mockApiGet.mockImplementation((url: string) => {
        if (url === "/disputes?status=open") {
          return Promise.resolve([
            {
              grantId: "101",
              grantTitle: "SDK Refactoring",
              milestoneIdx: 0,
              milestoneTitle: "Initial Draft",
              proofHash: "QmProof",
              contributorArgument: "Done.",
              funderArgument: "No.",
              fundedAmount: 15000000n,
              token: "native",
              priorVotes: { approved: 0, rejected: 0 },
            },
          ]);
        }
        if (url === "/disputes?status=resolved") {
          return Promise.resolve([]);
        }
        return Promise.resolve([]);
      });

      render(<DisputePanelClient />);

      await waitFor(() => {
        expect(screen.getByText("Council Dispute Panel")).toBeTruthy();
        expect(screen.getByText("SDK Refactoring")).toBeTruthy();
        expect(screen.getByText("Done.")).toBeTruthy();
      });
    });
  });
});
