import { MigrationInterface, QueryRunner, Table, TableIndex, TableUnique } from "typeorm";

export class AddMilestoneAppealTable1700000003000 implements MigrationInterface {
  name = "AddMilestoneAppealTable1700000003000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.createTable(
      new Table({
        name: "milestone_appeals",
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
            name: "reason",
            type: "text",
          },
          {
            name: "status",
            type: "varchar",
            length: "30",
            default: "'pending'",
          },
          {
            name: "reviewerVotes",
            type: "json",
            default: "'[]'",
          },
          {
            name: "appellantAddress",
            type: "varchar",
            length: "120",
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
      "milestone_appeals",
      new TableIndex({
        name: "IDX_milestone_appeals_grantId",
        columnNames: ["grantId"],
      })
    );

    await queryRunner.createUniqueConstraint(
      "milestone_appeals",
      new TableUnique({
        name: "UQ_milestone_appeals_grantId_milestoneIdx",
        columnNames: ["grantId", "milestoneIdx"],
      })
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.dropTable("milestone_appeals", true);
  }
}
