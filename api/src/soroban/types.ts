export type SorobanGrant = {
  id: number;
  title: string;
  description?: string;
  status: string;
  recipient: string;
  totalAmount: string;
};

export interface SorobanContractClient {
  fetchGrants(): Promise<SorobanGrant[]>;
  fetchGrantById(id: number): Promise<SorobanGrant | null>;
}
