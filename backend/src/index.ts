import { createServer } from "http";
import { env } from "./config/env";
import { createApp } from "./app";
import { buildDataSource } from "./db/data-source";
import { MockSorobanContractClient } from "./soroban/mock-client";
import { RealSorobanClient } from "./soroban/real-client";

const bootstrap = async () => {
  validateEnvOnStartup();

  const dataSource = buildDataSource();
  await dataSource.initialize();

  const useMock = process.env.USE_MOCK_SOROBAN === "true";
  const sorobanClient = useMock
    ? new MockSorobanContractClient()
    : new RealSorobanClient();

  const app = createApp(dataSource, sorobanClient);
  const server = createServer(app);

  server.listen(env.port, () => {
    // Keep startup log concise for container logs.
    console.log(`API listening on port ${env.port}`);
  });
};

bootstrap().catch((error) => {
  console.error("Failed to start API", error);
  process.exit(1);
});
