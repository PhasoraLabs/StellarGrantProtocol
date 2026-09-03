export interface Milestone {
  id: string;
  title: string;
  description: string;
  reward: number;
  status: 'pending' | 'submitted' | 'approved' | 'paid';
}

export interface Grant {
  id: string;
  title: string;
  description: string;
  status: 'open' | 'active' | 'completed' | 'disputed' | 'cancelled';
  totalBudget: number;
  token: 'XLM' | 'USDC';
  recipientAddress: string;
  deadline: string;
  milestones: Milestone[];
  category: string;
  author: string;
  createdAt: string;
}

export const grants: Grant[] = [
  {
    id: "g1",
    title: "Stellar Asset Sandbox",
    description: "A secure environment for testing new Stellar assets with pre-configured liquidity pools.",
    status: "open",
    totalBudget: 50000,
    token: "XLM",
    recipientAddress: "GD...1111",
    deadline: "2026-12-31",
    category: "Infrastructure",
    author: "GD...AUTH1",
    createdAt: "2026-01-15",
    milestones: [
      { id: "m1a", title: "Architecture Design", description: "Define system components and security model", reward: 10000, status: "pending" },
      { id: "m1b", title: "Core Engine", description: "Implement the sandbox core", reward: 20000, status: "pending" },
      { id: "m1c", title: "User Dashboard", description: "Frontend for managing assets", reward: 20000, status: "pending" }
    ]
  },
  {
    id: "g2",
    title: "Soroban DeFi Aggregator",
    description: "Yield optimizer for Soroban-based AMMs and lending protocols.",
    status: "active",
    totalBudget: 75000,
    token: "USDC",
    recipientAddress: "GD...2222",
    deadline: "2026-10-15",
    category: "DeFi",
    author: "GD...AUTH2",
    createdAt: "2026-02-20",
    milestones: [
      { id: "m2a", title: "Protocol Integration", description: "Connect to major Soroban protocols", reward: 25000, status: "approved" },
      { id: "m2b", title: "Strategy Implementation", description: "Yield farming algorithms", reward: 25000, status: "submitted" },
      { id: "m2c", title: "Audit and Security", description: "Third-party audit", reward: 25000, status: "pending" }
    ]
  },
  {
    id: "g3",
    title: "NFT Marketplace for Artists",
    description: "A low-fee marketplace specifically designed for digital artists on Stellar.",
    status: "completed",
    totalBudget: 30000,
    token: "XLM",
    recipientAddress: "GD...3333",
    deadline: "2026-05-01",
    category: "Collectibles",
    author: "GD...AUTH3",
    createdAt: "2025-11-10",
    milestones: [
      { id: "m3a", title: "Smart Contract Deployment", description: "Deploy basic NFT contracts", reward: 10000, status: "paid" },
      { id: "m3b", title: "Marketplace UI", description: "Responsive web interface", reward: 10000, status: "paid" },
      { id: "m3c", title: "Launch and Marketing", description: "Onboard first 50 artists", reward: 10000, status: "paid" }
    ]
  },
  {
    id: "g4",
    title: "Cross-border Payment Rails",
    description: "Simplified API for micro-payments between LATAM and Southeast Asia.",
    status: "disputed",
    totalBudget: 120000,
    token: "USDC",
    recipientAddress: "GD...4444",
    deadline: "2027-01-20",
    category: "Payments",
    author: "GD...AUTH4",
    createdAt: "2026-03-05",
    milestones: [
      { id: "m4a", title: "Regulatory Compliance", description: "Obtain necessary licenses", reward: 40000, status: "approved" },
      { id: "m4b", title: "Banking Integrations", description: "Connect to local rails", reward: 40000, status: "submitted" },
      { id: "m4c", title: "Beta Trial", description: "First 1000 transactions", reward: 40000, status: "pending" }
    ]
  },
  {
    id: "g5",
    title: "Eco-friendly Gaming Platform",
    description: "Stellar-powered games where rewards are tied to real-world carbon offsetting.",
    status: "cancelled",
    totalBudget: 45000,
    token: "XLM",
    recipientAddress: "GD...5555",
    deadline: "2026-08-30",
    category: "Gaming",
    author: "GD...AUTH5",
    createdAt: "2026-04-12",
    milestones: [
      { id: "m5a", title: "Game Engine Prototype", description: "Basic gameplay loop", reward: 15000, status: "submitted" },
      { id: "m5b", title: "Stellar Integration", description: "Off-setting oracle connection", reward: 15000, status: "pending" },
      { id: "m5c", title: "Content Creation", description: "Levels and assets", reward: 15000, status: "pending" }
    ]
  }
];

// Add 42 more grants to reach 47 total for pagination testing
for (let i = 6; i <= 47; i++) {
  grants.push({
    id: `g${i}`,
    title: `Community Grant #${i}`,
    description: `A community-driven initiative for Stellar expansion project #${i}.`,
    status: i % 2 === 0 ? "active" : "open",
    totalBudget: 1000 * i,
    token: i % 3 === 0 ? "USDC" : "XLM",
    recipientAddress: `GD...ADDR${i}`,
    deadline: "2027-12-31",
    category: i % 4 === 0 ? "Education" : "Utility",
    author: `GD...AUTH${i}`,
    createdAt: "2026-05-01",
    milestones: [
      { id: `m${i}a`, title: "Phase 1", description: "Initial delivery", reward: 500 * i, status: "pending" },
      { id: `m${i}b`, title: "Phase 2", description: "Final delivery", reward: 500 * i, status: "pending" }
    ]
  });
}
