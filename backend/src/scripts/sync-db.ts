import { buildDataSource } from "../db/data-source";

const run = async () => {
  const dataSource = buildDataSource();
  await dataSource.initialize();
  await dataSource.destroy();
  console.log("Database schema synchronized.");
};

run().catch((error) => {
  console.error("Failed to sync schema", error);
  process.exit(1);
});
