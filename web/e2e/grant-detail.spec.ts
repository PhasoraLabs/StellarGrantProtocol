import { test, expect } from "@playwright/test";

test.describe("Grant detail page", () => {
  test("renders grant details", async ({ page }) => {
    await page.goto("/grants/1");

    const title = page.locator("h1");
    await expect(title).toBeVisible();
    const titleText = await title.textContent();
    expect(titleText).toBeTruthy();
    expect(titleText).not.toBe("Sample Grant");

    const progressBar = page.locator("[class*=progress]");
    await expect(progressBar).toBeVisible();
  });

  test("fund button is visible", async ({ page }) => {
    await page.goto("/grants/1");

    const fundButton = page.getByText(/fund/i);
    await expect(fundButton).toBeVisible();
  });
});
