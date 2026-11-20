import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    environment: "node",
    // Run test files sequentially to avoid sqljs in-memory DB conflicts
    // between test files that share the same sql.js module singleton.
    fileParallelism: false,
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      exclude: ["tests/**", "src/soroban/mock-client.ts", "src/index.ts"],
    },
  },
});
