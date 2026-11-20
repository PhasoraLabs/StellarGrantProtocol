import { describe, it, expect } from "vitest";
import type { AxiosAdapter, AxiosError } from "axios";
import { api, apiGet } from "@/lib/api";

function adapterFailing(status: number | null, body?: unknown): AxiosAdapter {
  return (config) =>
    Promise.reject(
      Object.assign(new Error(status ? `HTTP ${status}` : "Network Error"), {
        isAxiosError: true,
        config,
        response: status ? { status, data: body, headers: {}, config } : undefined,
      }) as AxiosError,
    );
}

describe("lib/api error handling", () => {
  it("maps a server error to an api error with status and message", async () => {
    api.defaults.adapter = adapterFailing(404, { message: "Grant not found" });
    await expect(apiGet("/grants/1")).rejects.toMatchObject({
      message: "Grant not found",
      statusCode: 404,
      type: "api",
    });
  });

  it("falls back to the error field", async () => {
    api.defaults.adapter = adapterFailing(500, { error: "boom" });
    await expect(apiGet("/x")).rejects.toMatchObject({ message: "boom", statusCode: 500 });
  });

  it("maps a missing response to a network error", async () => {
    api.defaults.adapter = adapterFailing(null);
    await expect(apiGet("/x")).rejects.toMatchObject({ type: "network" });
  });
});
