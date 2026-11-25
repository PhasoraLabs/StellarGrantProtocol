import { test, expect } from "@playwright/test";
import { mockWalletConnected } from "./setup/wallet";

const recipient = "GCK4LZF4RXGPTVCQFSAZ5RYS7CY26BSSML5TJLSBVWUNFJDTNGJ2LOWH";

test.describe("Milestone proof submission", () => {
  test("uploads evidence to IPFS and attempts the proof transaction", async ({ page }) => {
    await mockWalletConnected(page, recipient);

    const pendingMilestone = {
      idx: 0,
      title: "First milestone",
      description: "Deliver the initial implementation",
      deadline: new Date(Date.now() + 86_400_000).toISOString(),
      submitted: false,
      approved: false,
      paid: false,
      proof_hash: null,
    };
    const grant = {
      id: 1,
      owner: recipient,
      title: "E2E grant",
      description: "Grant for the milestone submission flow",
      budget: "1000000000",
      funded: "1000000000",
      deadline: String(Math.floor(Date.now() / 1000) + 86_400),
      created_at: String(Math.floor(Date.now() / 1000)),
      status: "active",
      recipient,
      reviewers: [],
      totalAmount: "1000000000",
      milestones: [pendingMilestone],
    };

    await page.route("**/api/grants/1", (route) =>
      route.fulfill({ json: { grant, milestones: [pendingMilestone], completedMilestones: 0, isWatched: false } }),
    );
    await page.route("http://localhost:4000/grants/1", (route) =>
      route.fulfill({ json: { data: grant } }),
    );
    await page.route("http://localhost:4000/grant_reviewers/grant/1", (route) =>
      route.fulfill({ json: { data: [] } }),
    );

    let uploadedFileName = "";
    let transactionRequests = 0;
    await page.route("**/api/ipfs", async (route) => {
      const request = route.request();
      const body = request.postDataBuffer();
      uploadedFileName = body?.toString("utf8").includes("milestone-proof.txt")
        ? "milestone-proof.txt"
        : "";
      await route.fulfill({ json: { IpfsHash: "bafybeie2eproofcid" } });
    });
    await page.route("https://soroban-testnet.stellar.org/**", async (route) => {
      transactionRequests += 1;
      await route.abort("failed");
    });

    await page.goto("/grants/1/milestones/0");
    await expect(page.getByRole("heading", { name: "First milestone" })).toBeVisible();

    await page.getByPlaceholder(/provide a detailed description/i).fill(
      "Completed the initial implementation, added the requested changes, and documented the result.",
    );
    await page.locator('input[type="file"]').setInputFiles({
      name: "milestone-proof.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("Evidence for the first milestone"),
    });

    await expect(page.getByText("milestone-proof.txt")).toBeVisible();
    await page.getByRole("button", { name: "Submit Milestone" }).click();

    await expect.poll(() => uploadedFileName).toBe("milestone-proof.txt");
    await expect(page.getByText(/successfully pinned to ipfs/i)).toBeVisible();
    await expect(page.getByText("bafybeie2eproofcid")).toBeVisible();
    await expect.poll(() => transactionRequests).toBeGreaterThan(0);
    await expect(page.getByText(/Error: Network Error/).first()).toBeVisible();
  });
});
