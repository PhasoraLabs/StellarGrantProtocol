import type { Meta, StoryObj } from "@storybook/react";
import { StatusDot } from "@/components/ui/StatusDot";

const meta: Meta<typeof StatusDot> = {
  title: "UI/StatusDot",
  component: StatusDot,
  parameters: { backgrounds: { default: "dark" } },
  argTypes: {
    status: {
      control: "select",
      options: ["active", "inactive", "pending", "success", "danger"],
    },
  },
};
export default meta;

type Story = StoryObj<typeof StatusDot>;

export const Active: Story = { args: { status: "active", label: "Active" } };
export const Inactive: Story = { args: { status: "inactive", label: "Inactive" } };
export const Pending: Story = { args: { status: "pending", label: "Pending" } };
export const Success: Story = { args: { status: "success", label: "Success" } };
export const Danger: Story = { args: { status: "danger", label: "Danger" } };
