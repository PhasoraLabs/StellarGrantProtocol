/**
 * Global TypeScript Types
 *
 * Shared type definitions for the StellarGrants frontend.
 */

/**
 * Token metadata for display and formatting
 */
export interface TokenMetadata {
  address: string;
  symbol: string;
  decimals: number;
  name?: string;
}

/**
 * Token amount with metadata for display
 */
export interface TokenAmount {
  token: string; // token address
  amount: bigint;
  symbol?: string;
  decimals?: number;
}

export interface Grant {
  id: string;
  owner: string;
  recipient: string; // Grant recipient address
  title: string;
  description: string;
  budget: bigint;
  funded: bigint;
  deadline: bigint;
  status: number; // 0: Pending, 1: Active, 2: In Progress, 3: Complete, 4: Cancelled
  milestones: number;
  reviewers: string[];
  created_at: bigint;
  token?: string; // Primary token address for the grant
  contractAddress?: string; // Soroban contract account address for live balance polling
}

export interface Milestone {
  idx: number;
  title: string;
  description: string;
  proof_hash: string | null;
  submitted: boolean;
  approved: boolean;
  paid: boolean;
  submitted_at: bigint | null;
  approved_at: bigint | null;
  paid_at: bigint | null;
  token?: string; // Token address for this milestone's payout
  amount?: bigint; // Payout amount for this milestone
  votes?: MilestoneVote[]; // Reviewer votes cast on this milestone
  // UI-computed fields optionally hydrated by the API layer
  overdue?: boolean;
  daysUntilDeadline?: number;
}

export interface MilestoneVote {
  reviewer: string;
  vote: "approve" | "reject" | null;
  voted_at: bigint | null;
}

export interface Contributor {
  address: string;
  github_handle: string | null;
  skills: string[];
  reputation_score: number;
  grants_participated: number;
  milestones_completed: number;
}

export interface ContractEvent {
  type: string;
  data: Record<string, unknown>;
  ledger: number;
  timestamp: Date;
}
