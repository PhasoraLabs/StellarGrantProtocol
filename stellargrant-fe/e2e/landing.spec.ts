import { test, expect } from "@playwright/test";

test.describe("Landing page", () => {
  test("loads correctly with all key elements", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });

    await page.goto("/");

    await expect(page.getByText("FUND WHAT MATTERS")).toBeVisible();
    await expect(page.getByText("Browse Grants")).toBeVisible();

    const statTiles = page.locator("[class*=stat]");
    const tileCount = await statTiles.count();
    expect(tileCount).toBeGreaterThanOrEqual(3);

    expect(errors).toHaveLength(0);
  });

  test("is responsive at mobile viewport", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto("/");

    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    const viewportWidth = await page.evaluate(() => window.innerWidth);
    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth + 1);
  });
});
