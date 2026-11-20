import { MigrationInterface, QueryRunner, Table, TableForeignKey, TableIndex } from "typeorm";

const id = { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" as const };
const createdAt = { name: "createdAt", type: "timestamp", default: "now()" };

/**
 * Creates the tables for the entities that were registered with TypeORM but
 * never provisioned by a migration (#1197), so a database built only through
 * `npm run migration:run` has every table the app queries.
 */
export class AddMissingEntityTables1700000007000 implements MigrationInterface {
  name = "AddMissingEntityTables1700000007000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.createTable(
      new Table({
        name: "audit_logs",
        columns: [
          id,
          { name: "adminAddress", type: "varchar", length: "120" },
          { name: "action", type: "varchar", length: "50" },
          { name: "target", type: "text", isNullable: true },
          { name: "details", type: "text", isNullable: true },
          createdAt,
        ],
      }),
      true
    );

    await queryRunner.createTable(
      new Table({
        name: "user_watchlist",
        columns: [
          { name: "address", type: "varchar", length: "120", isPrimary: true },
          { name: "grantId", type: "int", isPrimary: true },
          createdAt,
        ],
        indices: [
          new TableIndex({ name: "IDX_user_watchlist_address", columnNames: ["address"] }),
          new TableIndex({ name: "IDX_user_watchlist_grant_id", columnNames: ["grantId"] }),
          new TableIndex({ name: "IDX_user_watchlist_unique", columnNames: ["address", "grantId"], isUnique: true }),
        ],
      }),
      true
    );

    await queryRunner.createTable(
      new Table({
        name: "grant_views",
        columns: [
          id,
          { name: "grantId", type: "int" },
          { name: "viewerKey", type: "varchar", length: "64" },
          { name: "hourBucket", type: "varchar", length: "13" },
          createdAt,
        ],
        indices: [
          new TableIndex({ name: "IDX_grant_views_grant_id", columnNames: ["grantId"] }),
          new TableIndex({ name: "IDX_grant_views_created_at", columnNames: ["createdAt"] }),
          new TableIndex({ name: "IDX_grant_views_dedup", columnNames: ["grantId", "viewerKey", "hourBucket"], isUnique: true }),
        ],
      }),
      true
    );

    await queryRunner.createTable(
      new Table({
        name: "reconciliation_checkpoints",
        columns: [
          { name: "name", type: "varchar", length: "50", isPrimary: true },
          { name: "lastLedger", type: "bigint" },
          { name: "updatedAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );

    await queryRunner.createTable(
      new Table({
        name: "rate_limit_logs",
        columns: [
          id,
          { name: "ip", type: "varchar", length: "64" },
          { name: "path", type: "varchar", length: "255" },
          { name: "method", type: "varchar", length: "16" },
          { name: "userAgent", type: "varchar", length: "255", isNullable: true },
          { name: "address", type: "varchar", length: "120", isNullable: true },
          createdAt,
        ],
      }),
      true
    );

    await queryRunner.createTable(
      new Table({
        name: "grant_history",
        columns: [
          id,
          { name: "grantId", type: "int", isNullable: true },
          { name: "snapshot", type: "text" },
          { name: "diff", type: "text" },
          createdAt,
        ],
      }),
      true
    );
    await queryRunner.createForeignKey(
      "grant_history",
      new TableForeignKey({
        name: "FK_grant_history_grantId",
        columnNames: ["grantId"],
        referencedTableName: "grants",
        referencedColumnNames: ["id"],
        onDelete: "SET NULL",
      })
    );

    await queryRunner.createTable(
      new Table({
        name: "fee_collections",
        columns: [
          id,
          { name: "grantId", type: "int" },
          { name: "funderAddress", type: "varchar", length: "120" },
          { name: "token", type: "varchar", length: "30" },
          { name: "totalContribution", type: "varchar", length: "60" },
          { name: "feeAmount", type: "varchar", length: "60" },
          { name: "feePercentage", type: "varchar", length: "30" },
          createdAt,
        ],
      }),
      true
    );

    await queryRunner.createTable(
      new Table({
        name: "platform_configs",
        columns: [
          { name: "key", type: "varchar", length: "50", isPrimary: true },
          { name: "value", type: "varchar", length: "255" },
          { name: "updatedAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );

    await queryRunner.createTable(
      new Table({
        name: "reports",
        columns: [
          id,
          { name: "reporterAddress", type: "varchar", length: "120" },
          { name: "grantId", type: "int", isNullable: true },
          { name: "reason", type: "text" },
          { name: "status", type: "varchar", length: "30", default: "'pending'" },
          createdAt,
        ],
      }),
      true
    );
    await queryRunner.createForeignKey(
      "reports",
      new TableForeignKey({
        name: "FK_reports_grantId",
        columnNames: ["grantId"],
        referencedTableName: "grants",
        referencedColumnNames: ["id"],
        onDelete: "SET NULL",
      })
    );

    await queryRunner.createTable(
      new Table({
        name: "webhook_delivery_logs",
        columns: [
          id,
          { name: "subscription_id", type: "int" },
          { name: "event_type", type: "varchar", length: "64" },
          { name: "payload", type: "text" },
          { name: "payload_signature", type: "varchar", length: "128" },
          { name: "status", type: "varchar", length: "20", default: "'pending'" },
          { name: "attempt_count", type: "int", default: "0" },
          { name: "http_status_code", type: "int", isNullable: true },
          { name: "response_body", type: "text", isNullable: true },
          { name: "error_message", type: "text", isNullable: true },
          { name: "next_retry_at", type: "varchar", length: "100", isNullable: true },
          { name: "created_at", type: "timestamp", default: "now()" },
          { name: "updated_at", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createForeignKey(
      "webhook_delivery_logs",
      new TableForeignKey({
        name: "FK_webhook_delivery_logs_subscription_id",
        columnNames: ["subscription_id"],
        referencedTableName: "webhook_subscriptions",
        referencedColumnNames: ["id"],
        onDelete: "CASCADE",
      })
    );

    await queryRunner.createTable(
      new Table({
        name: "reputation_logs",
        columns: [
          id,
          { name: "address", type: "varchar", length: "120" },
          { name: "gain", type: "int" },
          { name: "timestamp", type: "timestamp", default: "now()" },
        ],
        indices: [
          new TableIndex({ name: "IDX_reputation_logs_address", columnNames: ["address"] }),
          new TableIndex({ name: "IDX_reputation_logs_timestamp", columnNames: ["timestamp"] }),
        ],
      }),
      true
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    for (const table of [
      "reputation_logs",
      "webhook_delivery_logs",
      "reports",
      "platform_configs",
      "fee_collections",
      "grant_history",
      "rate_limit_logs",
      "reconciliation_checkpoints",
      "grant_views",
      "user_watchlist",
      "audit_logs",
    ]) {
      await queryRunner.dropTable(table, true);
    }
  }
}
