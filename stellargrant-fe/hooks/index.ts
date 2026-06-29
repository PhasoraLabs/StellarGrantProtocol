/**
 * StellarGrants React Hooks — public export index (Issue #494)
 *
 * Core SDK hooks:
 *   useStellarGrants()        — access the SDK client, logger, and sdk instance from the nearest provider
 *   useGrant(grantId)         — fetch and subscribe to a single grant with loading/error states
 *   useMyGrants()             — list grants owned or funded by the connected wallet
 *   useGrants()               — paginated list of all grants
 *
 * Usage example — minimal dashboard:
 *
 * ```tsx
 * import { StellarGrantsProvider } from "@/components/StellarGrantsProvider";
 * import { useStellarGrants, useMyGrants } from "@/hooks";
 *
 * function Dashboard() {
 *   const { sdk } = useStellarGrants();
 *   const { data, isLoading } = useMyGrants();
 *   // sdk.hydrate(data?.owned ?? []) — enables search/filter/sort helpers
 *   if (isLoading) return <p>Loading…</p>;
 *   return <ul>{data?.owned.map(g => <li key={g.id}>{g.title}</li>)}</ul>;
 * }
 *
 * export default function App() {
 *   return (
 *     <StellarGrantsProvider debug={process.env.NODE_ENV !== "production"}>
 *       <Dashboard />
 *     </StellarGrantsProvider>
 *   );
 * }
 * ```
 */

export { useRelativeTime } from "./useRelativeTime";
export { useCopyToClipboard } from "./useCopyToClipboard";
export { useAddressFormat } from "./useAddressFormat";
export { useContractEvents } from "./useContractEvents";
export { useContractTransaction } from "./useContractTransaction";
export { useFundGrant } from "./useFundGrant";
export type { UseFundGrantReturn, FundGrantParams, FundGrantResult, WalletBalance, TxStatus } from "./useFundGrant";
export { useFunders } from "./useFunders";
export { useGrant } from "./useGrant";
export type { GrantDetailData } from "./useGrant";
export { useGrantBalances } from "./useGrantBalances";
export { useGrantDraft } from "./useGrantDraft";
export { useGrantHistory } from "./useGrantHistory";
export { useGrants } from "./useGrants";
export { useIPFS } from "./useIPFS";
export { useMilestone } from "./useMilestone";
export { useMultiSig } from "./useMultiSig";
export { useMyGrants } from "./useMyGrants";
export { useNotifications } from "./useNotifications";
export { useOptimisticGrant } from "./useOptimisticGrant";
export { useReputation } from "./useReputation";
export { useStellarGrants } from "./useStellarGrants";
export { useVoting } from "./useVoting";
export { useWallet } from "./useWallet";
export { useWatchlist } from "./useWatchlist";

// Re-export the provider and context types for convenience
export { StellarGrantsProvider } from "@/components/StellarGrantsProvider";
export type { StellarGrantsProviderProps, StellarGrantsContextValue, StellarGrantsSDKConfig } from "@/components/StellarGrantsProvider";
