import type { Meta, StoryObj } from "@storybook/react";
import { GrantStatusBadge } from "@/components/grants/GrantStatusBadge";

const meta: Meta<typeof GrantStatusBadge> = {
  title: "Grants/GrantStatusBadge",
  component: GrantStatusBadge,
  parameters: { backgrounds: { default: "dark" } },
  argTypes: {
    status: { control: { type: "number", min: 0, max: 4 } },
  },
};
export default meta;

type Story = StoryObj<typeof GrantStatusBadge>;

export const Pending: Story = { args: { status: 0 } };
export const Active: Story = { args: { status: 1 } };
export const InProgress: Story = { args: { status: 2 } };
export const Completed: Story = { args: { status: 3 } };
export const Cancelled: Story = { args: { status: 4 } };
