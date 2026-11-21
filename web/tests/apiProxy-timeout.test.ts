import { describe, it, expect, vi, afterEach } from "vitest";
import { proxyGet } from "@/lib/apiProxy";

afterEach(() => vi.restoreAllMocks());

describe("proxyGet timeout", () => {
  it("returns a 503 UPSTREAM_TIMEOUT when the upstream request is aborted", async () => {
    vi.spyOn(globalThis, "fetch").mockRejectedValue(
      Object.assign(new Error("aborted"), { name: "AbortError" }),
    );
    const res = await proxyGet(new Request("http://localhost/api/x"), "/grants");
    expect(res.status).toBe(503);
    expect(await res.json()).toMatchObject({ code: "UPSTREAM_TIMEOUT" });
  });
});
