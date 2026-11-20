import { MigrationInterface, QueryRunner } from "typeorm";

export class AddGrantDraftFields1700000004000 implements MigrationInterface {
  name = "AddGrantDraftFields1700000004000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `ALTER TABLE "grants" ADD COLUMN "isDraft" boolean DEFAULT false`
    );
    await queryRunner.query(
      `ALTER TABLE "grants" ADD COLUMN "draftData" json`
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `ALTER TABLE "grants" DROP COLUMN "draftData"`
    );
    await queryRunner.query(
      `ALTER TABLE "grants" DROP COLUMN "isDraft"`
    );
  }
}
