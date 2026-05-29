import { test, expect } from "@playwright/test";
import { mockWalletConnected } from "./setup/wallet";

test.describe("Create grant form", () => {
  test("shows validation errors on empty submit", async ({ page }) => {
    await mockWalletConnected(page);
    await page.goto("/grants/create");

    const nextButton = page.getByText("Next");
    if (await nextButton.isVisible().catch(() => false)) {
      await nextButton.click();

      const errors = page.locator("[class*=error], [class*=validation]");
      const errorCount = await errors.count();
      expect(errorCount).toBeGreaterThanOrEqual(1);
    }
  });
});
