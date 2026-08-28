/**
 * @file errorCodes.ts
 *
 * Canonical mapping of every error code emitted by the StellarGrant Soroban
 * contract (`ContractError` enum in types.rs).
 *
 * The u32 discriminant values here MUST stay in sync with the `#[repr(u32)]`
 * values defined in:
 *   stellargrant-contracts/contracts/stellar-grants/src/types.rs
 *
 * When the contract gains new error variants add them here in the same order
 * and with the same numeric value.
 */

/**
 * Every error code the StellarGrant contract can return.
 *
 * Values are the `#[repr(u32)]` discriminants produced by the
 * `#[contracterror]` macro.  A Soroban host error message containing
 * `Error(Contract, #N)` maps to the variant whose value equals N.
 */
export enum ContractErrorCode {
  GrantNotFound             = 1,
  Unauthorized              = 2,
  MilestoneAlreadyApproved  = 3,
  QuorumNotReached          = 4,
  DeadlinePassed            = 5,
  InvalidInput              = 6,
  MilestoneNotSubmitted     = 7,
  AlreadyVoted              = 8,
  MilestoneNotFound         = 9,
  InvalidState              = 10,
  NoRefundableAmount        = 11,
  GrantAlreadyReleased      = 12,
  NotMultisigSigner         = 13,
  AlreadySignedRelease      = 14,
  NotAllMilestonesApproved  = 15,
  InsufficientStake         = 16,
  StakeNotFound             = 17,
  AlreadyRegistered         = 18,
  BatchEmpty                = 19,
  BatchTooLarge             = 20,
  MilestoneAlreadySubmitted = 21,
}

/**
 * Human-readable message for every `ContractErrorCode`.
 *
 * These messages are surfaced directly to application code and end-users via
 * `ContractError.message`.  Keep them concise and actionable.
 */
export const ErrorMessages: Record<ContractErrorCode, string> = {
  [ContractErrorCode.GrantNotFound]:
    "The specified grant was not found.",

  [ContractErrorCode.Unauthorized]:
    "You are not authorized to perform this action.",

  [ContractErrorCode.MilestoneAlreadyApproved]:
    "This milestone has already been approved.",

  [ContractErrorCode.QuorumNotReached]:
    "The required quorum of reviewers has not been reached.",

  [ContractErrorCode.DeadlinePassed]:
    "The deadline for this operation has already passed.",

  [ContractErrorCode.InvalidInput]:
    "The provided input is invalid.",

  [ContractErrorCode.MilestoneNotSubmitted]:
    "The milestone has not been submitted yet.",

  [ContractErrorCode.AlreadyVoted]:
    "You have already cast your vote.",

  [ContractErrorCode.MilestoneNotFound]:
    "The specified milestone was not found.",

  [ContractErrorCode.InvalidState]:
    "The grant or milestone is in an invalid state for this operation.",

  [ContractErrorCode.NoRefundableAmount]:
    "There is no refundable amount available.",

  [ContractErrorCode.GrantAlreadyReleased]:
    "The grant funds have already been released.",

  [ContractErrorCode.NotMultisigSigner]:
    "You are not an authorised multisig signer for this grant.",

  [ContractErrorCode.AlreadySignedRelease]:
    "You have already signed the release for this grant.",

  [ContractErrorCode.NotAllMilestonesApproved]:
    "All milestones must be approved before this action.",

  [ContractErrorCode.InsufficientStake]:
    "Insufficient stake to perform this action.",

  [ContractErrorCode.StakeNotFound]:
    "Reviewer stake not found.",

  [ContractErrorCode.AlreadyRegistered]:
    "The contributor is already registered.",

  [ContractErrorCode.BatchEmpty]:
    "The provided batch is empty.",

  [ContractErrorCode.BatchTooLarge]:
    "The provided batch exceeds the maximum allowed size.",

  [ContractErrorCode.MilestoneAlreadySubmitted]:
    "The milestone has already been submitted.",
};
