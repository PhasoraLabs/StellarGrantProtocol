import { MigrationInterface, QueryRunner } from "typeorm";

/**
 * Enforce a unique (grantId, reviewerStellarAddress) pair on grant_reviewers
 * (#1230) so the same reviewer cannot be added to a grant more than once even
 * when the schema is managed through migrations rather than synchronize.
 */
export class AddGrantReviewerUniqueConstraint1700000006000 implements MigrationInterface {
  name = "AddGrantReviewerUniqueConstraint1700000006000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `ALTER TABLE "grant_reviewers" ADD CONSTRAINT "UQ_grant_reviewers_grantId_reviewerStellarAddress" UNIQUE ("grantId", "reviewerStellarAddress")`,
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `ALTER TABLE "grant_reviewers" DROP CONSTRAINT "UQ_grant_reviewers_grantId_reviewerStellarAddress"`,
    );
  }
}
