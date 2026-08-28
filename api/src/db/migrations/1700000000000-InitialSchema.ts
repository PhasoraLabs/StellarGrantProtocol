import { MigrationInterface, QueryRunner, Table, TableIndex, TableForeignKey, TableUnique } from "typeorm";

export class InitialSchema1700000000000 implements MigrationInterface {
  name = "InitialSchema1700000000000";

  public async up(queryRunner: QueryRunner): Promise<void> {
    // Create users table
    await queryRunner.createTable(
      new Table({
        name: "users",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "stellarAddress", type: "varchar", length: "56", isUnique: true },
          { name: "email", type: "varchar", isNullable: true },
          { name: "notifyMilestoneApproved", type: "boolean", default: false },
          { name: "notifyMilestoneSubmitted", type: "boolean", default: false },
          { name: "githubId", type: "varchar", isNullable: true },
          { name: "githubUsername", type: "varchar", isNullable: true },
          { name: "twitterId", type: "varchar", isNullable: true },
          { name: "twitterUsername", type: "varchar", isNullable: true },
        ],
      }),
      true
    );

    // Create roles table
    await queryRunner.createTable(
      new Table({
        name: "roles",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "name", type: "varchar", length: "50", isUnique: true },
          { name: "description", type: "text", isNullable: true },
          { name: "permissions", type: "text", isNullable: true },
          { name: "createdAt", type: "timestamp", default: "now()" },
          { name: "updatedAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );

    // Create user_roles table
    await queryRunner.createTable(
      new Table({
        name: "user_roles",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "user_id", type: "int" },
          { name: "role_id", type: "int" },
          { name: "assigned_by", type: "varchar", length: "120", isNullable: true },
          { name: "created_at", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createUniqueConstraint("user_roles", new TableUnique({ columnNames: ["user_id", "role_id"] }));
    await queryRunner.createForeignKey(
      "user_roles",
      new TableForeignKey({
        columnNames: ["user_id"],
        referencedColumnNames: ["id"],
        referencedTableName: "users",
        onDelete: "CASCADE",
      })
    );
    await queryRunner.createForeignKey(
      "user_roles",
      new TableForeignKey({
        columnNames: ["role_id"],
        referencedColumnNames: ["id"],
        referencedTableName: "roles",
        onDelete: "CASCADE",
      })
    );

    // Create communities table
    await queryRunner.createTable(
      new Table({
        name: "communities",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "name", type: "varchar", length: "120", isUnique: true },
          { name: "description", type: "text", isNullable: true },
          { name: "logoUrl", type: "text", isNullable: true },
          { name: "adminAddresses", type: "text", isNullable: true },
          { name: "featured", type: "boolean", default: false },
          { name: "createdAt", type: "timestamp", default: "now()" },
          { name: "updatedAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );

    // Create contributors table
    await queryRunner.createTable(
      new Table({
        name: "contributors",
        columns: [
          { name: "address", type: "varchar", length: "120", isPrimary: true },
          { name: "reputation", type: "int", default: 0 },
          { name: "totalGrantsCompleted", type: "int", default: 0 },
          { name: "isBlacklisted", type: "boolean", default: false },
          { name: "email", type: "varchar", length: "254", isNullable: true },
          { name: "bio", type: "text", isNullable: true },
          { name: "profilePictureUrl", type: "varchar", length: "2048", isNullable: true },
          { name: "githubUrl", type: "varchar", length: "2048", isNullable: true },
          { name: "twitterUrl", type: "varchar", length: "2048", isNullable: true },
          { name: "linkedinUrl", type: "varchar", length: "2048", isNullable: true },
          { name: "emailNotifications", type: "boolean", default: true },
          { name: "updatedAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createIndex("contributors", new TableIndex({ name: "IDX_contributors_reputation", columnNames: ["reputation"] }));

    // Create grants table
    await queryRunner.createTable(
      new Table({
        name: "grants",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "title", type: "varchar", length: "200" },
          { name: "description", type: "text", isNullable: true },
          { name: "status", type: "varchar", length: "50" },
          { name: "owner", type: "varchar", length: "120" },
          { name: "recipient", type: "varchar", length: "120" },
          { name: "communityId", type: "int", isNullable: true },
          { name: "totalAmount", type: "varchar", length: "60" },
          { name: "tags", type: "text", isNullable: true },
          { name: "updatedAt", type: "timestamp", default: "now()" },
          { name: "createdAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createForeignKey(
      "grants",
      new TableForeignKey({
        columnNames: ["communityId"],
        referencedColumnNames: ["id"],
        referencedTableName: "communities",
        onDelete: "SET NULL",
      })
    );

    // Create milestones table
    await queryRunner.createTable(
      new Table({
        name: "milestones",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "grantId", type: "int" },
          { name: "idx", type: "int" },
          { name: "title", type: "varchar", length: "200" },
          { name: "description", type: "text", isNullable: true },
          { name: "deadline", type: "varchar", length: "40" },
          { name: "lastDeadlineReminderAt", type: "varchar", length: "40", isNullable: true },
          { name: "lastDeadlineReminderDaysBefore", type: "int", isNullable: true },
          { name: "overdueNotifiedAt", type: "varchar", length: "40", isNullable: true },
        ],
      }),
      true
    );
    await queryRunner.createUniqueConstraint("milestones", new TableUnique({ columnNames: ["grantId", "idx"] }));
    await queryRunner.createForeignKey(
      "milestones",
      new TableForeignKey({
        columnNames: ["grantId"],
        referencedColumnNames: ["id"],
        referencedTableName: "grants",
        onDelete: "CASCADE",
      })
    );

    // Create milestone_proofs table
    await queryRunner.createTable(
      new Table({
        name: "milestone_proofs",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "grantId", type: "int" },
          { name: "milestoneIdx", type: "int" },
          { name: "proofCid", type: "varchar", length: "255" },
          { name: "description", type: "text", isNullable: true },
          { name: "submittedBy", type: "varchar", length: "120" },
          { name: "signature", type: "varchar", length: "255" },
          { name: "nonce", type: "varchar", length: "80" },
          { name: "createdAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createUniqueConstraint("milestone_proofs", new TableUnique({ columnNames: ["grantId", "milestoneIdx"] }));
    await queryRunner.createForeignKey(
      "milestone_proofs",
      new TableForeignKey({
        columnNames: ["grantId"],
        referencedColumnNames: ["id"],
        referencedTableName: "grants",
        onDelete: "CASCADE",
      })
    );

    // Create milestone_approvals table
    await queryRunner.createTable(
      new Table({
        name: "milestone_approvals",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "grantId", type: "int" },
          { name: "milestoneIdx", type: "int" },
          { name: "reviewerStellarAddress", type: "varchar", length: "56" },
          { name: "approved", type: "boolean" },
          { name: "createdAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );

    // Create milestone_comments table
    await queryRunner.createTable(
      new Table({
        name: "milestone_comments",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "milestoneId", type: "int" },
          { name: "content", type: "text" },
          { name: "authorAddress", type: "varchar", length: "120" },
          { name: "parentCommentId", type: "int", isNullable: true },
          { name: "createdAt", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createForeignKey(
      "milestone_comments",
      new TableForeignKey({
        columnNames: ["milestoneId"],
        referencedColumnNames: ["id"],
        referencedTableName: "milestones",
        onDelete: "CASCADE",
      })
    );

    // Create grant_reviewers table
    await queryRunner.createTable(
      new Table({
        name: "grant_reviewers",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "grantId", type: "int" },
          { name: "reviewerStellarAddress", type: "varchar", length: "56" },
        ],
      }),
      true
    );
    await queryRunner.createForeignKey(
      "grant_reviewers",
      new TableForeignKey({
        columnNames: ["grantId"],
        referencedColumnNames: ["id"],
        referencedTableName: "grants",
        onDelete: "CASCADE",
      })
    );

    // Create activities table
    await queryRunner.createTable(
      new Table({
        name: "activities",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "type", type: "varchar", length: "50" },
          { name: "entityType", type: "varchar", length: "50" },
          { name: "entityId", type: "int", isNullable: true },
          { name: "actorAddress", type: "varchar", length: "120", isNullable: true },
          { name: "data", type: "json", isNullable: true },
          { name: "timestamp", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createIndex("activities", new TableIndex({ name: "IDX_activities_timestamp", columnNames: ["timestamp"] }));
    await queryRunner.createIndex("activities", new TableIndex({ name: "IDX_activities_type", columnNames: ["type"] }));
    await queryRunner.createIndex("activities", new TableIndex({ name: "IDX_activities_entity", columnNames: ["entityType", "entityId"] }));

    // Create webhook_subscriptions table
    await queryRunner.createTable(
      new Table({
        name: "webhook_subscriptions",
        columns: [
          { name: "id", type: "int", isPrimary: true, isGenerated: true, generationStrategy: "increment" },
          { name: "target_url", type: "varchar", length: "2048" },
          { name: "secret_key", type: "varchar", length: "255" },
          { name: "events", type: "text", isNullable: true },
          { name: "is_active", type: "boolean", default: true },
          { name: "failure_count", type: "int", default: 0 },
          { name: "max_retries", type: "int", default: 5 },
          { name: "community_id", type: "int", isNullable: true },
          { name: "owner_address", type: "varchar", length: "56", isNullable: true },
          { name: "created_by", type: "int" },
          { name: "created_at", type: "timestamp", default: "now()" },
          { name: "updated_at", type: "timestamp", default: "now()" },
        ],
      }),
      true
    );
    await queryRunner.createForeignKey(
      "webhook_subscriptions",
      new TableForeignKey({
        columnNames: ["created_by"],
        referencedColumnNames: ["id"],
        referencedTableName: "users",
        onDelete: "CASCADE",
      })
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.dropTable("webhook_subscriptions", true);
    await queryRunner.dropTable("activities", true);
    await queryRunner.dropTable("grant_reviewers", true);
    await queryRunner.dropTable("milestone_comments", true);
    await queryRunner.dropTable("milestone_approvals", true);
    await queryRunner.dropTable("milestone_proofs", true);
    await queryRunner.dropTable("milestones", true);
    await queryRunner.dropTable("grants", true);
    await queryRunner.dropTable("contributors", true);
    await queryRunner.dropTable("communities", true);
    await queryRunner.dropTable("user_roles", true);
    await queryRunner.dropTable("roles", true);
    await queryRunner.dropTable("users", true);
  }
}
