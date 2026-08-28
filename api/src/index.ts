import { createServer } from "http";
import { env } from "./config/env";
import { createApp } from "./app";
import { buildDataSource } from "./db/data-source";
import { MockSorobanContractClient } from "./soroban/mock-client";

const bootstrap = async () => {
  validateEnvOnStartup();
  
  const dataSource = buildDataSource();
  await dataSource.initialize();

  const app = createApp(dataSource, new MockSorobanContractClient());
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
