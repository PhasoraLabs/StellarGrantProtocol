import type { Meta, StoryObj } from "@storybook/react";
import { WalletAddress } from "@/components/wallet/WalletAddress";

const SAMPLE_ADDRESS =
  "GABC1234567890WXYZABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890XY23";

const meta: Meta<typeof WalletAddress> = {
  title: "Wallet/WalletAddress",
  component: WalletAddress,
  parameters: { backgrounds: { default: "dark" } },
  argTypes: {
    showCopyIcon: { control: "boolean" },
    showAvatar: { control: "boolean" },
  },
};
export default meta;

type Story = StoryObj<typeof WalletAddress>;

export const Default: Story = {
  args: { address: SAMPLE_ADDRESS },
};

export const WithAvatar: Story = {
  args: { address: SAMPLE_ADDRESS, showAvatar: true },
};

export const NoCopyIcon: Story = {
  args: { address: SAMPLE_ADDRESS, showCopyIcon: false },
};

export const AvatarNoCopy: Story = {
  args: { address: SAMPLE_ADDRESS, showAvatar: true, showCopyIcon: false },
};
