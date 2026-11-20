import { MigrationInterface, QueryRunner } from "typeorm";

/**
 * One dispute per (grantId, milestoneIdx) (#1200) and one approval per
 * (grantId, milestoneIdx, reviewerStellarAddress) (#1201). Existing duplicates
 * are collapsed first (oldest dispute kept, latest vote kept) so the
 * constraints can be applied to populated databases.
 */
export class AddDisputeAndApprovalUniqueConstraints1700000008000 implements MigrationInterface {
  name = "AddDisputeAndApprovalUniqueConstraints1700000008000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `DELETE FROM "disputes" WHERE "id" NOT IN (SELECT MIN("id") FROM "disputes" GROUP BY "grantId", "milestoneIdx")`,
    );
    await queryRunner.query(
      `ALTER TABLE "disputes" ADD CONSTRAINT "UQ_disputes_grantId_milestoneIdx" UNIQUE ("grantId", "milestoneIdx")`,
    );

    await queryRunner.query(
      `DELETE FROM "milestone_approvals" WHERE "id" NOT IN (SELECT MAX("id") FROM "milestone_approvals" GROUP BY "grantId", "milestoneIdx", "reviewerStellarAddress")`,
    );
    await queryRunner.query(
      `ALTER TABLE "milestone_approvals" ADD CONSTRAINT "UQ_milestone_approvals_grantId_milestoneIdx_reviewer" UNIQUE ("grantId", "milestoneIdx", "reviewerStellarAddress")`,
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`ALTER TABLE "milestone_approvals" DROP CONSTRAINT "UQ_milestone_approvals_grantId_milestoneIdx_reviewer"`);
    await queryRunner.query(`ALTER TABLE "disputes" DROP CONSTRAINT "UQ_disputes_grantId_milestoneIdx"`);
  }
}
