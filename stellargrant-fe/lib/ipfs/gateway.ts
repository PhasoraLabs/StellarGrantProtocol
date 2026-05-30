export const IPFS_GATEWAYS = [
  "https://ipfs.io/ipfs",
  "https://cloudflare-ipfs.com/ipfs",
  "https://gateway.pinata.cloud/ipfs",
  "https://dweb.link/ipfs",
  "https://ipfs.filebase.io/ipfs",
];

// Populated by detectFastestGateway(); used as the first attempt in fetchFromIPFS.
let fastestGateway: string | null = null;

export async function fetchFromIPFS(
  cid: string,
  timeoutMs = 8000
): Promise<Response> {
  const orderedGateways =
    fastestGateway && fastestGateway !== IPFS_GATEWAYS[0]
      ? [fastestGateway, ...IPFS_GATEWAYS.filter((g) => g !== fastestGateway)]
      : IPFS_GATEWAYS;

  let lastError: Error | null = null;

  for (const gateway of orderedGateways) {
    try {
      const url = `${gateway}/${cid}`;
      const response = await fetch(url, {
        signal: AbortSignal.timeout(timeoutMs),
      });
      if (response.ok) return response;
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err));
      continue;
    }
  }

  throw lastError ?? new Error(`Failed to fetch ${cid} from all gateways`);
}

// Known public CID used to benchmark gateway latency.
const TEST_CID = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

export async function detectFastestGateway(): Promise<string> {
  const results = await Promise.allSettled(
    IPFS_GATEWAYS.map(async (gateway) => {
      const start = Date.now();
      const res = await fetch(`${gateway}/${TEST_CID}`, {
        signal: AbortSignal.timeout(5000),
      });
      if (!res.ok) throw new Error("not ok");
      return { gateway, latency: Date.now() - start };
    })
  );

  const fastest = results
    .filter((r) => r.status === "fulfilled")
    .map((r) => (r as PromiseFulfilledResult<{ gateway: string; latency: number }>).value)
    .sort((a, b) => a.latency - b.latency)[0];

  const winner = fastest?.gateway ?? IPFS_GATEWAYS[0];
  fastestGateway = winner;
  return winner;
}

// Kick off gateway detection once when this module is first imported in a browser.
if (typeof window !== "undefined") {
  detectFastestGateway().catch(() => {
    // detection failure — fall back to default gateway order
  });
}
