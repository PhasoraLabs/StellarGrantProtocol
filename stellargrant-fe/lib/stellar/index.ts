export { getRpcClient, rpcClient, networkPassphraseConfig, getHorizonClient, horizonClient } from "./client";
export {
  ContractClient,
  contractClient,
  isNativeXlmAddress,
  getNativeXlmContractId,
  NATIVE_SENTINEL,
} from "./contract";
export { decodeScVal } from "./decode";
export { fetchContractEvents, decodeEvent } from "./events";
export type { ContractEvent } from "./events";
export { BatchBuilder } from "./batchBuilder";
export type { BatchOperation, BatchResult } from "./batchBuilder";
export {
  getGrantBalances,
  getGrantXlmBalance,
  getGrantTokenBalance,
  listenToBalanceChanges,
  parseBalanceToStroops,
  formatStroops,
} from "./balances";
export type {
  GrantBalance,
  GrantBalances,
  BalanceChangeListenerOptions,
} from "./balances";

// Multi-signature transaction support
export {
  buildUnsignedTransaction,
  combineSignatures,
  submitSignedXdr,
  isValidTransactionXdr,
  MultiSigTracker,
} from "./multisig";
export type {
  TransactionXdr,
  SignerStatus,
  SignerEntry,
  MultiSigStatus,
  BuildUnsignedTxOptions,
  SubmitOptions,
} from "./multisig";

// Grant search, filter, and sort utilities (Issue #253)
export {
  StellarGrantsSDK,
  stellarGrantsSDK,
  grantListAll,
  grantFilterByCategory,
  grantSearchByTitle,
  sortGrants,
  queryGrants,
} from "./sdk";
export type {
  PaginationOptions,
  GrantListResult,
  SortOptions,
  SortDirection,
  GrantSortField,
  QueryGrantsOptions,
  GrantWithMilestones,
} from "./sdk";

// Resilient event subscription (Issue #254)
export { subscribeToEvents } from "./subscription";
export type {
  EventHandler,
  StatusHandler,
  SubscriptionStatus,
  SubscribeOptions,
  Subscription,
} from "./subscription";

// Transaction history retrieval (Issue #256)
export { getTransactionHistory, getGrantHistory } from "./history";
export type {
  GrantOperationType,
  GrantHistoryRecord,
  HistoryOptions,
  HistoryResult,
} from "./history";

// Error mapping (Issue #250)
export {
  ErrorCode,
  ERROR_MESSAGES,
  StellarGrantsError,
  SorobanContractError,
  StellarGrantsNetworkError,
  Errors,
  parseSorobanError,
  isContractError,
  getErrorMessage,
} from "../errors";
export type { ErrorCodeValue } from "../errors";

// Native XLM funding support (Issue #504)
export {
  getNativeXlmSacId,
  isNativeXlm,
  resolveTokenAddress,
  computeRequiredBalance,
  getWalletXlmBalance,
  checkXlmSufficiency,
  buildNativeXlmFundXdr,
  STELLAR_BASE_RESERVE_STROOPS,
  TX_FEE_BUFFER_STROOPS,
} from "./xlm-native";
export type { XlmFundingRequirement, NativeXlmFundParams } from "./xlm-native";
