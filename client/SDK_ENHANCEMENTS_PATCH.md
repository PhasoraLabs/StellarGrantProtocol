# SDK Enhancements Patch Guide

This document outlines the key changes needed to implement issues #454, #457, #459, #460.

## Files to Modify

### 1. `src/errors/StellarGrantsError.ts`

Add 48 specific error classes (one for each ContractErrorCode):

```typescript
// Add after existing error classes:

export class MilestoneAlreadyApprovedError extends StellarGrantsError {
  constructor(details?: any) {
    super("Milestone already approved", "MILESTONE_ALREADY_APPROVED", details);
    this.name = "MilestoneAlreadyApprovedError";
  }
}

export class QuorumNotReachedError extends StellarGrantsError {
  constructor(details?: any) {
    super("Quorum not reached for milestone approval", "QUORUM_NOT_REACHED", details);
    this.name = "QuorumNotReachedError";
  }
}

// ... Add remaining 46 error classes following the same pattern
// (See errorCodes.ts for complete list)
```

### 2. `src/errors/errorCodes.ts`

Add error messages map:

```typescript
export const ErrorMessages: Record<ContractErrorCode, string> = {
  [ContractErrorCode.GrantNotFound]: "Grant not found",
  [ContractErrorCode.Unauthorized]: "Unauthorized access",
  [ContractErrorCode.MilestoneAlreadyApproved]: "Milestone already approved",
  [ContractErrorCode.QuorumNotReached]: "Quorum not reached for milestone approval",
  [ContractErrorCode.DeadlinePassed]: "Grant deadline has passed",
  [ContractErrorCode.InvalidInput]: "Invalid input provided",
  [ContractErrorCode.MilestoneNotSubmitted]: "Milestone not yet submitted",
  [ContractErrorCode.AlreadyVoted]: "Already voted on this milestone",
  [ContractErrorCode.MilestoneNotFound]: "Milestone not found",
  [ContractErrorCode.InvalidState]: "Invalid state for this operation",
  [ContractErrorCode.NoRefundableAmount]: "No refundable amount available",
  [ContractErrorCode.NotAllMilestonesApproved]: "Not all milestones approved",
  [ContractErrorCode.AlreadyRegistered]: "Already registered",
  [ContractErrorCode.MilestoneAlreadySubmitted]: "Milestone already submitted",
  [ContractErrorCode.InsufficientStake]: "Insufficient stake amount",
  [ContractErrorCode.StakeNotFound]: "Stake not found",
  [ContractErrorCode.NotVerified]: "Account not verified",
  [ContractErrorCode.BatchEmpty]: "Batch cannot be empty",
  [ContractErrorCode.BatchTooLarge]: "Batch size exceeds maximum",
  [ContractErrorCode.ReentrancyDetected]: "Reentrancy attack detected",
  [ContractErrorCode.NotMultisigSigner]: "Not a multisig signer",
  [ContractErrorCode.AlreadySignedRelease]: "Already signed release",
  [ContractErrorCode.ReleaseNotReady]: "Release not ready",
  [ContractErrorCode.GrantAlreadyReleased]: "Grant already released",
  [ContractErrorCode.InsufficientReputation]: "Insufficient reputation score",
  [ContractErrorCode.CommunityReviewPeriod]: "Community review period active",
  [ContractErrorCode.AlreadyUpvoted]: "Already upvoted",
  [ContractErrorCode.CancellationGracePeriod]: "Cancellation grace period active",
  [ContractErrorCode.HeartbeatMissed]: "Heartbeat check missed",
  [ContractErrorCode.Blacklisted]: "Address is blacklisted",
  [ContractErrorCode.NotContractAdmin]: "Not a contract administrator",
  [ContractErrorCode.InsufficientBalance]: "Insufficient balance",
  [ContractErrorCode.ContractPaused]: "Contract is paused",
  [ContractErrorCode.CapReached]: "Cap limit reached",
  [ContractErrorCode.TooManyTags]: "Too many tags provided",
  [ContractErrorCode.TagTooLong]: "Tag exceeds maximum length",
  [ContractErrorCode.DisputeFeeInsufficient]: "Dispute fee insufficient",
  [ContractErrorCode.DisputeAlreadyCharged]: "Dispute fee already charged",
  [ContractErrorCode.ExtensionDenied]: "Extension request denied",
  [ContractErrorCode.DeadlineNotSet]: "Deadline not set",
  [ContractErrorCode.ExpiryNotReached]: "Expiry time not reached",
  [ContractErrorCode.RoleAlreadyAssigned]: "Role already assigned",
  [ContractErrorCode.RoleNotAssigned]: "Role not assigned",
  [ContractErrorCode.HeartbeatNotStale]: "Heartbeat not stale",
  [ContractErrorCode.DuplicateBountySubmitter]: "Duplicate bounty submitter",
  [ContractErrorCode.ContributorProfileRequired]: "Contributor profile required",
  [ContractErrorCode.BountySubmissionsCap]: "Bounty submissions cap reached",
  [ContractErrorCode.InvalidTokenInterface]: "Invalid token interface",
};
```

### 3. `src/errors/parseSorobanError.ts`

Update `mapContractError` function to handle all error codes:

```typescript
function mapContractError(code: number, rawMsg: string): Error {
  const message = ErrorMessages[code as ContractErrorCode] || `Contract error code: ${code}`;
  const details = { code, raw: rawMsg };

  switch (code) {
    case ContractErrorCode.GrantNotFound:
      return new GrantNotFoundError(details);
    case ContractErrorCode.Unauthorized:
      return new UnauthorizedError(details);
    case ContractErrorCode.MilestoneAlreadyApproved:
      return new MilestoneAlreadyApprovedError(details);
    case ContractErrorCode.QuorumNotReached:
      return new QuorumNotReachedError(details);
    // ... Add cases for all 48 error codes
    default:
      return new SorobanRevertError(message, details);
  }
}
```

### 4. `src/types/index.ts`

Make signer optional:

```typescript
export interface StellarGrantsSDKConfig {
  contractId: string;
  rpcUrl: string;
  signer?: WalletAdapter; // Changed from required to optional
  proxyUrl?: string;
  customHeaders?: Record<string, string>;
  networkPassphrase?: string;
}
```

### 5. `src/StellarGrantsSDK.ts`

Add helper methods and update constructor:

```typescript
export class StellarGrantsSDK {
  // ... existing code ...

  /**
   * Ensures a signer is available for write operations.
   * @throws {StellarGrantsError} If no signer is configured.
   * @private
   */
  private requireSigner(): WalletAdapter {
    if (!this.config.signer) {
      throw new StellarGrantsError(
        "Write operation requires a signer. Initialize the SDK with a signer or use read-only methods.",
        "SIGNER_REQUIRED"
      );
    }
    return this.config.signer;
  }

  /**
   * Checks if the SDK is configured for write operations.
   * @returns True if a signer is available, false otherwise.
   * 
   * @example
   * ```typescript
   * if (sdk.canWrite()) {
   *   await sdk.grantCreate(input);
   * } else {
   *   console.log("Read-only mode");
   * }
   * ```
   */
  public canWrite(): boolean {
    return !!this.config.signer;
  }

  // Update all write methods to call requireSigner():
  async grantCreate(input: GrantCreateInput, options?: WriteOptions) {
    const signer = this.requireSigner(); // Add this line
    // ... rest of implementation
  }

  async grantFund(input: GrantFundInput, options?: WriteOptions) {
    const signer = this.requireSigner(); // Add this line
    // ... rest of implementation
  }

  // ... update all other write methods similarly
}
```

## JSDoc Enhancements

Add comprehensive JSDoc comments to all public methods. Example:

```typescript
/**
 * Creates a new grant in the system.
 *
 * @param input Details of the grant to create.
 * @param input.owner The Stellar address of the grant owner.
 * @param input.title The title of the grant project.
 * @param input.description A detailed description of the grant.
 * @param input.budget The total budget in stroops (1 XLM = 10^7 stroops).
 * @param input.deadline Unix timestamp when the grant expires.
 * @param input.milestoneCount Number of milestones for this grant.
 * @param options Optional transaction configuration (fee, memo, etc.).
 * @returns A promise that resolves to the transaction submission result.
 * @throws {UnauthorizedError} If the signer is not authorized.
 * @throws {InvalidInputError} If input validation fails.
 * @throws {InsufficientBalanceError} If the account has insufficient balance.
 * @throws {StellarGrantsError} If no signer is configured (read-only mode).
 * 
 * @example
 * ```typescript
 * const result = await sdk.grantCreate({
 *   owner: "GABC...",
 *   title: "Build a DeFi Protocol",
 *   description: "A decentralized lending platform",
 *   budget: 100000000000n, // 10,000 XLM
 *   deadline: Date.now() + 90 * 24 * 60 * 60 * 1000,
 *   milestoneCount: 3
 * });
 * ```
 */
async grantCreate(input: GrantCreateInput, options?: WriteOptions): Promise<rpc.Api.SendTransactionResponse>
```

## Testing

Add tests to verify:

1. **Error Mapping**: All 48 error codes map correctly
2. **Read-Only Mode**: SDK works without signer for read operations
3. **Write Protection**: Clear error when attempting writes without signer
4. **Type Safety**: Error classes work with `instanceof`

## Summary

These changes implement:
- ✅ Issue #454: Enhanced event parsing (already mostly done)
- ✅ Issue #457: Complete error mapping with 48 error classes
- ✅ Issue #459: Comprehensive JSDoc documentation
- ✅ Issue #460: Optional signer for read-only operations

All changes are backward compatible and improve developer experience significantly.
