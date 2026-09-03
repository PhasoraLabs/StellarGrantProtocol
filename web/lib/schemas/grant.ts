import { z } from "zod";

export const step1Schema = z.object({
  title: z.string()
    .min(10, "Title must be at least 10 characters")
    .max(120, "Title must be at most 120 characters"),
  description: z.string()
    .min(50, "Description must be at least 50 characters")
    .max(5000, "Description must be at most 5000 characters"),
  recipientAddress: z.string().regex(/^G[A-Z2-7]{55}$/, "Invalid Stellar address"),
  totalBudget: z.number({ message: "Budget must be a number" })
    .positive("Budget must be greater than 0")
    .max(10000000, "Budget cannot exceed 10,000,000"),
  budgetToken: z.enum(["native", "USDC"], { message: "Select a funding token" }),
  deadline: z.string().refine(
    (d) => !isNaN(Date.parse(d)) && new Date(d) > new Date(),
    "Deadline must be in the future"
  ),
});

export const step2Schema = z.object({
  milestones: z.array(
    z.object({
      title: z.string()
        .min(5, "Milestone title must be at least 5 characters")
        .max(100, "Milestone title must be at most 100 characters"),
      description: z.string()
        .min(20, "Milestone description must be at least 20 characters"),
      reward: z.number({ message: "Reward must be a number" })
        .positive("Reward must be greater than 0"),
    })
  )
  .min(1, "At least one milestone required")
  .max(10, "Maximum 10 milestones allowed"),
});

export const step3Schema = z.object({
  reviewers: z.array(
    z.string().regex(/^G[A-Z2-7]{55}$/, "Invalid Stellar address")
  )
  .min(3, "Minimum 3 reviewers required")
  .max(7, "Maximum 7 reviewers")
  .refine(
    (arr) => new Set(arr).size === arr.length,
    "Duplicate reviewer addresses"
  ),
  quorum: z.number({ message: "Quorum must be a number" })
    .min(1, "Quorum must be at least 1"),
});

export const fullGrantSchema = step1Schema
  .merge(step2Schema)
  .merge(step3Schema)
  .superRefine((data, ctx) => {
    // 1. Milestone rewards sum must equal totalBudget
    const sum = data.milestones.reduce((acc, m) => acc + (m.reward || 0), 0);
    if (Math.abs(sum - data.totalBudget) > 0.0001) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["milestones"],
        message: `Sum of milestone rewards (${sum}) must equal the total budget (${data.totalBudget})`,
      });
    }

    // 2. Quorum <= reviewers.length
    if (data.quorum > data.reviewers.length) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["quorum"],
        message: `Quorum (${data.quorum}) cannot exceed the number of reviewers (${data.reviewers.length})`,
      });
    }
  });

export type GrantFormData = z.infer<typeof fullGrantSchema>;
export type Step1FormData = z.infer<typeof step1Schema>;
export type Step2FormData = z.infer<typeof step2Schema>;
export type Step3FormData = z.infer<typeof step3Schema>;
