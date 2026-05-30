/**
 * Server-side helpers for the Next.js API proxy layer (Issue #371).
 *
 * Every proxy route in `app/api/*` forwards to the Express backend through
 * `proxyGet`. The backend URL is read from the *private* `API_URL` env var
 * (with a fallback to the legacy `NEXT_PUBLIC_API_URL` for environments that
 * still set it) so it never reaches client-side JavaScript. An optional shared
 * secret is forwarded as `X-Internal-Secret`.
 *
 * Upstream errors are normalised into a stable JSON shape:
 *   { error: string, code: string, status: number }
 */

import { NextResponse } from "next/server";

/** Internal backend base URL — server-side only. */
export const INTERNAL_API_URL =
  process.env.API_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  "http://localhost:4000";

/** Optional shared secret forwarded to the upstream as `X-Internal-Secret`. */
export const INTERNAL_API_SECRET = process.env.API_SECRET ?? "";

export interface ProxyErrorBody {
  error: string;
  code: string;
  status: number;
}

const TIMEOUT_MS = 10_000;

/**
 * Forward `path` (with the caller's query string) to the upstream API. Returns
 * the upstream JSON on success or a normalised `ProxyErrorBody` on any kind of
 * failure. Always emits a `Cache-Control: s-maxage=<revalidate>` header so the
 * Vercel/Next edge layer can cache the response.
 */
export async function proxyGet(
  request: Request,
  upstreamPath: string,
  options: { revalidate: number; forwardSearch?: boolean } = { revalidate: 30 },
): Promise<NextResponse> {
  const { revalidate, forwardSearch = true } = options;

  let url = `${INTERNAL_API_URL}${upstreamPath}`;
  if (forwardSearch) {
    const search = new URL(request.url).searchParams.toString();
    if (search) url += `?${search}`;
  }

  const headers: HeadersInit = { Accept: "application/json" };
  if (INTERNAL_API_SECRET) {
    headers["X-Internal-Secret"] = INTERNAL_API_SECRET;
  }

  const cacheHeader = `public, s-maxage=${revalidate}, stale-while-revalidate=${revalidate}`;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);

  try {
    const res = await fetch(url, {
      headers,
      next: { revalidate },
      signal: controller.signal,
    });

    if (!res.ok) {
      return jsonError({
        error: `Upstream returned ${res.status}`,
        code: "UPSTREAM_ERROR",
        status: res.status,
      });
    }

    const data = await res.json();
    return NextResponse.json(data, { headers: { "Cache-Control": cacheHeader } });
  } catch (err) {
    const aborted = err instanceof Error && err.name === "AbortError";
    return jsonError({
      error: aborted ? "Upstream request timed out" : "Failed to reach upstream API",
      code: aborted ? "UPSTREAM_TIMEOUT" : "UPSTREAM_UNREACHABLE",
      status: 503,
    });
  } finally {
    clearTimeout(timer);
  }
}

/** Build a normalised error response. */
export function jsonError(body: ProxyErrorBody): NextResponse {
  return NextResponse.json(body, { status: body.status });
}
