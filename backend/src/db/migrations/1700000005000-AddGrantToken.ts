import { MigrationInterface, QueryRunner } from "typeorm";

export class AddGrantToken1700000005000 implements MigrationInterface {
  name = "AddGrantToken1700000005000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `ALTER TABLE "grants" ADD COLUMN IF NOT EXISTS "token" varchar(120) DEFAULT 'native'`,
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`ALTER TABLE "grants" DROP COLUMN IF EXISTS "token"`);
  }
}
