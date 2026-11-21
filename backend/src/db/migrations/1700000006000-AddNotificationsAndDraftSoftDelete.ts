import { MigrationInterface, QueryRunner, Table } from "typeorm";

export class AddNotificationsAndDraftSoftDelete1700000006000 implements MigrationInterface {
  name = "AddNotificationsAndDraftSoftDelete1700000006000";

  async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`ALTER TABLE "grants" ADD COLUMN "deletedAt" timestamp`);
    await queryRunner.createTable(new Table({
      name: "notifications",
      columns: [
        { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
        { name: "address", type: "varchar", length: "120" },
        { name: "type", type: "varchar", length: "80" },
        { name: "payload", type: "text" },
        { name: "readAt", type: "timestamp", isNullable: true },
        { name: "createdAt", type: "timestamp", default: "now()" },
      ],
    }), true);
    await queryRunner.query(`CREATE INDEX "IDX_notifications_address_read" ON "notifications" ("address", "readAt")`);
  }

  async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.dropTable("notifications", true);
    await queryRunner.query(`ALTER TABLE "grants" DROP COLUMN "deletedAt"`);
  }
}
