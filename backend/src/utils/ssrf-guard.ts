import { promises as dns } from "node:dns";
import { isIPv4, isIPv6 } from "node:net";

/**
 * CIDR ranges that must never be webhook targets.
 * Covers loopback, private (RFC 1918), link-local (RFC 3927),
 * and the AWS/GCP metadata endpoint (169.254.169.254).
 */
const BLOCKED_IPV4_RANGES: [number, number, number][] = [
  // loopback 127.0.0.0/8
  [0x7f000000, 0xff000000, 8],
  // private 10.0.0.0/8
  [0x0a000000, 0xff000000, 8],
  // private 172.16.0.0/12
  [0xac100000, 0xfff00000, 12],
  // private 192.168.0.0/16
  [0xc0a80000, 0xffff0000, 16],
  // link-local + metadata 169.254.0.0/16 (includes 169.254.169.254)
  [0xa9fe0000, 0xffff0000, 16],
  // unspecified 0.0.0.0/8
  [0x00000000, 0xff000000, 8],
];

function ipv4ToInt(ip: string): number {
  return (
    ip
      .split(".")
      .reduce((acc, octet) => (acc << 8) | parseInt(octet, 10), 0) >>> 0
  );
}

function isBlockedIPv4(ip: string): boolean {
  const n = ipv4ToInt(ip);
  return BLOCKED_IPV4_RANGES.some(
    ([base, mask]) => (n & mask) >>> 0 === base >>> 0,
  );
}

function isBlockedIPv6(ip: string): boolean {
  // Normalise brackets that appear in URLs (e.g. [::1])
  const raw = ip.replace(/^\[|\]$/g, "").toLowerCase();
  // ::1  loopback
  if (raw === "::1") return true;
  // ::  unspecified
  if (raw === "::") return true;
  // fc00::/7  unique-local (fc00:: – fdff::)
  if (/^f[cd]/i.test(raw)) return true;
  // fe80::/10  link-local
  if (/^fe[89ab]/i.test(raw)) return true;
  // ::ffff:0:0/96  IPv4-mapped — treat the embedded IPv4 part
  const v4mapped = raw.match(/^::ffff:(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})$/);
  if (v4mapped) return isBlockedIPv4(v4mapped[1]);
  return false;
}

/**
 * Resolve `hostname` via DNS and return true if *any* resolved address
 * is in a blocked range.  Throws on resolution failure so the caller can
 * treat unresolvable hosts as unsafe.
 */
async function hostnameResolvesToBlockedAddress(
  hostname: string,
): Promise<boolean> {
  // If the hostname is already a bare IP address, check it directly.
  if (isIPv4(hostname)) return isBlockedIPv4(hostname);
  if (isIPv6(hostname.replace(/^\[|\]$/g, ""))) {
    return isBlockedIPv6(hostname);
  }

  let addresses: string[];
  try {
    const results = await dns.lookup(hostname, { all: true });
    addresses = results.map((r) => r.address);
  } catch {
    // Treat unresolvable as unsafe (fail closed).
    return true;
  }

  if (addresses.length === 0) return true;

  return addresses.some((addr) =>
    isIPv4(addr) ? isBlockedIPv4(addr) : isBlockedIPv6(addr),
  );
}

/**
 * Validate that `rawUrl` is safe to use as a webhook target.
 *
 * Checks:
 *   1. Must be a valid URL.
 *   2. Scheme must be `https`.
 *   3. Resolved IP addresses must not fall into private/loopback/link-local/
 *      metadata ranges.
 *
 * Throws an `Error` with a descriptive message if validation fails.
 */
export async function assertSafeWebhookUrl(rawUrl: string): Promise<void> {
  let parsed: URL;
  try {
    parsed = new URL(rawUrl);
  } catch {
    throw new Error("Invalid URL");
  }

  if (parsed.protocol !== "https:") {
    throw new Error("Webhook target URL must use HTTPS");
  }

  const hostname = parsed.hostname;

  if (await hostnameResolvesToBlockedAddress(hostname)) {
    throw new Error(
      "Webhook target URL resolves to a private, loopback, or link-local address",
    );
  }
}
