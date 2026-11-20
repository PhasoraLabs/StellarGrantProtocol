export type SorobanContractEvent = {
  id: string;
  grantId: number;
  type: string;
  actorAddress: string;
  ledger: number;
  data: Record<string, unknown>;
};

export type SorobanMilestone = {
  idx: number;
  title: string;
  description?: string;
  deadline: string;
};

export type SorobanGrant = {
  id: number;
  title: string;
  status: string;
  recipient: string;
  totalAmount: string;
  owner?: string;
  /** Funding token — "native" or a Stellar asset/SAC address. */
  token?: string;
  milestones?: SorobanMilestone[];
};

export type GrantFetchResult = {
  grants: SorobanGrant[];
  failedIds: number[];
};

export interface SorobanContractClient {
  fetchGrants(): Promise<GrantFetchResult>;
  fetchGrantById(id: number): Promise<SorobanGrant | null>;
  getLatestLedger(): Promise<number>;
  fetchEvents(fromLedger: number, toLedger: number): Promise<SorobanContractEvent[]>;
}
