import { DataSource, MigrationExecutor } from "typeorm";
import dataSource from "../db/typeorm.config";

export interface RunMigrationsOptions {
  /** Print pending migrations only; do not apply them. */
  dryRun?: boolean;
}

export const hasDryRunFlag = (
  argv: string[] = process.argv.slice(2),
): boolean => argv.includes("--dry-run");

/** Names of migrations that have not been applied yet. */
export const getPendingMigrations = async (
  connection: DataSource,
): Promise<string[]> => {
  const executor = new MigrationExecutor(connection);
  const pending = await executor.getPendingMigrations();
  return pending.map((migration) => migration.name);
};

export const runMigrations = async (
  connection: DataSource = dataSource,
  options: RunMigrationsOptions = {},
): Promise<string[]> => {
  await connection.initialize();

  try {
    const pending = await getPendingMigrations(connection);

    if (options.dryRun) {
      if (pending.length === 0) {
        console.log("No pending migrations.");
      } else {
        console.log(`Pending migrations (${pending.length}):`);
        pending.forEach((name) => console.log(`  - ${name}`));
      }
      return pending;
    }

    await connection.runMigrations();
    console.log("Database migrations applied successfully.");
    return pending;
  } finally {
    await connection.destroy();
  }
};

// Only execute when invoked as `npm run migration:run [-- --dry-run]`;
// importing this module (e.g. from tests) must not connect to the database.
const entryPoint = process.argv[1] ?? "";
if (entryPoint.endsWith("run-migrations.ts") || entryPoint.endsWith("run-migrations.js")) {
  runMigrations(dataSource, { dryRun: hasDryRunFlag() }).catch((error) => {
    if (error?.code === "ECONNREFUSED") {
      console.error("Failed to run migrations: cannot connect to PostgreSQL.");
      console.error("Ensure the database is running and DATABASE_URL is set.");
      console.error("For local development, run: npm run db:up");
    } else {
      console.error("Failed to run migrations", error);
    }
    process.exit(1);
  });
}
