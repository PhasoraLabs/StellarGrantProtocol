import { PostgreSqlContainer, StartedPostgreSqlContainer } from "@testcontainers/postgresql";
import { DataSource } from "typeorm";
import { buildDataSource } from "../../src/db/data-source";

export class TestDbSetup {
  private container!: StartedPostgreSqlContainer;
  private dataSource!: DataSource;

  async start(): Promise<DataSource> {
    this.container = await new PostgreSqlContainer("postgres:15-alpine")
      .withDatabase("stellargrant_test")
      .withUsername("test")
      .withPassword("test")
      .start();

    const url = `postgres://test:test@${this.container.getHost()}:${this.container.getMappedPort(5432)}/stellargrant_test`;
    
    this.dataSource = buildDataSource(url);
    await this.dataSource.initialize();
    
    return this.dataSource;
  }

  async stop(): Promise<void> {
    if (this.dataSource?.isInitialized) {
      await this.dataSource.destroy();
    }
    if (this.container) {
      await this.container.stop();
    }
  }

  getDataSource(): DataSource {
    return this.dataSource;
  }
}
