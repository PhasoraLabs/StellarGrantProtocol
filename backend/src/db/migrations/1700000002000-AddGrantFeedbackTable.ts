import { MigrationInterface, QueryRunner, Table, TableIndex, TableUnique } from "typeorm";

export class AddGrantFeedbackTable1700000002000 implements MigrationInterface {
  name = "AddGrantFeedbackTable1700000002000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.createTable(
      new Table({
        name: "grant_feedback",
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
            name: "reviewerAddress",
            type: "varchar",
            length: "120",
          },
          {
            name: "role",
            type: "varchar",
            length: "20",
          },
          {
            name: "rating",
            type: "smallint",
          },
          {
            name: "comment",
            type: "text",
            isNullable: true,
          },
          {
            name: "createdAt",
            type: "timestamp",
            default: "now()",
          },
        ],
        uniques: [
          new TableUnique({
            name: "UQ_grant_feedback_grantId_reviewerAddress",
            columnNames: ["grantId", "reviewerAddress"],
          }),
        ],
      }),
      true
    );

    await queryRunner.createIndex(
      "grant_feedback",
      new TableIndex({
        name: "IDX_grant_feedback_grantId",
        columnNames: ["grantId"],
      })
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.dropTable("grant_feedback", true);
  }
}
