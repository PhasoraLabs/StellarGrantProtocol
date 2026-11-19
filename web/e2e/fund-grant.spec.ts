import { test, expect } from "@playwright/test";
import { mockWalletConnected } from "./setup/wallet";

test.describe("Fund grant flow", () => {
  test.beforeEach(async ({ page }) => {
    await mockWalletConnected(page);
    // Mock the pre-submit funding-state check: the grant is fully funded.
    await page.route("**/api/grants/1", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ grant: { id: "1", budget: "1000000000", funded: "1000000000" } }),
      }),
    );
  });

  test("opens the fund modal and blocks funding a fully funded grant", async ({ page }) => {
    await page.goto("/grants/1");

    await page.getByRole("button", { name: /fund/i }).first().click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();

    await dialog.getByLabel(/amount/i).fill("10");
    await expect(dialog.getByText(/your contribution/i)).toBeVisible();

    await dialog.getByRole("button", { name: /^fund grant$/i }).click();
    await expect(dialog.getByText(/already fully funded/i)).toBeVisible();
  });
});
