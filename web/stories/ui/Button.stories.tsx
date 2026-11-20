import type { Meta, StoryObj } from "@storybook/react";
import { Button } from "@/components/ui/Button";

const meta: Meta<typeof Button> = {
  title: "UI/Button",
  component: Button,
  parameters: { backgrounds: { default: "dark" } },
  argTypes: {
    variant: { control: "radio", options: ["primary", "ghost"] },
  },
};
export default meta;

type Story = StoryObj<typeof Button>;

export const Primary: Story = {
  args: { variant: "primary", children: "Connect Wallet" },
};

export const Ghost: Story = {
  args: { variant: "ghost", children: "Browse Grants" },
};

export const WithHref: Story = {
  args: { variant: "primary", href: "/grants", children: "View Grants" },
};

export const Disabled: Story = {
  args: { variant: "primary", children: "Unavailable", disabled: true },
};

export const Loading: Story = {
  args: { variant: "primary", children: "Submitting…", disabled: true },
};
