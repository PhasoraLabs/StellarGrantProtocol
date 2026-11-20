import { DataSource, MigrationInterface } from "typeorm";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  getPendingMigrations,
  hasDryRunFlag,
  runMigrations,
} from "../../src/scripts/run-migrations";

class TestPendingMigration1700000000000 implements MigrationInterface {
  name = "TestPendingMigration1700000000000";

  async up(): Promise<void> {}

  async down(): Promise<void> {}
}

const buildTestDataSource = (migrations: Function[] = []) =>
  new DataSource({
    type: "sqljs",
    location: "memory",
    autoSave: false,
    entities: [],
    migrations,
    synchronize: false,
  });

describe("run-migrations --dry-run", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("recognises the --dry-run flag", () => {
    expect(hasDryRunFlag(["node", "run-migrations.ts"])).toBe(false);
    expect(hasDryRunFlag(["node", "run-migrations.ts", "--dry-run"])).toBe(true);
    // Flags passed through npm run migration:run -- --dry-run
    expect(hasDryRunFlag(["node", "run-migrations.ts", "--", "--dry-run"])).toBe(true);
  });

  it("reports migrations that have not been applied", async () => {
    const connection = buildTestDataSource([TestPendingMigration1700000000000]);
    await connection.initialize();

    try {
      await expect(getPendingMigrations(connection)).resolves.toEqual([
        "TestPendingMigration1700000000000",
      ]);
    } finally {
      await connection.destroy();
    }
  });

  it("prints pending migrations without applying them in dry-run mode", async () => {
    const connection = buildTestDataSource([TestPendingMigration1700000000000]);
    const apply = vi.spyOn(connection, "runMigrations");
    const log = vi.spyOn(console, "log").mockImplementation(() => {});

    const pending = await runMigrations(connection, { dryRun: true });

    expect(pending).toEqual(["TestPendingMigration1700000000000"]);
    expect(apply).not.toHaveBeenCalled();
    expect(log).toHaveBeenCalledWith("Pending migrations (1):");
    expect(log).toHaveBeenCalledWith("  - TestPendingMigration1700000000000");
    expect(connection.isInitialized).toBe(false);
  });

  it("applies migrations in normal mode", async () => {
    const connection = buildTestDataSource([TestPendingMigration1700000000000]);
    const apply = vi.spyOn(connection, "runMigrations").mockResolvedValue([]);
    const log = vi.spyOn(console, "log").mockImplementation(() => {});

    const pending = await runMigrations(connection);

    expect(pending).toEqual(["TestPendingMigration1700000000000"]);
    expect(apply).toHaveBeenCalledTimes(1);
    expect(log).toHaveBeenCalledWith("Database migrations applied successfully.");
    expect(connection.isInitialized).toBe(false);
  });

  it("reports when nothing is pending", async () => {
    const connection = buildTestDataSource();
    const log = vi.spyOn(console, "log").mockImplementation(() => {});

    await expect(runMigrations(connection, { dryRun: true })).resolves.toEqual([]);
    expect(log).toHaveBeenCalledWith("No pending migrations.");
  });
});
