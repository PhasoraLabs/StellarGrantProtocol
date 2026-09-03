import path from "path";
import type { NextConfig } from "next";
import bundleAnalyzer from "@next/bundle-analyzer";

const withBundleAnalyzer = bundleAnalyzer({
  enabled: process.env.ANALYZE === "true",
});

const ContentSecurityPolicy = `
  default-src 'self';
  script-src 'self' 'unsafe-eval' 'unsafe-inline';
  style-src 'self' 'unsafe-inline' https://fonts.googleapis.com;
  font-src 'self' https://fonts.gstatic.com;
  img-src 'self' data: https://ipfs.io https://gateway.pinata.cloud blob:;
  connect-src 'self'
    https://horizon-testnet.stellar.org
    https://horizon.stellar.org
    https://soroban-testnet.stellar.org
    https://api.pinata.cloud
    ${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000"}
    ws: wss:;
  frame-src 'none';
  object-src 'none';
  base-uri 'self';
  form-action 'self';
  upgrade-insecure-requests;
`;

const securityHeaders = [
  {
    key: "X-DNS-Prefetch-Control",
    value: "on",
  },
  {
    key: "Strict-Transport-Security",
    value: "max-age=63072000; includeSubDomains; preload",
  },
  {
    key: "X-Frame-Options",
    value: "SAMEORIGIN",
  },
  {
    key: "X-Content-Type-Options",
    value: "nosniff",
  },
  {
    key: "Referrer-Policy",
    value: "origin-when-cross-origin",
  },
  {
    key: "Permissions-Policy",
    value: "camera=(), microphone=(), geolocation=()",
  },
  {
    key:
      process.env.NEXT_PUBLIC_CSP_REPORT_ONLY === "true"
        ? "Content-Security-Policy-Report-Only"
        : "Content-Security-Policy",
    value: ContentSecurityPolicy.replace(/\s{2,}/g, " ").trim(),
  },
];

const nextConfig: NextConfig = {
  /**
   * Point Next.js at the correct project root when multiple lockfiles exist
   * in the monorepo. Without this, Next.js may pick the root lockfile and
   * fail to resolve native platform binaries (SWC, lightningcss) on CI.
   */
  outputFileTracingRoot: path.resolve(__dirname),

  /**
   * Tree-shaking / bundle optimisation (#275)
   *
   * `optimizePackageImports` tells the Next.js bundler to rewrite named imports
   * from these packages into per-file deep imports so only the code that is
   * actually used ends up in the client bundle (no full-library barrel imports).
   */
  experimental: {
    optimizePackageImports: [
      "lucide-react",
      "framer-motion",
      "@stellar/stellar-sdk",
    ],
  },

  async headers() {
    return [
      {
        source: "/:path*",
        headers: securityHeaders,
      },
    ];
  },

  /**
   * Reduce the client-side @stellar/stellar-sdk footprint.
   * The RPC/XDR heavy-lifting is done server-side or in route handlers;
   * the browser bundle only needs the types + lightweight helpers.
   */
  webpack(config, { isServer }) {
    if (!isServer) {
      // Replace node-specific crypto builtins with browser-safe stubs where
      // the SDK pulls them in transitively.
      config.resolve = config.resolve ?? {};
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
        net: false,
        tls: false,
      };
    }
    return config;
  },
};

export default withBundleAnalyzer(nextConfig);
