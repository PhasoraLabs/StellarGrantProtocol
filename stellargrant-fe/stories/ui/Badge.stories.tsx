import type { Meta, StoryObj } from "@storybook/react";
import { Badge } from "@/components/ui/Badge";

const meta: Meta<typeof Badge> = {
  title: "UI/Badge",
  component: Badge,
  parameters: { backgrounds: { default: "dark" } },
  argTypes: {
    variant: {
      control: "select",
      options: ["default", "success", "danger", "warning", "info", "muted"],
    },
    size: { control: "radio", options: ["sm", "md"] },
  },
};
export default meta;

type Story = StoryObj<typeof Badge>;

export const Default: Story = { args: { children: "Default" } };
export const Success: Story = { args: { variant: "success", children: "Active" } };
export const Danger: Story = { args: { variant: "danger", children: "Cancelled" } };
export const Warning: Story = { args: { variant: "warning", children: "Testnet" } };
export const Info: Story = { args: { variant: "info", children: "In Review" } };
export const Muted: Story = { args: { variant: "muted", children: "Archived" } };
export const Small: Story = { args: { variant: "warning", size: "sm", children: "Testnet" } };
