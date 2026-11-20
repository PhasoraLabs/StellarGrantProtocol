import { retryWithBackoff, withRetry } from "../src/utils/retry";

// Mock setTimeout to avoid actual delays in tests
let mockDelay = 0;
const originalSetTimeout = global.setTimeout;
global.setTimeout = ((fn: Function, delay: number) => {
  return originalSetTimeout(fn, mockDelay);
}) as any;

describe("retryWithBackoff", () => {
  it("should return result on first successful attempt", async () => {
    const fn = jest.fn().mockResolvedValue("success");
    const result = await retryWithBackoff(fn);
    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("should retry on retryable errors", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("429 Too Many Requests"))
      .mockRejectedValueOnce(new Error("timeout"))
      .mockResolvedValue("success");

    const result = await retryWithBackoff(fn, { maxAttempts: 3, initialDelayMs: 100 });
    
    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(3);
  });

  it("should not retry on non-retryable errors", async () => {
    const fn = jest.fn().mockRejectedValue(new Error("Invalid signature"));
    
    await expect(retryWithBackoff(fn, { maxAttempts: 3 })).rejects.toThrow("Invalid signature");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("should throw after max attempts exhausted", async () => {
    const fn = jest.fn().mockRejectedValue(new Error("429 Too Many Requests"));
    
    await expect(retryWithBackoff(fn, { maxAttempts: 2, initialDelayMs: 100 }))
      .rejects.toThrow("429 Too Many Requests");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("should use exponential backoff", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("timeout"))
      .mockRejectedValueOnce(new Error("timeout"))
      .mockResolvedValue("success");

    const onRetry = jest.fn();
    await retryWithBackoff(fn, {
      maxAttempts: 3,
      initialDelayMs: 100,
      backoffMultiplier: 2,
      onRetry,
    });
    
    expect(onRetry).toHaveBeenCalledTimes(2);
    expect(onRetry).toHaveBeenNthCalledWith(1, 1, expect.any(Error), 100);
    expect(onRetry).toHaveBeenNthCalledWith(2, 2, expect.any(Error), 200);
  });

  it("should respect max delay", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("timeout"))
      .mockRejectedValueOnce(new Error("timeout"))
      .mockResolvedValue("success");

    const onRetry = jest.fn();
    await retryWithBackoff(fn, {
      maxAttempts: 3,
      initialDelayMs: 100,
      backoffMultiplier: 10,
      maxDelayMs: 500,
      onRetry,
    });
    
    expect(onRetry).toHaveBeenCalledTimes(2);
    expect(onRetry).toHaveBeenNthCalledWith(1, 1, expect.any(Error), 100);
    expect(onRetry).toHaveBeenNthCalledWith(2, 2, expect.any(Error), 500); // Capped at maxDelayMs
  });

  it("should call onRetry callback", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("429"))
      .mockResolvedValue("success");

    const onRetry = jest.fn();
    await retryWithBackoff(fn, { maxAttempts: 2, initialDelayMs: 100, onRetry });

    expect(onRetry).toHaveBeenCalledTimes(1);
    expect(onRetry).toHaveBeenCalledWith(1, expect.any(Error), 100);
  });

  it("should retry on rate limit errors", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("429 Too Many Requests"))
      .mockResolvedValue("success");

    const result = await retryWithBackoff(fn, { maxAttempts: 2, retryOnRateLimit: true });
    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("should not retry on rate limit when disabled", async () => {
    const fn = jest.fn().mockRejectedValue(new Error("429 Too Many Requests"));
    
    await expect(retryWithBackoff(fn, { maxAttempts: 2, retryOnRateLimit: false }))
      .rejects.toThrow("429 Too Many Requests");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("should retry on timeout errors", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("request timeout"))
      .mockResolvedValue("success");

    const result = await retryWithBackoff(fn, { maxAttempts: 2, retryOnTimeout: true });
    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("should retry on network errors", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("ECONNREFUSED"))
      .mockResolvedValue("success");

    const result = await retryWithBackoff(fn, { maxAttempts: 2, retryOnNetworkError: true });
    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("should use default retry config when not provided", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("429 Too Many Requests"))
      .mockRejectedValueOnce(new Error("429 Too Many Requests"))
      .mockRejectedValueOnce(new Error("429 Too Many Requests"))
      .mockResolvedValue("success");

    // When no config is provided, it uses defaults (maxAttempts: 3)
    await expect(retryWithBackoff(fn)).rejects.toThrow("429 Too Many Requests");
    expect(fn).toHaveBeenCalledTimes(3); // Default maxAttempts is 3
  });
});

describe("withRetry", () => {
  it("should wrap a function with retry logic", async () => {
    const fn = jest.fn()
      .mockRejectedValueOnce(new Error("429"))
      .mockResolvedValue("success");

    const wrappedFn = withRetry(fn, { maxAttempts: 2, initialDelayMs: 100 });
    const result = await wrappedFn();

    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("should pass arguments to the wrapped function", async () => {
    const fn = jest.fn().mockResolvedValue("success");
    const wrappedFn = withRetry(fn);

    await wrappedFn("arg1", "arg2");

    expect(fn).toHaveBeenCalledWith("arg1", "arg2");
  });

  it("should preserve function return type", async () => {
    const fn = jest.fn().mockResolvedValue({ data: "test" });
    const wrappedFn = withRetry(fn);

    const result = await wrappedFn();
    expect(result).toEqual({ data: "test" });
  });
});
