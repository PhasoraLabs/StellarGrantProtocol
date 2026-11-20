import { MigrationInterface, QueryRunner, TableIndex } from "typeorm";

export class AddGrantQueryIndexes1700000005000 implements MigrationInterface {
  name = "AddGrantQueryIndexes1700000005000";
  async up(queryRunner: QueryRunner): Promise<void> {
    for (const [name, column] of [["IDX_grants_status", "status"], ["IDX_grants_recipient", "recipient"], ["IDX_grants_updated_at", "updatedAt"]] as const) {
      await queryRunner.createIndex("grants", new TableIndex({ name, columnNames: [column] }));
    }
  }
  async down(queryRunner: QueryRunner): Promise<void> {
    for (const name of ["IDX_grants_status", "IDX_grants_recipient", "IDX_grants_updated_at"]) await queryRunner.dropIndex("grants", name);
  }
}
