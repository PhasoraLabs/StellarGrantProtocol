import type { Meta, StoryObj } from "@storybook/react";
import { AddressInput } from "@/components/ui/AddressInput";

const meta: Meta<typeof AddressInput> = {
  title: "UI/AddressInput",
  component: AddressInput,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof AddressInput>;

export const Empty: Story = {
  args: { label: "Recipient Address", onChange: () => {} },
};

export const Valid: Story = {
  args: {
    label: "Recipient Address",
    value: "GABC1234567890WXYZABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890XY23",
    onChange: () => {},
  },
};

export const Invalid: Story = {
  args: {
    label: "Recipient Address",
    value: "not-a-valid-address",
    onChange: () => {},
  },
};

export const Disabled: Story = {
  args: {
    label: "Recipient Address",
    value: "GABC1234567890WXYZABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890XY23",
    disabled: true,
    onChange: () => {},
  },
};
