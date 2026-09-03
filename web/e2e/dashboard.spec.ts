import { test, expect } from "@playwright/test";
import { mockWalletConnected } from "./setup/wallet";

test.describe("Dashboard", () => {
  test("shows connect prompt when wallet not connected", async ({ page }) => {
    await page.goto("/dashboard");

    await expect(page.getByText(/connect your wallet/i)).toBeVisible();
  });

  test("shows tabs when wallet is connected", async ({ page }) => {
    await mockWalletConnected(page);
    await page.goto("/dashboard");

    const tabs = ["My Grants", "Grants I Fund", "Grants I Review"];
    for (const tab of tabs) {
      await expect(page.getByText(tab)).toBeVisible();
    }
  });
});
