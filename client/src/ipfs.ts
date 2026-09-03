/**
 * Grant metadata IPFS integration helpers (#488).
 *
 * Stores rich grant metadata (descriptions, attachments, structured JSON)
 * on IPFS via Pinata and retrieves it using a list of fallback gateways to
 * maximise availability.
 *
 * Why off-chain? Soroban storage costs scale with byte size. Keeping large
 * blobs on IPFS and storing only the 46-character CID on-chain cuts costs
 * significantly for description-heavy grants.
 *
 * Usage:
 * ```typescript
 * import { uploadMetadataToIPFS, fetchMetadataFromIPFS } from "@stellargrants/client-sdk";
 *
 * const { cid } = await uploadMetadataToIPFS(
 *   { title: "Ocean clean-up", description: "..." },
 *   { pinataJwt: process.env.PINATA_JWT }
 * );
 *
 * const meta = await fetchMetadataFromIPFS(cid);
 * ```
 */

import { IpfsUploadConfig, IpfsUploadResult } from "./types";
import {
  inferMetadataSchemaName,
  validateMetadataAgainstSchema,
} from "./metadataSchemas";

const PINATA_PIN_URL = "https://api.pinata.cloud/pinning/pinJSONToIPFS";

/** Public gateways used in order when fetching a CID. */
const DEFAULT_GATEWAYS = [
  "https://gateway.pinata.cloud/ipfs/",
  "https://ipfs.io/ipfs/",
  "https://cloudflare-ipfs.com/ipfs/",
  "https://dweb.link/ipfs/",
];

/** Default per-gateway timeout (ms). */
const GATEWAY_TIMEOUT_MS = 10_000;

/**
 * Uploads a JSON metadata object to IPFS via the Pinata pinning service.
 *
 * @param metadata Any JSON-serialisable object.
 * @param config   Pinata authentication config.
 * @returns The CID and a public gateway URL.
 * @throws If no Pinata credentials are provided or the upload fails.
 */
export async function uploadMetadataToIPFS(
  metadata: Record<string, unknown>,
  config: IpfsUploadConfig,
): Promise<IpfsUploadResult> {
  if (!config.skipSchemaValidation) {
    const schemaName = config.metadataSchema ?? inferMetadataSchemaName(metadata);
    validateMetadataAgainstSchema(schemaName, metadata);
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (config.pinataJwt) {
    headers["Authorization"] = `Bearer ${config.pinataJwt}`;
  } else if (config.pinataApiKey && config.pinataSecretKey) {
    headers["pinata_api_key"] = config.pinataApiKey;
    headers["pinata_secret_api_key"] = config.pinataSecretKey;
  } else {
    throw new Error(
      "IPFS upload requires either a Pinata JWT (pinataJwt) " +
      "or an API key + secret (pinataApiKey + pinataSecretKey).",
    );
  }

  const body = JSON.stringify({
    pinataContent: metadata,
    pinataMetadata: { name: config.name ?? "stellargrant-metadata" },
  });

  const res = await fetchWithTimeout(PINATA_PIN_URL, {
    method: "POST",
    headers,
    body,
  });

  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`Pinata upload failed (${res.status}): ${text}`);
  }

  const json = (await res.json()) as { IpfsHash: string };
  const cid = json.IpfsHash;

  return {
    cid,
    gatewayUrl: `${DEFAULT_GATEWAYS[0]}${cid}`,
  };
}

/**
 * Fetches and parses JSON metadata from IPFS.
 *
 * Tries each gateway in order (first the provided list, then the SDK
 * defaults). Returns the first successful parse, ensuring availability even
 * when individual gateways are slow or unreachable.
 *
 * @param cid      IPFS Content Identifier.
 * @param gateways Optional ordered list of gateway base URLs to prefer.
 * @returns Parsed JSON metadata object.
 * @throws If all gateways fail or the response is not valid JSON.
 */
export async function fetchMetadataFromIPFS(
  cid: string,
  gateways: string[] = [],
): Promise<Record<string, unknown>> {
  const ordered = [...gateways, ...DEFAULT_GATEWAYS];
  const errors: string[] = [];

  for (const gateway of ordered) {
    const url = `${gateway.replace(/\/$/, "")}/${cid}`;
    try {
      const res = await fetchWithTimeout(url, {}, GATEWAY_TIMEOUT_MS);
      if (!res.ok) {
        errors.push(`${gateway}: HTTP ${res.status}`);
        continue;
      }
      return (await res.json()) as Record<string, unknown>;
    } catch (err) {
      errors.push(`${gateway}: ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  throw new Error(
    `Failed to fetch IPFS metadata for CID "${cid}" from all gateways:\n` +
    errors.join("\n"),
  );
}

/** fetch() wrapper that aborts after `timeoutMs` milliseconds. */
async function fetchWithTimeout(
  url: string,
  init: RequestInit = {},
  timeoutMs = GATEWAY_TIMEOUT_MS,
): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}
