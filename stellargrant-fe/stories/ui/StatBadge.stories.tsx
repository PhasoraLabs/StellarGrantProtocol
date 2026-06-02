import type { Meta, StoryObj } from "@storybook/react";
import { StatBadge } from "@/components/ui/StatBadge";

const meta: Meta<typeof StatBadge> = {
  title: "UI/StatBadge",
  component: StatBadge,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof StatBadge>;

export const Reputation: Story = {
  args: { label: "Reputation", value: "94" },
};

export const Grants: Story = {
  args: { label: "Grants Completed", value: "12" },
};

export const Earned: Story = {
  args: { label: "Total Earned", value: "45,000 XLM" },
};
