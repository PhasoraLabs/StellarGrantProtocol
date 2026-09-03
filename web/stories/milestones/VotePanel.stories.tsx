import type { Meta, StoryObj } from "@storybook/react";
import { VotePanel } from "@/components/milestones/VotePanel";

/**
 * VotePanel stories require the wallet store and contract hooks.
 * We use static reviewer/vote data; Freighter signing is not triggered
 * in Storybook — action buttons are visible but submitting is a no-op.
 */

const REVIEWERS = [
  "GABC1234567890WXYZABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890XY23",
  "GBCD1234567890WXYZABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ZA34",
  "GCDE1234567890WXYZABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890AB45",
];

const meta: Meta<typeof VotePanel> = {
  title: "Milestones/VotePanel",
  component: VotePanel,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof VotePanel>;

/** Non-reviewer wallet — vote buttons are hidden. */
export const NonReviewer: Story = {
  args: {
    grantId: "1",
    milestoneIdx: 0,
    reviewers: REVIEWERS,
    quorum: 2,
  },
};

/** Reviewer wallet — Approve / Reject buttons are visible. */
export const ReviewerCanVote: Story = {
  args: {
    grantId: "1",
    milestoneIdx: 0,
    reviewers: REVIEWERS,
    quorum: 2,
  },
};

/** Reviewer who has already voted. */
export const AlreadyVoted: Story = {
  args: {
    grantId: "2",
    milestoneIdx: 1,
    reviewers: REVIEWERS,
    quorum: 2,
  },
};

/** No reviewers assigned. */
export const NoReviewers: Story = {
  args: {
    grantId: "3",
    milestoneIdx: 0,
    reviewers: [],
    quorum: 0,
  },
};

/** Quorum already reached. */
export const QuorumReached: Story = {
  args: {
    grantId: "4",
    milestoneIdx: 0,
    reviewers: REVIEWERS,
    quorum: 2,
    threshold: 0.67,
  },
};
