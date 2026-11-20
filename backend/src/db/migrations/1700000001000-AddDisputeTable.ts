import { MigrationInterface, QueryRunner, Table, TableIndex } from "typeorm";

export class AddDisputeTable1700000001000 implements MigrationInterface {
  name = "AddDisputeTable1700000001000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.createTable(
      new Table({
        name: "disputes",
        columns: [
          {
            name: "id",
            type: "int",
            isPrimary: true,
            isGenerated: true,
            generationStrategy: "increment",
          },
          {
            name: "grantId",
            type: "int",
          },
          {
            name: "milestoneIdx",
            type: "int",
          },
          {
            name: "status",
            type: "varchar",
            length: "30",
            default: "'open'",
          },
          {
            name: "contributorArgument",
            type: "text",
            isNullable: true,
          },
          {
            name: "funderArgument",
            type: "text",
            isNullable: true,
          },
          {
            name: "resolvedBy",
            type: "varchar",
            length: "120",
            isNullable: true,
          },
          {
            name: "resolutionTxHash",
            type: "varchar",
            length: "64",
            isNullable: true,
          },
          {
            name: "openedAt",
            type: "timestamp",
            default: "now()",
          },
          {
            name: "resolvedAt",
            type: "timestamp",
            isNullable: true,
          },
        ],
      }),
      true
    );

    await queryRunner.createIndex(
      "disputes",
      new TableIndex({
        name: "IDX_disputes_grantId",
        columnNames: ["grantId"],
      })
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.dropTable("disputes", true);
  }
}
