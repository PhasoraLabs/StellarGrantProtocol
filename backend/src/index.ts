import { createServer } from "http";
import { env } from "./config/env";
import { validateEnvOnStartup } from "./config/env-validation";
import { createApp } from "./app";
import { buildDataSource } from "./db/data-source";
import { MockSorobanContractClient } from "./soroban/mock-client";
import { RealSorobanClient } from "./soroban/real-client";
import { registerGracefulShutdown } from "./utils/graceful-shutdown";

const bootstrap = async () => {
  validateEnvOnStartup();

  const dataSource = buildDataSource();
  await dataSource.initialize();

  const sorobanClient = env.useMockSoroban
    ? new MockSorobanContractClient()
    : new RealSorobanClient();

  const app = createApp(dataSource, sorobanClient);
  const server = createServer(app);

  registerGracefulShutdown(server, dataSource);

  server.listen(env.port, () => {
    // Keep startup log concise for container logs.
    console.log(`API listening on port ${env.port}`);
  });
};

bootstrap().catch((error) => {
  console.error("Failed to start API", error);
  process.exit(1);
});
