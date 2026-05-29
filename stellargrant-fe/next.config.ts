import path from "path";
import type { NextConfig } from "next";

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

export default nextConfig;
