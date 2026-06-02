import type { Meta, StoryObj } from "@storybook/react";
import { MilestoneTimeline } from "@/components/milestones/MilestoneTimeline";
import type { Milestone } from "@/types";

const baseMilestone = (
  idx: number,
  overrides: Partial<Milestone> = {}
): Milestone => ({
  idx,
  title: `Milestone ${idx + 1}`,
  description: `Deliverables for milestone ${idx + 1} of the grant.`,
  amount: 250_000_000n,
  submitted: false,
  paid: false,
  proof_hash: "",
  deadline: BigInt(Math.floor(Date.now() / 1000) + 86400 * (idx + 1) * 14),
  ...overrides,
});

const meta: Meta<typeof MilestoneTimeline> = {
  title: "Milestones/MilestoneTimeline",
  component: MilestoneTimeline,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof MilestoneTimeline>;

export const Default: Story = {
  args: {
    grantId: "1",
    milestones: [
      baseMilestone(0),
      baseMilestone(1),
      baseMilestone(2),
    ],
  },
};

export const WithSubmitted: Story = {
  args: {
    grantId: "1",
    milestones: [
      baseMilestone(0, { submitted: true, paid: true, proof_hash: "Qm123" }),
      baseMilestone(1, { submitted: true, proof_hash: "Qm456" }),
      baseMilestone(2),
    ],
    reviewers: ["GABC…XY23", "GBCD…ZA34"],
    quorum: 2,
  },
};

export const Empty: Story = {
  args: { grantId: "1", milestones: [] },
};
