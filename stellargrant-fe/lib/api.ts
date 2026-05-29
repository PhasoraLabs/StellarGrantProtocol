/**
 * StellarGrants — Axios API Instance
 *
 * Centralised Axios instance for all calls to the Express.js indexing API.
 * Replaces ad-hoc fetch() calls throughout the app so that:
 *   - The base URL is configured once via NEXT_PUBLIC_API_URL
 *   - The wallet address header is attached automatically
 *   - Response data is unwrapped from the { data: ... } envelope
 *   - Errors are normalised to StellarGrantsError
 *
 * Usage:
 *   import { api } from "@/lib/api"
 *   const grant = await api.get<Grant>(`/grants/${id}`)
 */

import axios, { AxiosError } from "axios";
import { StellarGrantsError } from "@/lib/errors";
import { useWalletStore } from "@/lib/store/walletStore";

// ── Instance ──────────────────────────────────────────────────────────────────

export const api = axios.create({
  baseURL: process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000",
  timeout: 15_000,
  headers: { "Content-Type": "application/json" },
});

// ── Request interceptor — attach wallet address header ────────────────────────

api.interceptors.request.use((config) => {
  const address = useWalletStore.getState().address;
  if (address) {
    config.headers["X-Wallet-Address"] = address;
  }
  return config;
});

// ── Response interceptor — unwrap envelope + normalise errors ─────────────────

/**
 * Unwrap the `{ data: ... }` envelope returned by the indexing API.
 * If the response body has a top-level `data` key, return that; otherwise
 * pass the full body through so callers always receive the payload directly.
 */
api.interceptors.response.use(
  (response) => {
    // Unwrap { data: payload } envelope when present
    if (
      response.data !== null &&
      typeof response.data === "object" &&
      "data" in response.data
    ) {
      response.data = response.data.data;
    }
    return response;
  },
  (error: AxiosError<{ message?: string; error?: string }>) => {
    const status = error.response?.status;
    const serverMsg =
      error.response?.data?.message ??
      error.response?.data?.error ??
      error.message ??
      "An unexpected error occurred";

    const stellarError = new StellarGrantsError(serverMsg, {
      cause: error,
    });

    // Attach HTTP status for callers that want to branch on it
    Object.defineProperty(stellarError, "statusCode", {
      value: status,
      writable: false,
      enumerable: true,
    });

    return Promise.reject(stellarError);
  },
);

// ── Typed helper wrappers ─────────────────────────────────────────────────────

/**
 * GET request — returns the unwrapped payload directly.
 *
 * @example
 *   const grant = await apiGet<Grant>(`/grants/${id}`)
 */
export async function apiGet<T>(
  url: string,
  params?: Record<string, unknown>,
): Promise<T> {
  const response = await api.get<T>(url, { params });
  return response.data;
}

/**
 * POST request — returns the unwrapped payload directly.
 *
 * @example
 *   const result = await apiPost<CreateGrantResponse>("/grants", payload)
 */
export async function apiPost<T>(url: string, data?: unknown): Promise<T> {
  const response = await api.post<T>(url, data);
  return response.data;
}
