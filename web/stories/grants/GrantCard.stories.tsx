import type { Meta, StoryObj } from "@storybook/react";
import { GrantCard } from "@/components/grants/GrantCard";

const baseGrant = {
  id: 1,
  title: "AI Safety Research",
  status: 1,
  funded: 750_000_000n,
  budget: 1_000_000_000n,
  deadline: BigInt(Math.floor(Date.now() / 1000) + 86400 * 30),
  token: "native",
};

const meta: Meta<typeof GrantCard> = {
  title: "Grants/GrantCard",
  component: GrantCard,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof GrantCard>;

export const Default: Story = {
  args: { grant: baseGrant },
};

export const FullyFunded: Story = {
  args: {
    grant: { ...baseGrant, funded: 1_000_000_000n },
  },
};

export const Overdue: Story = {
  args: {
    grant: {
      ...baseGrant,
      deadline: BigInt(Math.floor(Date.now() / 1000) - 86400),
    },
  },
};

export const Cancelled: Story = {
  args: {
    grant: { ...baseGrant, status: 4 },
  },
};

export const InReview: Story = {
  args: {
    grant: { ...baseGrant, status: 2 },
  },
};

export const WithOwner: Story = {
  args: {
    grant: {
      ...baseGrant,
      owner: "GABC1234567890WXYZABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890XY23",
    },
    showOwner: true,
  },
};

export const Compact: Story = {
  args: { grant: baseGrant, compact: true },
};
