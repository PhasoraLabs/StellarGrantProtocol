/**
 * MilestoneSubmitForm + ProofViewer Component Tests (Issue #338)
 *
 * Tests are structured in two sections:
 *  A. MilestoneSubmitForm — guard rendering, validation messages, file upload UX
 *  B. ProofViewer          — IPFS CID rendering, HTTPS link, raw SHA-256 hash
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

// ── Shared mock constants ──────────────────────────────────────────────────────
const WALLET_ADDRESS = "GDUMMYWALLETADDRESS1234567890ABCDE12345678";
const GRANT_ID = "42";
const MILESTONE_IDX = 0;

// ── Module mocks ───────────────────────────────────────────────────────────────

vi.mock("@/hooks/useWallet", () => ({
  useWallet: () => ({ address: WALLET_ADDRESS, signTransaction: vi.fn() }),
}));

vi.mock("@/hooks/useGrant", () => ({
  useGrant: () => ({
    data: {
      id: GRANT_ID,
      owner: WALLET_ADDRESS,
      reviewers: [],
      title: "Test Grant",
      budget: 1_000_000n,
      funded: 0n,
      status: 1,
      milestones: 1,
      deadline: 9999999999n,
      created_at: 1000000n,
      description: "",
    },
    isLoading: false,
    error: null,
  }),
}));

vi.mock("@/hooks/useMilestone", () => ({
  useMilestone: () => ({
    milestone: {
      idx: 0,
      title: "M0",
      description: "First milestone",
      submitted: false,
      approved: false,
      paid: false,
      proof_hash: null,
      submitted_at: null,
      approved_at: null,
      paid_at: null,
    },
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  }),
}));

vi.mock("@/hooks/useIPFS", () => ({
  useIPFS: () => ({
    upload: vi.fn().mockResolvedValue("QmMockedCID1234"),
    cid: null,
    isUploading: false,
    progress: 0,
    error: null,
    reset: vi.fn(),
  }),
}));

vi.mock("@/hooks/useContractTransaction", () => ({
  useContractTransaction: () => ({
    execute: vi.fn().mockResolvedValue("tx_hash_mock_123"),
    isPending: false,
    isSimulating: false,
    isSuccess: false,
    txHash: null,
    error: null,
    reset: vi.fn(),
  }),
}));

vi.mock("@/components/ui/RichTextRenderer", () => ({
  default: ({ content }: { content: string }) => (
    <div data-testid="rich-text-renderer">{content}</div>
  ),
}));

// ── Global setup ──────────────────────────────────────────────────────────────

beforeEach(() => {
  // Clipboard mock
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText: vi.fn().mockResolvedValue(undefined) },
    writable: true,
    configurable: true,
  });
  // crypto.subtle mock (read-only in jsdom, must use defineProperty)
  Object.defineProperty(globalThis, "crypto", {
    value: {
      subtle: {
        digest: vi.fn().mockResolvedValue(new Uint8Array(32).buffer),
      },
      randomUUID: () => "mock-uuid-1234",
    },
    writable: true,
    configurable: true,
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Part A: MilestoneSubmitForm
// ─────────────────────────────────────────────────────────────────────────────

import { MilestoneSubmitForm } from "@/components/milestones/MilestoneSubmitForm";

describe("MilestoneSubmitForm", () => {
  it("renders the description textarea", () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    expect(screen.getByPlaceholderText(/minimum 50 characters/i)).toBeTruthy();
  });

  it("renders the proof URL input", () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    expect(screen.getByPlaceholderText(/https:\/\/github\.com/i)).toBeTruthy();
  });

  it("renders the file upload drop zone", () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    expect(screen.getByText(/drag & drop file here/i)).toBeTruthy();
  });

  it("renders the Submit Milestone button", () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    expect(screen.getByText(/submit milestone/i)).toBeTruthy();
  });

  it("shows validation error when description is too short", async () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    const textarea = screen.getByPlaceholderText(/minimum 50 characters/i);
    fireEvent.change(textarea, { target: { value: "Too short" } });
    const urlInput = screen.getByPlaceholderText(/https:\/\/github\.com/i);
    fireEvent.change(urlInput, { target: { value: "https://github.com/foo/bar" } });
    fireEvent.click(screen.getByText(/submit milestone/i));
    await waitFor(() => {
      expect(screen.getByText(/at least 50 characters/i)).toBeTruthy();
    });
  });

  it("shows error when neither URL nor file is provided", async () => {
    const { container } = render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    const textarea = screen.getByPlaceholderText(/minimum 50 characters/i);
    fireEvent.change(textarea, { target: { value: "A".repeat(55) } });
    // Button is disabled without URL/file, so submit the form directly
    const form = container.querySelector("form") as HTMLFormElement;
    fireEvent.submit(form);
    await waitFor(() => {
      const errors = screen.getAllByText(/Provide a proof URL or upload a file/i);
      expect(errors.length).toBeGreaterThan(0);
    });
  });

  it("disables the URL input once a file is selected", async () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(["# Proof"], "proof.md", { type: "text/markdown" });
    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/URL disabled while file is uploaded/i)).toBeTruthy();
    });
  });

  it("shows the selected file name after file selection", async () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(["content"], "milestone-proof.md", { type: "text/markdown" });
    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText("milestone-proof.md")).toBeTruthy();
    });
  });

  it("rejects a file exceeding 10 MB", async () => {
    render(<MilestoneSubmitForm grantId={GRANT_ID} milestoneIdx={MILESTONE_IDX} />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const bigFile = new File(["x"], "big.pdf", { type: "application/pdf" });
    // Override size property to simulate large file
    Object.defineProperty(bigFile, "size", { value: 11 * 1024 * 1024 });
    fireEvent.change(fileInput, { target: { files: [bigFile] } });
    await waitFor(() => {
      expect(screen.getByText(/exceeds 10 MB limit/i)).toBeTruthy();
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Part B: ProofViewer
// ─────────────────────────────────────────────────────────────────────────────

import { ProofViewer } from "@/components/milestones/ProofViewer";

// ── SHA-256 hash ──────────────────────────────────────────────────────────────

describe("ProofViewer — raw SHA-256 hash", () => {
  const SHA256_HASH = "a".repeat(64);

  it("renders the SHA-256 hash label", () => {
    render(<ProofViewer proofHash={SHA256_HASH} />);
    expect(screen.getByText(/Proof Hash.*SHA-256/i)).toBeTruthy();
  });

  it("displays the full hash string", () => {
    render(<ProofViewer proofHash={SHA256_HASH} />);
    // Hash appears in both the display block and the disclosure section
    const matches = screen.getAllByText(SHA256_HASH);
    expect(matches.length).toBeGreaterThan(0);
  });

  it("has at least one copy button", () => {
    render(<ProofViewer proofHash={SHA256_HASH} />);
    expect(screen.getAllByRole("button").length).toBeGreaterThan(0);
  });

  it("copies hash to clipboard when copy button clicked", async () => {
    render(<ProofViewer proofHash={SHA256_HASH} />);
    const buttons = screen.getAllByRole("button");
    fireEvent.click(buttons[0]);
    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(SHA256_HASH);
    });
  });

  it("renders the Technical Details disclosure", () => {
    render(<ProofViewer proofHash={SHA256_HASH} />);
    expect(screen.getByText(/Technical Details/i)).toBeTruthy();
  });

  it("shows format type in the disclosure", () => {
    render(<ProofViewer proofHash={SHA256_HASH} />);
    expect(screen.getByText(/Cryptographic Hash/i)).toBeTruthy();
  });
});

// ── HTTPS links ───────────────────────────────────────────────────────────────

describe("ProofViewer — HTTPS URL", () => {
  const GITHUB_URL = "https://github.com/stellar/stellar-protocol/issues/1234";
  const YOUTUBE_URL = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
  const PLAIN_URL = "https://example.com/my-proof-document";

  it("renders 'External Proof Link' heading for HTTPS URL", () => {
    render(<ProofViewer proofHash={GITHUB_URL} />);
    expect(screen.getByText(/External Proof Link/i)).toBeTruthy();
  });

  it("renders a clickable anchor with the URL as href", () => {
    render(<ProofViewer proofHash={GITHUB_URL} />);
    const link = document.querySelector(`a[href="${GITHUB_URL}"]`);
    expect(link).toBeTruthy();
  });

  it("shows GitHub-specific label for github.com URLs", () => {
    render(<ProofViewer proofHash={GITHUB_URL} />);
    expect(screen.getByText(/GitHub Repository \/ Issue \/ PR Evidence/i)).toBeTruthy();
  });

  it("renders a YouTube embed iframe for YouTube URLs", () => {
    render(<ProofViewer proofHash={YOUTUBE_URL} />);
    const iframe = document.querySelector('iframe[src*="youtube.com/embed"]');
    expect(iframe).toBeTruthy();
  });

  it("renders short URLs without truncation", () => {
    render(<ProofViewer proofHash={PLAIN_URL} />);
    // URL appears in link text and the disclosure raw value
    const matches = screen.getAllByText(PLAIN_URL);
    expect(matches.length).toBeGreaterThan(0);
  });

  it("truncates URLs longer than 60 chars with '...'", () => {
    const longUrl = "https://example.com/" + "a".repeat(80);
    render(<ProofViewer proofHash={longUrl} />);
    expect(screen.getByText(/\.\.\./)).toBeTruthy();
  });

  it("renders Technical Details section for URL proofs", () => {
    render(<ProofViewer proofHash={GITHUB_URL} />);
    expect(screen.getByText(/Technical Details/i)).toBeTruthy();
  });
});

// ── IPFS CID ──────────────────────────────────────────────────────────────────

describe("ProofViewer — IPFS CID", () => {
  const IPFS_CID = "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco";

  beforeEach(() => {
    vi.restoreAllMocks();
    // Re-apply clipboard mock after restoreAllMocks
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      writable: true,
      configurable: true,
    });
  });

  it("renders a loading shimmer while fetching IPFS content", () => {
    global.fetch = vi.fn().mockReturnValue(new Promise(() => {})); // hang forever
    const { container } = render(<ProofViewer proofHash={IPFS_CID} />);
    const shimmer = container.querySelector(".shimmer");
    expect(shimmer).toBeTruthy();
  });

  it("renders plain text proof inside <pre> after load", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/plain" },
      text: async () => "Hello from IPFS plain text proof.",
    } as unknown as Response);

    render(<ProofViewer proofHash={IPFS_CID} />);
    await waitFor(() => {
      expect(screen.getByText(/Hello from IPFS plain text proof\./)).toBeTruthy();
    });
  });

  it("renders markdown via RichTextRenderer for markdown content-type", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/markdown" },
      text: async () => "# Proof Title\n\nThis is my **proof** document.",
    } as unknown as Response);

    render(<ProofViewer proofHash={IPFS_CID} />);
    await waitFor(() => {
      expect(screen.getByTestId("rich-text-renderer")).toBeTruthy();
    });
  });

  it("shows error state when IPFS gateway fails", async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error("Network failure"));

    render(<ProofViewer proofHash={IPFS_CID} />);
    await waitFor(() => {
      expect(screen.getByText(/Content unavailable/i)).toBeTruthy();
    }, { timeout: 3000 });
  });

  it("shows a public gateway link in error state", async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error("Timeout"));

    render(<ProofViewer proofHash={IPFS_CID} />);
    await waitFor(() => {
      expect(screen.getByText(/View on public IPFS gateway/i)).toBeTruthy();
    }, { timeout: 3000 });
  });

  it("strips ipfs:// prefix and fetches from the bare CID", async () => {
    const withPrefix = `ipfs://${IPFS_CID}`;
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/plain" },
      text: async () => "content",
    } as unknown as Response);

    render(<ProofViewer proofHash={withPrefix} />);
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining(IPFS_CID),
        expect.any(Object)
      );
    });
  });

  it("shows IPFS Source link after content loads", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/plain" },
      text: async () => "Hello IPFS!",
    } as unknown as Response);

    render(<ProofViewer proofHash={IPFS_CID} />);
    await waitFor(() => {
      const link = document.querySelector(`a[href*="${IPFS_CID}"]`);
      expect(link).toBeTruthy();
    }, { timeout: 3000 });
  });
});
