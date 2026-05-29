/**
 * Create Grant Page
 * 
 * Multi-step form for creating a new grant on-chain.
 * Wallet connection required.
 * 
 * Form steps:
 * 1. Basic Info - title, description, category
 * 2. Budget - funding token, target amount, platform fee
 * 3. Timeline - start date, deadline
 * 4. Milestones - dynamic list of milestone titles and proof types
 * 5. Reviewers - add reviewer Stellar addresses (min 1, max 7)
 * 6. Review & Sign - summary card + Freighter signing prompt
 */

import { CreateGrantForm } from "@/components/grants/CreateGrantForm";

export default function CreateGrantPage() {
  return (
    <div className="container mx-auto px-4 py-8">
      <h1 className="font-orbitron text-3xl text-text-primary mb-2">Create Grant</h1>
      <p className="font-mono text-sm text-text-muted mb-8 max-w-xl">
        Define milestones and allocate your budget. The chart updates as you type.
      </p>
      <CreateGrantForm />
    </div>
  );
}
