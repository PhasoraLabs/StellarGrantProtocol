const CACHE_PREFIX = "sg:cache:v1:";

export const responseCacheKeys = {
  stats: () => `${CACHE_PREFIX}stats`,
  grantsFirstPage: (lang: string) => `${CACHE_PREFIX}grants:first:${lang}`,
};

/**
 * In-memory response cache fallback (no Redis dependency in tests).
 */
export class ResponseCacheService {
  private memoryCache = new Map<string, { value: string; expiresAt: number }>();

  constructor(redisUrl?: string) {
    // no-op for tests and environments without Redis
  }

  isEnabled(): boolean {
    return true;
  }

  async get(key: string): Promise<string | null> {
    const cached = this.memoryCache.get(key);
    if (!cached) return null;
    if (cached.expiresAt < Date.now()) {
      this.memoryCache.delete(key);
      return null;
    }
    return cached.value;
  }

  async set(key: string, value: string, ttlSeconds: number = 3600): Promise<void> {
    this.memoryCache.set(key, {
      value,
      expiresAt: Date.now() + ttlSeconds * 1000,
    });
  }

  async delete(key: string): Promise<void> {
    this.memoryCache.delete(key);
  }

  async flush(): Promise<void> {
    this.memoryCache.clear();
  }
}
