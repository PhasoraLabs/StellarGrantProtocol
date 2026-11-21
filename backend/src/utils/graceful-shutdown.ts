import type { Server } from "node:http";
import type { DataSource } from "typeorm";

export function registerGracefulShutdown(
  server: Server,
  dataSource: DataSource,
): () => void {
  let shutdownPromise: Promise<void> | undefined;

  const shutdown = (signal: NodeJS.Signals): Promise<void> => {
    if (!shutdownPromise) {
      console.info(`Received ${signal}; shutting down gracefully`);
      shutdownPromise = (async () => {
        let exitCode = 0;

        try {
          await new Promise<void>((resolve, reject) => {
            server.close((error) => {
              if (error) {
                reject(error);
              } else {
                resolve();
              }
            });
          });
        } catch (error) {
          console.error("Failed to close HTTP server", error);
          exitCode = 1;
        }

        try {
          await dataSource.destroy();
        } catch (error) {
          console.error("Failed to close database connection", error);
          exitCode = 1;
        }

        process.exit(exitCode);
      })();
    }

    return shutdownPromise;
  };

  const handleSigint = () => {
    void shutdown("SIGINT");
  };
  const handleSigterm = () => {
    void shutdown("SIGTERM");
  };

  process.on("SIGINT", handleSigint);
  process.on("SIGTERM", handleSigterm);

  return () => {
    process.off("SIGINT", handleSigint);
    process.off("SIGTERM", handleSigterm);
  };
}
