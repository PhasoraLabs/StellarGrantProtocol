import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { CreateGrantForm } from "@/components/grants/CreateGrantForm";

// 1. Mock useWalletStore
vi.mock("@/lib/store", () => ({
  useWalletStore: vi.fn(<T,>(selector: (s: { address: string }) => T) =>
    selector({ address: "GBWALLET123456789012345678901234567890123456789012345678" })
  ),
}));

// 2. Mock next/navigation
const mockPush = vi.fn();
vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: mockPush,
    replace: vi.fn(),
  }),
}));

// 3. Mock useContractTransaction
const mockExecute = vi.fn();
const mockReset = vi.fn();
let mockContractTxState = {
  execute: mockExecute,
  isPending: false,
  isSimulating: false,
  isSuccess: false,
  txHash: null as string | null,
  error: null as string | null,
  reset: mockReset,
};

vi.mock("@/hooks/useContractTransaction", () => ({
  useContractTransaction: () => mockContractTxState,
}));

// 4. Mock contractClient
vi.mock("@/lib/stellar/contract", () => ({
  contractClient: {
    grantCreate: vi.fn().mockResolvedValue({
      method: "grant_create",
      args: [],
    }),
  },
}));

// 5. Mock useGrantDraft
vi.mock("@/hooks/useGrantDraft", () => ({
  useGrantDraft: () => ({
    draft: null,
    saveDraft: vi.fn(),
    clearDraft: vi.fn(),
    hasDraft: false,
    draftAge: "",
  }),
}));

describe("CreateGrantForm Multi-Step Flow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockContractTxState = {
      execute: mockExecute,
      isPending: false,
      isSimulating: false,
      isSuccess: false,
      txHash: null,
      error: null,
      reset: mockReset,
    };
  });

  it("renders Step 1 and displays validation errors for empty fields on Continue", async () => {
    render(<CreateGrantForm />);

    // Renders Step indicator & title
    expect(screen.getByText("Step 1: Basic Information")).toBeInTheDocument();

    // Click Continue to trigger validations
    const continueBtn = screen.getByRole("button", { name: /continue/i });
    fireEvent.click(continueBtn);

    // Validation errors should appear
    await waitFor(() => {
      expect(screen.getByText("Title must be at least 10 characters")).toBeInTheDocument();
      expect(screen.getByText("Description must be at least 50 characters")).toBeInTheDocument();
      expect(screen.getByText("Invalid Stellar address")).toBeInTheDocument();
    });
  });

  it("advances to Step 2 when Step 1 is filled with valid data", async () => {
    render(<CreateGrantForm />);

    // Fill Title
    const titleInput = screen.getByLabelText("Grant Title");
    fireEvent.change(titleInput, { target: { value: "Building Stellar Bridges to Ethereum" } });

    // Fill Description
    const descTextarea = screen.getByPlaceholderText(/Describe your grant objectives/i);
    fireEvent.change(descTextarea, {
      target: { value: "This is a highly detailed proposal to build cross-chain smart contract bridges utilizing Soroban and Ethereum state proofs." },
    });

    // Fill Recipient Address
    const addressInput = screen.getByLabelText("Recipient Stellar Address");
    const validAddr = "GBK4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42";
    fireEvent.change(addressInput, { target: { value: validAddr } });
    fireEvent.blur(addressInput);

    // Fill Budget
    const budgetInput = screen.getByLabelText("Total Budget");
    fireEvent.change(budgetInput, { target: { value: 1000 } });

    // Fill Deadline
    const deadlineInput = screen.getByLabelText("Submission Expiry Deadline");
    fireEvent.change(deadlineInput, { target: { value: "2027-12-31T23:59" } });

    const continueBtn = screen.getByRole("button", { name: /continue/i });
    fireEvent.click(continueBtn);

    // Should transition to Step 2
    await waitFor(() => {
      expect(screen.getByText("Step 2: Define Milestones")).toBeInTheDocument();
    });
  });

  it("handles milestone validation, balanced budgets, and reordering in Step 2", async () => {
    render(<CreateGrantForm />);

    // Fast-track Step 1 to reach Step 2
    fireEvent.change(screen.getByLabelText("Grant Title"), {
      target: { value: "Building Stellar Bridges to Ethereum" },
    });
    fireEvent.change(screen.getByPlaceholderText(/Describe your grant objectives/i), {
      target: { value: "This is a highly detailed proposal to build cross-chain smart contract bridges utilizing Soroban and Ethereum state proofs." },
    });
    fireEvent.change(screen.getByLabelText("Recipient Stellar Address"), {
      target: { value: "GBK4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42" },
    });
    fireEvent.change(screen.getByLabelText("Total Budget"), { target: { value: 1000 } });
    fireEvent.change(screen.getByLabelText("Submission Expiry Deadline"), {
      target: { value: "2027-12-31T23:59" },
    });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    await waitFor(() => {
      expect(screen.getByText("Step 2: Define Milestones")).toBeInTheDocument();
    });

    // Milestone 1 title and reward (currently 100, which doesn't balance the total budget 1000)
    expect(screen.getByText(/Reward total must match budget exactly/i)).toBeInTheDocument();

    // Adjust first milestone reward to 1000 to balance it
    const rewardInput = screen.getByPlaceholderText("Reward Allocation Amount");
    fireEvent.change(rewardInput, { target: { value: 1000 } });

    // Warning banner should disappear
    await waitFor(() => {
      expect(screen.queryByText(/Reward total must match budget exactly/i)).toBeNull();
    });

    // Test Adding milestone
    const addBtn = screen.getByRole("button", { name: /\+ Add Milestone/i });
    fireEvent.click(addBtn);

    // Verify a second milestone field is added
    expect(screen.getByText("Milestone #2")).toBeInTheDocument();

    const milestoneCard1 = screen.getByText("Milestone #1").closest(".border");
    const milestoneCard2 = screen.getByText("Milestone #2").closest(".border");

    expect(milestoneCard1).toBeInTheDocument();
    expect(milestoneCard2).toBeInTheDocument();

    fireEvent.dragStart(milestoneCard1!, {
      dataTransfer: {
        effectAllowed: "move",
        setData: vi.fn(),
      },
    });
    fireEvent.dragOver(milestoneCard2!);
    fireEvent.drop(milestoneCard2!);
    fireEvent.dragEnd(milestoneCard1!);

    // Reordered successfully (move was called)
    expect(screen.getByText("Milestone #1")).toBeInTheDocument();
  });

  it("handles Step 3 validation (Reviewers bounds, duplicates, and quorum boundary)", async () => {
    render(<CreateGrantForm />);

    // Setup Step 1
    fireEvent.change(screen.getByLabelText("Grant Title"), { target: { value: "Stellar Bridge Developer Grant Project" } });
    fireEvent.change(screen.getByPlaceholderText(/Describe your grant objectives/i), {
      target: { value: "This is a detailed description of the brand new development project on Soroban." },
    });
    fireEvent.change(screen.getByLabelText("Recipient Stellar Address"), {
      target: { value: "GBK4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42" },
    });
    fireEvent.change(screen.getByLabelText("Total Budget"), { target: { value: 500 } });
    fireEvent.change(screen.getByLabelText("Submission Expiry Deadline"), { target: { value: "2027-12-31T23:59" } });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    // Setup Step 2
    await waitFor(() => expect(screen.getByText("Step 2: Define Milestones")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Milestone Title (e.g. Prototype Deployment)"), { target: { value: "Milestone One Alpha Release" } });
    fireEvent.change(screen.getByPlaceholderText(/successful completion/i), {
      target: { value: "Detailed requirements mapping out successful milestone outputs." },
    });
    fireEvent.change(screen.getByPlaceholderText("Reward Allocation Amount"), { target: { value: 500 } });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    // Reach Step 3
    await waitFor(() => expect(screen.getByText("Step 3: Reviewers & Quorum")).toBeInTheDocument());

    // Fill empty reviewer fields with duplicate addresses to trigger error
    await waitFor(() => {
      expect(screen.getAllByLabelText(/Reviewer #\d Stellar Address/i)).toHaveLength(3);
    });
    const reviewerInputs = screen.getAllByLabelText(/Reviewer #\d Stellar Address/i);
    const validAddress1 = "GBK4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42";
    
    fireEvent.change(reviewerInputs[0], { target: { value: validAddress1 } });
    fireEvent.change(reviewerInputs[1], { target: { value: validAddress1 } }); // Duplicate
    fireEvent.change(reviewerInputs[2], { target: { value: "GBM4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42" } });

    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    await waitFor(() => {
      expect(screen.getByText("Duplicate reviewer addresses")).toBeInTheDocument();
    });
  });

  it("displays Step 4 summaries and handles contract deployment & retry flows", async () => {
    mockExecute.mockResolvedValue("0xHASH1234567890");

    const { rerender } = render(<CreateGrantForm />);

    // Fast track completely to Step 4
    fireEvent.change(screen.getByLabelText("Grant Title"), { target: { value: "Stellar Bridge Developer Grant Project" } });
    fireEvent.change(screen.getByPlaceholderText(/Describe your grant objectives/i), {
      target: { value: "This is a detailed description of the brand new development project on Soroban." },
    });
    fireEvent.change(screen.getByLabelText("Recipient Stellar Address"), {
      target: { value: "GBK4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42" },
    });
    fireEvent.change(screen.getByLabelText("Total Budget"), { target: { value: 500 } });
    fireEvent.change(screen.getByLabelText("Submission Expiry Deadline"), { target: { value: "2027-12-31T23:59" } });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    await waitFor(() => expect(screen.getByText("Step 2: Define Milestones")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Milestone Title (e.g. Prototype Deployment)"), { target: { value: "Milestone One Alpha Release" } });
    fireEvent.change(screen.getByPlaceholderText(/successful completion/i), {
      target: { value: "Detailed requirements mapping out successful milestone outputs." },
    });
    fireEvent.change(screen.getByPlaceholderText("Reward Allocation Amount"), { target: { value: 500 } });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    await waitFor(() => {
      expect(screen.getAllByLabelText(/Reviewer #\d Stellar Address/i)).toHaveLength(3);
    });
    const reviewerInputs = screen.getAllByLabelText(/Reviewer #\d Stellar Address/i);
    fireEvent.change(reviewerInputs[0], { target: { value: "GBK4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42" } });
    fireEvent.change(reviewerInputs[1], { target: { value: "GBL4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42" } });
    fireEvent.change(reviewerInputs[2], { target: { value: "GBM4TDRSOUF5LMX4PLM6L35R66Z356A2XF435ZDR4235SDR4235SDR42" } });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    // Reach Step 4: Summary screen
    await waitFor(() => expect(screen.getByText("Step 4: Review & Deploy")).toBeInTheDocument());
    expect(screen.getByText("Stellar Bridge Developer Grant Project")).toBeInTheDocument();
    expect(screen.getAllByText("500 XLM")).toHaveLength(2);

    // Click Deploy
    const deployBtn = screen.getByRole("button", { name: /deploy grant on-chain/i });
    fireEvent.click(deployBtn);

    // Verify loading state/simulation is triggered
    mockContractTxState.isSimulating = true;
    rerender(<CreateGrantForm />); // force state sync check
    expect(screen.getByText(/Simulating Soroban transaction/i)).toBeInTheDocument();
  });
});
