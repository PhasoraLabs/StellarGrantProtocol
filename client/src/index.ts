export { StellarGrantsSDK } from "./StellarGrantsSDK";
export { parseSorobanError } from "./errors/parseSorobanError";
export { ContractError, SorobanRevertError, StellarGrantsError } from "./errors/StellarGrantsError";
export { TransactionTimeoutError } from "./errors/TransactionTimeoutError";
export { TransactionFailedError } from "./errors/TransactionFailedError";
export { ContractErrorCode, ErrorMessages } from "./errors/errorCodes";
export type {
  GrantCreateInput,
  GrantFundInput,
  MilestoneSubmitInput,
  MilestoneVoteInput,
  StellarGrantsSDKConfig,
  StellarGrantsSigner,
  WalletAdapter,
  TransactionResult,
  WaitForTransactionOptions,
  TransactionPollingStatus,
} from "./types";

// IPFS helpers — Task #488
export { uploadMetadataToIPFS, fetchMetadataFromIPFS } from "./ipfs";
export type { IpfsUploadConfig, IpfsUploadResult } from "./types";

// Batch operations — Issue #491
export { BatchBuilder, BatchOperationError } from "./batch/BatchBuilder";

// Optimistic UI utilities — Task #487
export { TransactionTracker } from "./utils/TransactionTracker";
export { OptimisticStateManager } from "./utils/OptimisticStateManager";
export type { TransactionStage, TransactionTrackerEvents, OptimisticUpdate } from "./utils/TransactionTracker";

// Multi-sig utilities — Issue #484
export { combineSignatures } from "./utils/transactions";

// Balance monitoring — Issue #489
export type {
  GrantBalance,
  GrantBalances,
  BalanceChangeListenerOptions,
} from "./types";

// Transaction history — Issue #483
export type {
  GrantOperationType,
  GrantHistoryRecord,
  HistoryOptions,
  HistoryResult,
} from "./types";

// Vue 3 composables — Issue #500
export {
  useStellarGrants,
  useGrants,
  useGrant,
  useGrantBalances,
  useTransactionHistory,
  useGrantHistory,
  provideStellarGrants,
} from "./composables";
export type {
  StellarGrantsContext,
  UseGrantsOptions,
  UseGrantsResult,
  UseGrantOptions,
  UseGrantResult,
  UseGrantBalancesOptions,
  UseGrantBalancesResult,
  UseTransactionHistoryOptions,
  UseTransactionHistoryResult,
  UseGrantHistoryOptions,
  UseGrantHistoryResult,
} from "./composables";

// Wallet adapters — import directly from @stellargrants/client-sdk
export { FreighterAdapter } from "./wallets/FreighterAdapter";
export { AlbedoAdapter } from "./wallets/AlbedoAdapter";
export { XBullAdapter } from "./wallets/XBullAdapter";
export { WalletConnectAdapter } from "./wallets/WalletConnectAdapter";
export { createPreferredWalletAdapter } from "./wallets/createPreferredWalletAdapter";
