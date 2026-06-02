import type { Meta, StoryObj } from "@storybook/react";
import { FundingProgress } from "@/components/grants/FundingProgress";

const meta: Meta<typeof FundingProgress> = {
  title: "Grants/FundingProgress",
  component: FundingProgress,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof FundingProgress>;

export const Partial: Story = {
  args: {
    current: 750_000_000n,
    target: 1_000_000_000n,
    token: "native",
    showBreakdown: false,
  },
};

export const FullyFunded: Story = {
  args: {
    current: 1_000_000_000n,
    target: 1_000_000_000n,
    token: "native",
    showBreakdown: false,
  },
};

export const Empty: Story = {
  args: {
    current: 0n,
    target: 1_000_000_000n,
    token: "native",
    showBreakdown: false,
  },
};

export const Overfunded: Story = {
  args: {
    current: 1_200_000_000n,
    target: 1_000_000_000n,
    token: "native",
    showBreakdown: false,
  },
};
