import { z } from "zod";

import { WebhookEventType } from "../entities/WebhookSubscription";

// ---------------------------------------------------------------------------
// Common Validation Schemas
// ---------------------------------------------------------------------------

/**
 * Stellar address validation
 */
export const stellarAddressSchema = z.string().min(10).max(120);

/**
 * Pagination query parameters
 */
export const paginationSchema = z.object({
  page: z.coerce.number().int().positive().default(1),
  limit: z.coerce.number().int().min(1).max(100).default(20),
});

/**
 * ID parameter validation
 */
export const idParamSchema = z.object({
  id: z.coerce.number().int().positive(),
});

/**
 * Grant ID parameter validation
 */
export const grantIdParamSchema = z.object({
  id: z.coerce.number().int().positive(),
});

/**
 * Community ID parameter validation
 */
export const communityIdParamSchema = z.object({
  id: z.coerce.number().int().positive(),
});

/**
 * Milestone ID parameter validation
 */
export const milestoneIdParamSchema = z.object({
  id: z.coerce.number().int().positive(),
});

/**
 * Stellar address parameter validation
 */
export const addressParamSchema = z.object({
  address: stellarAddressSchema,
});

/**
 * Signature validation for authenticated requests
 */
export const signatureSchema = z.object({
  address: stellarAddressSchema,
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
});

/**
 * URL validation
 */
export const urlSchema = z.string().url().max(2048);

/**
 * GitHub URL validation
 */
export const githubUrlSchema = urlSchema.refine(
  (u: string) => /^https:\/\/(www\.)?github\.com\/[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$/.test(u),
  { message: "Invalid GitHub profile URL" },
);

/**
 * Twitter/X URL validation
 */
export const twitterUrlSchema = urlSchema.refine(
  (u: string) => /^https:\/\/(x\.com|twitter\.com)\/[A-Za-z0-9_]{1,15}$/.test(u),
  { message: "Invalid Twitter/X profile URL" },
);

/**
 * LinkedIn URL validation
 */
export const linkedinUrlSchema = urlSchema.refine(
  (u: string) => /^https:\/\/(www\.)?linkedin\.com\/in\/[A-Za-z0-9-_%]+\/?$/.test(u),
  { message: "Invalid LinkedIn profile URL" },
);

// ---------------------------------------------------------------------------
// Route-Specific Schemas
// ---------------------------------------------------------------------------

/**
 * User registration schema
 */
export const userRegisterSchema = z.object({
  email: z.string().email().max(255),
  stellarAddress: stellarAddressSchema,
  notifyMilestoneApproved: z.boolean().optional(),
  notifyMilestoneSubmitted: z.boolean().optional(),
});

/**
 * Grant report schema
 */
export const grantReportSchema = z.object({
  reporterAddress: stellarAddressSchema,
  reason: z.string().min(1).max(1000),
});

/**
 * Grant fund schema
 */
export const grantFundSchema = z.object({
  funderAddress: stellarAddressSchema,
  amount: z.string().min(1),
});

/**
 * Community create schema
 */
export const communityCreateSchema = z.object({
  name: z.string().min(2).max(120),
  description: z.string().max(2000).optional(),
  logoUrl: urlSchema.max(2000).optional(),
  adminAddresses: z.array(stellarAddressSchema).max(20).optional(),
  featured: z.boolean().optional(),
});

/**
 * Community update schema
 */
export const communityUpdateSchema = z.object({
  description: z.string().max(2000).optional(),
  logoUrl: urlSchema.nullable().optional(),
  featured: z.boolean().optional(),
});

/**
 * Milestone comment create schema
 */
export const milestoneCommentCreateSchema = z.object({
  content: z.string().trim().min(1).max(4000),
  authorAddress: stellarAddressSchema,
  parentCommentId: z.number().int().positive().optional(),
});

/**
 * Profile update schema
 */
export const profileUpdateSchema = signatureSchema.extend({
  bio: z.string().max(500).nullable().optional(),
  profilePictureUrl: urlSchema.nullable().optional(),
  githubUrl: githubUrlSchema.nullable().optional(),
  twitterUrl: twitterUrlSchema.nullable().optional(),
  linkedinUrl: linkedinUrlSchema.nullable().optional(),
});

/**
 * Milestone proof schema
 */
export const milestoneProofSchema = z.object({
  grantId: z.number().int().positive(),
  milestoneIdx: z.number().int().nonnegative(),
  proofCid: z.string().min(3).max(255),
  description: z.string().optional(),
  submittedBy: stellarAddressSchema,
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
});

/**
 * Admin bulk action schema
 */
export const adminBulkActionSchema = z.object({
  grantIds: z.array(z.number().int().positive()).min(1).max(100),
  action: z.enum(["approve", "reject", "flag"]),
});

/**
 * Admin blacklist schema
 */
export const adminBlacklistSchema = z.object({
  blacklist: z.boolean(),
});

/**
 * Admin config schema
 */
export const adminConfigSchema = z.object({
  feePercentage: z.number().min(0).max(100),
});

/**
 * Watchlist add schema
 */
export const watchlistAddSchema = z.object({
  grantId: z.coerce.number().int().positive(),
});

/**
 * Grant list query schema
 */
export const grantListQuerySchema = paginationSchema.extend({
  sortBy: z.enum(["updatedAt", "totalAmount", "id"]).default("id"),
  order: z.enum(["ASC", "DESC"]).default("ASC"),
  status: z.string().trim().toLowerCase().optional(),
  funder: z.string().trim().optional(),
  tags: z.string().optional(),
  communityId: z.coerce.number().int().positive().optional(),
});

/**
 * Search query schema
 */
export const searchQuerySchema = z.object({
  q: z.string().min(2).max(100),
});

// ---------------------------------------------------------------------------
// Webhook Schemas
// ---------------------------------------------------------------------------

const WEBHOOK_EVENT_VALUES = Object.values(WebhookEventType) as [string, ...string[]];

/**
 * Webhook subscription creation schema
 */
export const webhookSubscriptionCreateSchema = z.object({
  targetUrl: z.string().url().max(2048),
  secretKey: z.string().min(16).max(255),
  events: z.array(z.enum(WEBHOOK_EVENT_VALUES)).min(1).max(20),
  communityId: z.coerce.number().int().positive().optional(),
});

/**
 * Webhook subscription update schema
 */
export const webhookSubscriptionUpdateSchema = z.object({
  targetUrl: z.string().url().max(2048).optional(),
  secretKey: z.string().min(16).max(255).optional(),
  events: z.array(z.enum(WEBHOOK_EVENT_VALUES)).min(1).max(20).optional(),
  isActive: z.boolean().optional(),
});

/**
 * Webhook test schema
 */
export const webhookTestSchema = z.object({
  targetUrl: z.string().url().max(2048),
  secretKey: z.string().min(16).max(255),
  event: z.enum(WEBHOOK_EVENT_VALUES),
});
