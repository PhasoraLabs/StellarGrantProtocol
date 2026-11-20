export interface CreateGrantMilestone {
  title: string;
  description?: string;
  reward: number;
}

export interface CreateGrantFormValues {
  title: string;
  description: string;
  totalBudget: number;
  token: string;
  milestones: CreateGrantMilestone[];
}

export const defaultCreateGrantValues: CreateGrantFormValues = {
  title: "",
  description: "",
  totalBudget: 1000,
  token: "XLM",
  milestones: [
    { title: "Phase 1", description: "", reward: 0 },
    { title: "Phase 2", description: "", reward: 0 },
  ],
};
