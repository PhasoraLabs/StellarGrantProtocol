import { z } from "zod";

export const GrantBasicInfoSchema = z.object({
  title: z
    .string()
    .min(10, "Title must be at least 10 characters")
    .max(120, "Title must be at most 120 characters"),
  description: z
    .string()
    .min(50, "Description must be at least 50 characters")
    .max(2000, "Description must be at most 2000 characters"),
  category: z.string().optional(),
});

export const GrantBudgetSchema = z.object({
  token: z.enum(["XLM", "USDC"], { message: "Select a funding token" }),
  target: z.coerce
    .bigint()
    .refine((v) => v > BigInt(0), "Target amount must be greater than 0"),
  platformFee: z.coerce.number().min(0).max(100).default(2),
});

export const GrantTimelineSchema = z.object({
  startDate: z.coerce
    .date()
    .refine((d) => d >= new Date(), "Start date cannot be in the past"),
  deadline: z.coerce
    .date()
    .refine((d) => d > new Date(), "Deadline must be in the future"),
});

export const GrantMilestoneSchema = z.object({
  milestones: z
    .array(
      z.object({
        title: z
          .string()
          .min(5, "Milestone title must be at least 5 characters"),
        description: z
          .string()
          .min(20, "Milestone description must be at least 20 characters"),
        proofType: z.enum(["ipfs", "github", "manual"]).default("ipfs"),
      }),
    )
    .min(1, "At least one milestone is required")
    .max(20, "Maximum 20 milestones allowed"),
});

export const GrantReviewersSchema = z.object({
  reviewers: z
    .array(z.string().regex(/^G[A-Z0-9]{55}$/, "Invalid Stellar address"))
    .min(1, "At least one reviewer is required")
    .max(7, "Maximum 7 reviewers allowed"),
});

export const CompleteGrantSchema = GrantBasicInfoSchema.merge(GrantBudgetSchema)
  .merge(GrantTimelineSchema)
  .merge(GrantMilestoneSchema)
  .merge(GrantReviewersSchema);
