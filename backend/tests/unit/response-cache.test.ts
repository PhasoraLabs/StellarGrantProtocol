import { describe, expect, it } from "vitest";
import { ResponseCacheService, responseCacheKeys } from "../../src/services/response-cache";

describe("ResponseCacheService.invalidateGrantsAndStats", () => {
  it("clears the stats key and every grants first-page key but nothing else", async () => {
    const cache = new ResponseCacheService();
    await cache.set(responseCacheKeys.stats(), "stats");
    await cache.set(responseCacheKeys.grantsFirstPage("en"), "en");
    await cache.set(responseCacheKeys.grantsFirstPage("fr"), "fr");
    await cache.set(responseCacheKeys.grantFeedback(1), "feedback");

    await cache.invalidateGrantsAndStats();

    expect(await cache.get(responseCacheKeys.stats())).toBeNull();
    expect(await cache.get(responseCacheKeys.grantsFirstPage("en"))).toBeNull();
    expect(await cache.get(responseCacheKeys.grantsFirstPage("fr"))).toBeNull();
    expect(await cache.get(responseCacheKeys.grantFeedback(1))).toBe("feedback");
  });
});
