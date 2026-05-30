import dataSource from "../db/typeorm.config";

const run = async () => {
  await dataSource.initialize();
  await dataSource.runMigrations();
  await dataSource.destroy();
  console.log("Database migrations applied successfully.");
};

run().catch((error) => {
  if (error?.code === "ECONNREFUSED") {
    console.error(
      "Failed to run migrations: cannot connect to PostgreSQL."
    );
    console.error(
      "Ensure the database is running and DATABASE_URL is set."
    );
    console.error(
      "For local development, run: npm run db:up"
    );
  } else {
    console.error("Failed to run migrations", error);
  }
  process.exit(1);
});
