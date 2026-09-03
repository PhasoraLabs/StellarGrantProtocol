/**
 * StellarGrants SDK — Contract Error Codes
 *
 * Exhaustive registry of every numeric error code that the StellarGrants
 * Soroban contract can return. The numbers are taken directly from the
 * contract's `Error` enum (see `contracts/src/errors.rs`).
 *
 * Keeping codes here as a plain const enum means:
 *   1. They are erased at compile time (no runtime overhead).
 *   2. They can be used as both type and value in switch statements.
 *   3. A single source of truth shared by the mapper and the SDK.
 *
 * @module web/lib/errors/errorCodes
 */

// ── Numeric error codes ───────────────────────────────────────────────────────

export const ErrorCode = {
  // ── General / Auth ─────────────────────────────────────────────────────────
  /** Caller is not authorised to perform this operation */
  Unauthorized: 1,
  /** The contract is paused; no state-mutating calls are accepted */
  ContractPaused: 2,
  /** An arithmetic overflow or underflow occurred */
  ArithmeticError: 3,

  // ── Grant lifecycle ─────────────────────────────────────────────────────────
  /** A grant with the given ID does not exist */
  GrantNotFound: 10,
  /** A grant with the given ID already exists */
  GrantAlreadyExists: 11,
  /** The grant is not in an active state */
  GrantNotActive: 12,
  /** The grant is already fully funded and cannot receive more */
  GrantAlreadyFunded: 13,
  /** The grant has been cancelled */
  GrantCancelled: 14,
  /** The grant deadline has passed */
  GrantExpired: 15,
  /** The hard-cap would be exceeded by this contribution */
  HardCapExceeded: 16,
  /** The caller is not the grant owner */
  NotGrantOwner: 17,
  /** The requested action requires the grant to be fully funded */
  GrantNotFullyFunded: 18,

  // ── Funding / balances ──────────────────────────────────────────────────────
  /** Caller's balance is insufficient to complete the operation */
  InsufficientFunds: 20,
  /** The token is not whitelisted for this grant */
  InvalidToken: 21,
  /** The amount provided is zero or negative */
  InvalidAmount: 22,
  /** Transfer of funds failed at the token contract level */
  TransferFailed: 23,
  /** A withdrawal was attempted before the grant is cancellable */
  WithdrawNotAllowed: 24,

  // ── Milestones ──────────────────────────────────────────────────────────────
  /** The milestone index is out of range */
  MilestoneNotFound: 30,
  /** The milestone has already been approved */
  MilestoneAlreadyApproved: 31,
  /** The milestone has already had a proof submitted */
  MilestoneAlreadySubmitted: 32,
  /** Quorum of reviewers has not been reached */
  QuorumNotReached: 33,
  /** The milestone deadline has passed */
  MilestoneExpired: 34,
  /** The proof hash provided is empty or malformed */
  InvalidProofHash: 35,
  /** A payout was requested before the milestone was approved */
  MilestoneNotApproved: 36,
  /** The reviewer has already cast a vote on this milestone */
  AlreadyVoted: 37,
  /** The caller is not in the reviewer list for this grant */
  NotAReviewer: 38,

  // ── Governance / delegation ─────────────────────────────────────────────────
  /** A delegate for this grant/address pair already exists */
  DelegateAlreadySet: 40,
  /** No delegate has been configured for this grant/address pair */
  DelegateNotFound: 41,
  /** The delegatee is not authorised to vote on behalf of the delegator */
  InvalidDelegate: 42,

  // ── Reputation / disputes ───────────────────────────────────────────────────
  /** The reputation profile for this address does not exist */
  ProfileNotFound: 50,
  /** A dispute on this milestone is already open */
  DisputeAlreadyOpen: 51,
  /** No open dispute exists for this milestone */
  DisputeNotFound: 52,
  /** The dispute fee transfer failed */
  DisputeFeeFailed: 53,

  // ── Configuration ───────────────────────────────────────────────────────────
  /** An admin account has not been configured */
  AdminNotSet: 60,
  /** The requested configuration value is invalid */
  InvalidConfig: 61,
} as const;

/** Union of every valid error code number */
export type ErrorCodeValue = (typeof ErrorCode)[keyof typeof ErrorCode];

/**
 * Human-readable message for each error code.
 * Shown to developers and — after localisation — potentially to end-users.
 */
export const ERROR_MESSAGES: Record<ErrorCodeValue, string> = {
  // General
  [ErrorCode.Unauthorized]:        "You are not authorised to perform this action.",
  [ErrorCode.ContractPaused]:      "The StellarGrants contract is currently paused. Please try again later.",
  [ErrorCode.ArithmeticError]:     "An arithmetic error occurred in the contract. This is likely a bug — please report it.",

  // Grant lifecycle
  [ErrorCode.GrantNotFound]:       "Grant not found. Please check the grant ID and try again.",
  [ErrorCode.GrantAlreadyExists]:  "A grant with this ID already exists.",
  [ErrorCode.GrantNotActive]:      "This grant is not currently active.",
  [ErrorCode.GrantAlreadyFunded]:  "This grant has already reached its funding target.",
  [ErrorCode.GrantCancelled]:      "This grant has been cancelled.",
  [ErrorCode.GrantExpired]:        "This grant's deadline has passed.",
  [ErrorCode.HardCapExceeded]:     "Your contribution would exceed this grant's funding cap.",
  [ErrorCode.NotGrantOwner]:       "Only the grant owner can perform this action.",
  [ErrorCode.GrantNotFullyFunded]: "This action requires the grant to be fully funded first.",

  // Funding
  [ErrorCode.InsufficientFunds]:   "Insufficient funds to complete this transaction.",
  [ErrorCode.InvalidToken]:        "This token is not accepted for this grant.",
  [ErrorCode.InvalidAmount]:       "The amount must be greater than zero.",
  [ErrorCode.TransferFailed]:      "Token transfer failed. Please check your balance and allowance.",
  [ErrorCode.WithdrawNotAllowed]:  "Withdrawal is not permitted at this time.",

  // Milestones
  [ErrorCode.MilestoneNotFound]:        "Milestone not found.",
  [ErrorCode.MilestoneAlreadyApproved]: "This milestone has already been approved.",
  [ErrorCode.MilestoneAlreadySubmitted]:"A proof for this milestone has already been submitted.",
  [ErrorCode.QuorumNotReached]:         "The required quorum of reviewers has not been reached.",
  [ErrorCode.MilestoneExpired]:         "This milestone's deadline has passed.",
  [ErrorCode.InvalidProofHash]:         "The proof hash is invalid or empty.",
  [ErrorCode.MilestoneNotApproved]:     "This milestone has not been approved yet.",
  [ErrorCode.AlreadyVoted]:             "You have already voted on this milestone.",
  [ErrorCode.NotAReviewer]:             "You are not a registered reviewer for this grant.",

  // Delegation
  [ErrorCode.DelegateAlreadySet]: "A delegate is already configured for this grant.",
  [ErrorCode.DelegateNotFound]:   "No delegate found for this grant and address.",
  [ErrorCode.InvalidDelegate]:    "The specified delegate is not authorised to vote on your behalf.",

  // Reputation / disputes
  [ErrorCode.ProfileNotFound]:     "Reputation profile not found for this address.",
  [ErrorCode.DisputeAlreadyOpen]:  "A dispute for this milestone is already open.",
  [ErrorCode.DisputeNotFound]:     "No open dispute found for this milestone.",
  [ErrorCode.DisputeFeeFailed]:    "The dispute fee transfer failed. Please check your balance.",

  // Configuration
  [ErrorCode.AdminNotSet]:    "No admin account has been configured for this contract.",
  [ErrorCode.InvalidConfig]:  "An invalid configuration value was provided.",
};
