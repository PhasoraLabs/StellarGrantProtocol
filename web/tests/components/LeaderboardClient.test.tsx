import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import LeaderboardClient from "@/app/leaderboard/LeaderboardClient";
import type { LeaderboardEntry } from "@/components/leaderboard";

const PAGE_SIZE = 25;

function makeEntry(address: string, rank: number): LeaderboardEntry {
  return {
    rank,
    address,
    reputationScore: Math.max(0, 100 - rank),
    grantsCompleted: rank,
    totalEarned: BigInt(rank) * 1_000_000n,
    token: "native",
  };
}

/** 25 rows for page 1, 25 rows for page 2 — page 2 is never pre-loaded. */
const page1Entries = Array.from({ length: PAGE_SIZE }, (_, i) =>
  makeEntry(`GAAA${String(i).padStart(2, "0")}${"B".repeat(50)}`, i + 1),
);
const page2Entries = Array.from({ length: PAGE_SIZE }, (_, i) =>
  makeEntry(`GCCC${String(i).padStart(2, "0")}${"DDD".repeat(16)}`, PAGE_SIZE + i + 1),
);

/** An address that only exists on page 2 of the leaderboard. */
const page2Address = page2Entries[7].address;

const fetchMock = vi.fn();

function okPageResponse(pageNumber: number) {
  const entries = pageNumber === 1 ? page1Entries : pageNumber === 2 ? page2Entries : [];
  return {
    ok: true,
    json: async () => ({ entries, total: 50, page: pageNumber }),
  };
}

function pageFromUrl(input: unknown): number {
  const urlStr =
    typeof input === "object" && input !== null && "url" in input
      ? (input as { url: string }).url
      : String(input);
  const parsed = new URL(urlStr, "http://localhost");
  return Number(parsed.searchParams.get("page") ?? "1");
}

describe("LeaderboardClient search", () => {
  beforeEach(() => {
    fetchMock.mockReset();
    fetchMock.mockImplementation(async (url: unknown) => okPageResponse(pageFromUrl(url)));
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders the currently loaded page", async () => {
    render(<LeaderboardClient />);

    expect(await screen.findByTitle(page1Entries[0].address)).toBeTruthy();
    expect(screen.queryByTitle(page2Address)).toBeNull();
  });

  it("finds an address that only exists on a later page (global search)", async () => {
    render(<LeaderboardClient />);
    await screen.findByTitle(page1Entries[0].address);

    // Address lives on page 2 — it must not be visible yet.
    expect(screen.queryByTitle(page2Address)).toBeNull();

    fireEvent.change(screen.getByLabelText("Search the full leaderboard by address"), {
      target: { value: page2Address },
    });

    expect(await screen.findByTitle(page2Address, {}, { timeout: 3000 })).toBeTruthy();
    expect(
      await screen.findByText("Showing 1 match across the full leaderboard."),
    ).toBeTruthy();

    // The search really paged through the API instead of filtering page 1.
    const searchedPages = fetchMock.mock.calls.map(([url]) => pageFromUrl(url));
    expect(searchedPages).toContain(2);

    // Search results are not page-scoped, so pagination is hidden meanwhile.
    expect(screen.queryByLabelText("Previous page")).toBeNull();

    // Clearing the search returns to the paginated view.
    fireEvent.click(screen.getByRole("button", { name: /clear search/i }));
    await waitFor(() => expect(screen.queryByTitle(page2Address)).toBeNull());
    expect(await screen.findByTitle(page1Entries[0].address)).toBeTruthy();
  });

  it("reports no match instead of silently showing an empty table", async () => {
    render(<LeaderboardClient />);
    await screen.findByTitle(page1Entries[0].address);

    const missing = `GZZZ${"9".repeat(52)}`;
    fireEvent.change(screen.getByLabelText("Search the full leaderboard by address"), {
      target: { value: missing },
    });

    expect(
      await screen.findByText(
        new RegExp(`No contributor address matches “${missing}”`),
        {},
        { timeout: 3000 },
      ),
    ).toBeTruthy();
  });

  it("surfaces a failed search rather than pretending nothing matched", async () => {
    fetchMock.mockReset();
    fetchMock.mockResolvedValue({ ok: false, status: 500, json: async () => ({}) });
    render(<LeaderboardClient />);

    fireEvent.change(screen.getByLabelText("Search the full leaderboard by address"), {
      target: { value: page2Address },
    });

    expect(
      await screen.findByText(/could not be completed/, {}, { timeout: 3000 }),
    ).toBeTruthy();
  });
});
