import { test, expect } from "@playwright/test";

test.describe("Browse and filter grants", () => {
  test("shows grant cards or empty state", async ({ page }) => {
    await page.goto("/grants");

    const cards = page.locator("a[href*='/grants/']");
    const emptyMessage = page.getByText(/no grants|no results|empty/i);

    const cardsVisible = (await cards.count()) > 0;
    const emptyVisible = await emptyMessage.isVisible().catch(() => false);

    expect(cardsVisible || emptyVisible).toBeTruthy();
  });

  test("filter bar has status pills and filter changes URL", async ({ page }) => {
    await page.goto("/grants");

    await expect(page.getByText("Active")).toBeVisible();

    await page.getByText("Active").click();
    await expect(page).toHaveURL(/status=active/);
  });

  test("search input updates URL after debounce", async ({ page }) => {
    await page.goto("/grants");

    const searchInput = page.locator('input[type="text"], input[placeholder*="search" i]');
    if ((await searchInput.count()) === 0) return;

    await searchInput.fill("research");
    await page.waitForTimeout(400);
    await expect(page).toHaveURL(/q=research/);
  });
});
