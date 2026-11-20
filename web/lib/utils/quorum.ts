/**
 * Quorum helpers aligned with on-chain `governance::quorum_reached`.
 *
 * Contract rule: approvals * 2 > total_reviewers (strictly more than half).
 * Required approvals is therefore floor(n / 2) + 1.
 */

/** Fraction of reviewers that must approve (>50% majority). */
export const QUORUM_THRESHOLD = 0.5;

/**
 * Number of approvals required for a reviewer set of size `reviewerCount`.
 * Returns 0 when there are no reviewers.
 */
export function requiredApprovals(reviewerCount: number): number {
  if (reviewerCount <= 0) return 0;
  return Math.floor(reviewerCount / 2) + 1;
}

/**
 * Whether the given approval count has reached quorum for `reviewerCount`.
 * Mirrors on-chain `quorum_reached(approvals, total_reviewers)`.
 */
export function quorumReached(approvals: number, reviewerCount: number): boolean {
  if (reviewerCount <= 0) return false;
  return approvals * 2 > reviewerCount;
}
