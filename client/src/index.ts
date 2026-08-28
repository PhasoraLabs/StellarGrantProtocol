export { StellarGrantsSDK } from "./StellarGrantsSDK";
export { parseSorobanError } from "./errors/parseSorobanError";
export { ContractError, SorobanRevertError, StellarGrantsError } from "./errors/StellarGrantsError";
export { ContractErrorCode, ErrorMessages } from "./errors/errorCodes";
export type {
  GrantCreateInput,
  GrantFundInput,
  MilestoneSubmitInput,
  MilestoneVoteInput,
  StellarGrantsSDKConfig,
  StellarGrantsSigner,
  WalletAdapter,
} from "./types";

// Wallet adapters — import directly from @stellargrants/client-sdk
export { FreighterAdapter } from "./wallets/FreighterAdapter";
export { AlbedoAdapter } from "./wallets/AlbedoAdapter";
export { XBullAdapter } from "./wallets/XBullAdapter";
export { WalletConnectAdapter } from "./wallets/WalletConnectAdapter";
export { createPreferredWalletAdapter } from "./wallets/createPreferredWalletAdapter";
