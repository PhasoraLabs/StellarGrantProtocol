/**
 * Contract Client
 * 
 * Typed ContractClient class that wraps all StellarGrants contract methods.
 * Uses auto-generated TypeScript bindings from the contract ABI.
 */

import { nativeToScVal, xdr } from "@stellar/stellar-sdk";
import { networkPassphraseConfig } from "./client";

const contractId = process.env.NEXT_PUBLIC_CONTRACT_ID || "";

export class ContractClient {
  private _contractId: string;
  private _rpcUrl: string;
  private _networkPassphrase: string;

  constructor(config?: {
    contractId?: string;
    rpcUrl?: string;
    networkPassphrase?: string;
  }) {
    this._contractId = config?.contractId || contractId;
    this._rpcUrl = config?.rpcUrl || process.env.NEXT_PUBLIC_STELLAR_RPC_URL || "";
    this._networkPassphrase = config?.networkPassphrase || networkPassphraseConfig;
  }

  /**
   * Read-only: Fetch a grant by ID
   */
  async grantGet(_params: { grant_id: bigint }) {
    // TODO: Implement contract method calls
    throw new Error("Not implemented");
  }

  /**
   * Read-only: Fetch all milestones for a grant
   */
  async milestonesGet(_params: { grant_id: bigint }) {
    // TODO: Implement contract method calls
    throw new Error("Not implemented");
  }

  /**
   * Read-only: Get contributor reputation score
   */
  async contributorScore(_params: { address: string }) {
    // TODO: Implement contract method calls
    throw new Error("Not implemented");
  }

  /**
   * Read-only: Get reviewer list for a grant
   */
  async grantReviewers(_params: { grant_id: bigint }) {
    // TODO: Implement contract method calls
    throw new Error("Not implemented");
  }

  /**
   * Write: Create a new grant
   */
  async grantCreate(params: {
    owner: string;
    title: string;
    description: string;
    tokenAddress: string;
    totalAmount: bigint;
    milestoneAmount: bigint;
    numMilestones: number;
    reviewers: string[];
    quorum: number;
  }) {
    return {
      method: "grant_create",
      args: [
        nativeToScVal(params.owner, { type: "address" }),
        nativeToScVal(params.title),
        nativeToScVal(params.description),
        nativeToScVal(params.tokenAddress, { type: "address" }),
        nativeToScVal(params.totalAmount, { type: "i128" }),
        nativeToScVal(params.milestoneAmount, { type: "i128" }),
        nativeToScVal(params.numMilestones, { type: "u32" }),
        xdr.ScVal.scvVec(params.reviewers.map(r => nativeToScVal(r, { type: "address" }))),
        nativeToScVal(params.quorum, { type: "u32" }),
        nativeToScVal(null), // Option<Vec<u64>>
        nativeToScVal(0n, { type: "i128" }), // min_funding
        nativeToScVal(params.totalAmount, { type: "i128" }), // hard_cap
        xdr.ScVal.scvVec([]), // tags
        nativeToScVal(false), // _is_open_bounty
        nativeToScVal(false), // is_public_good
      ]
    };
  }

  /**
   * Write: Fund a grant
   */
  async grantFund(_params: {
    grant_id: string;
    token: string;
    amount: bigint;
  }) {
    // TODO: Implement contract method calls
    throw new Error("Not implemented");
  }

  /**
   * Write: Approve a SAC token (e.g. USDC) for spending by the grant contract.
   *
   * USDC on Stellar is a Soroban Asset Contract, so funding with USDC is a
   * two-step flow: first `approveToken` (this), then `grantFund`. Returns the
   * unsigned transaction XDR for the wallet to sign.
   */
  async approveToken(params: {
    tokenAddress: string;
    spender: string; // grant contract address
    amount: bigint;
    owner: string; // connected wallet address
  }): Promise<string> {
    if (!params.tokenAddress) throw new Error("tokenAddress is required");
    if (!params.spender) throw new Error("spender is required");
    if (!params.owner) throw new Error("owner is required");
    if (params.amount <= 0n) throw new Error("amount must be greater than zero");
    // TODO: build the token_approve invocation against the SAC and return XDR
    throw new Error("Not implemented");
  }

  /**
   * Write: Submit milestone proof
   */
  async milestoneSubmit(_params: {
    grant_id: string;
    milestone_idx: number;
    proof_hash: string;
  }) {
    // TODO: Implement contract method calls
    throw new Error("Not implemented");
  }

  /**
   * Write: Approve milestone
   */
  async milestoneApprove(_params: {
    grant_id: string;
    milestone_idx: number;
    reviewer: string;
  }) {
    // TODO: Implement contract method calls
    throw new Error("Not implemented");
  }

  /**
   * Write: Cast a vote on a milestone (approve or reject).
   *
   * Wraps milestoneApprove / future milestoneReject contract methods.
   * The `approve` flag maps to the correct underlying call.
   *
   * @param grantId       - Grant ID string
   * @param milestoneIdx  - Zero-based milestone index
   * @param approve       - true → approve, false → reject
   */
  async voteOnMilestone(
    grantId: string,
    milestoneIdx: number,
    approve: boolean
  ): Promise<void> {
    if (approve) {
      await this.milestoneApprove({
        grant_id: grantId,
        milestone_idx: milestoneIdx,
        reviewer: "",   // caller address injected at signing time
      });
    } else {
      // TODO: wire to milestoneReject contract method once added
      throw new Error("Milestone rejection not yet implemented in contract");
    }
  }

  /**
   * Read-only: Check if an address is a council member
   */
  async isCouncilMember(params: { address: string }): Promise<boolean> {
    const raw = process.env.NEXT_PUBLIC_COUNCIL_ADDRESSES ?? "";
    const councilSet = new Set(
      raw
        .split(",")
        .map((a) => a.trim())
        .filter(Boolean)
    );
    return councilSet.has(params.address);
  }

  /**
   * Write: Resolve milestone dispute
   */
  async resolveDispute(params: {
    grantId: string;
    milestoneIdx: number;
    approvePayout: boolean;
    councilAddress: string;
  }) {
    return {
      method: "milestone_resolve_dispute",
      args: [
        nativeToScVal(params.councilAddress, { type: "address" }),
        nativeToScVal(BigInt(params.grantId), { type: "u64" }),
        nativeToScVal(params.milestoneIdx, { type: "u32" }),
        nativeToScVal(params.approvePayout, { type: "bool" }),
      ]
    };
  }
}

// Export singleton instance
export const contractClient = new ContractClient();
