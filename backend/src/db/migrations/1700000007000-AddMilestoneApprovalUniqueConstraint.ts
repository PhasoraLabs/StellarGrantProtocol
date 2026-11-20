import { MigrationInterface, QueryRunner } from "typeorm";

export class AddMilestoneApprovalUniqueConstraint1700000007000 implements MigrationInterface {
  name = "AddMilestoneApprovalUniqueConstraint1700000007000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    // Keep the newest vote before adding the constraint to installations that
    // already contain duplicate rows from the unauthenticated endpoint.
    await queryRunner.query(`
      DELETE FROM "milestone_approvals" older
      USING "milestone_approvals" newer
      WHERE older."grantId" = newer."grantId"
        AND older."milestoneIdx" = newer."milestoneIdx"
        AND older."reviewerStellarAddress" = newer."reviewerStellarAddress"
        AND older."id" < newer."id"
    `);
    await queryRunner.query(
      `ALTER TABLE "milestone_approvals" ADD CONSTRAINT "UQ_milestone_approvals_vote" UNIQUE ("grantId", "milestoneIdx", "reviewerStellarAddress")`,
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `ALTER TABLE "milestone_approvals" DROP CONSTRAINT "UQ_milestone_approvals_vote"`,
    );
  }
}
