import { SorobanContractClient, SorobanGrant, SorobanContractEvent } from "./types";

const mockGrants: SorobanGrant[] = [
  {
    id: 1,
    title: "Open Source Grants Q2",
    status: "active",
    recipient: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD",
    totalAmount: "250000000",
    owner: "GOWNEREXAMPLEADDRESS0000000000000000000000000000000000",
    milestones: [
      {
        idx: 0,
        title: "Phase 1: Planning",
        description: "Establish project scope and requirements.",
        deadline: "2026-06-06",
      },
      {
        idx: 1,
        title: "Phase 2: Development",
        description: "Build core features.",
        deadline: "2026-06-02",
      },
      {
        idx: 2,
        title: "Phase 3: Testing",
        description: "QA and testing phase.",
        deadline: "2026-05-31",
      },
      {
        idx: 3,
        title: "Phase 4: Deployment",
        description: "Deploy to production.",
        deadline: "2026-05-30",
      },
    ],
  },
  {
    id: 2,
    title: "Climate Data Tools",
    status: "review",
    recipient: "GCBQ6JQXQTVV7T7OUVPR4Q6PGACCUAKS6S2YDG3YQYQYRR2NJB5A6NAA",
    totalAmount: "100000000",
    owner: "GOWNEREXAMPLEADDRESS1111111111111111111111111111111111",
    milestones: [
      {
        idx: 0,
        title: "Proof-of-concept milestone",
        description: "Build and validate initial climate data tooling.",
        deadline: "2026-06-15",
      },
    ],
  },
  {
    id: 3,
    title: "Data Privacy Research",
    status: "completed",
    recipient: "GDATAPRIVACYRECPNT0000000000000000000000000000000000000",
    totalAmount: "150000000",
    owner: "GOWNEREXAMPLEADDRESS2222222222222222222222222222222222",
    milestones: [
      {
        idx: 0,
        title: "Research milestone",
        description: "Collect and analyze privacy-preserving datasets.",
        deadline: "2026-06-20",
      },
    ],
  },
  {
    id: 4,
    title: "Educational Outreach",
    status: "active",
    recipient: "GEDUCATIONOUTREACHRECIPIENT0000000000000000000000000",
    totalAmount: "50000000",
    owner: "GOWNEREXAMPLEADDRESS3333333333333333333333333333333333333",
    milestones: [
      {
        idx: 0,
        title: "Outreach milestone",
        description: "Launch community workshops and outreach sessions.",
        deadline: "2026-07-10",
      },
    ],
  },
];

const mockEvents: SorobanContractEvent[] = [
  {
    id: "evt-1",
    grantId: 1,
    type: "grant_created",
    actorAddress: "GDUMMYACTORADDRESS0000000000000000000000000000000000",
    ledger: 900,
    data: { details: "Grant created on chain" },
  },
  {
    id: "evt-2",
    grantId: 2,
    type: "grant_funded",
    actorAddress: "GDUMMYACTORADDRESS1111111111111111111111111111111111",
    ledger: 901,
    data: { amount: "1000" },
  },
];

export class MockSorobanContractClient implements SorobanContractClient {
  async fetchGrants(): Promise<SorobanGrant[]> {
    return mockGrants;
  }

  async fetchGrantById(id: number): Promise<SorobanGrant | null> {
    return mockGrants.find((grant) => grant.id === id) ?? null;
  }

  async getLatestLedger(): Promise<number> {
    return 1000;
  }

  async fetchEvents(fromLedger: number, toLedger: number): Promise<SorobanContractEvent[]> {
    return mockEvents.filter((event) => event.ledger >= fromLedger && event.ledger <= toLedger);
  }
}
