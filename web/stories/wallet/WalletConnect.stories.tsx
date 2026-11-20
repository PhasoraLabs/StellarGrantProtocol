import type { Meta, StoryObj } from "@storybook/react";
import { WalletConnect } from "@/components/wallet/WalletConnect";

/**
 * WalletConnect renders differently depending on Freighter's installation
 * and connection state (managed via useWalletStore). In Storybook the wallet
 * store starts empty, so the component renders in its "not connected" state.
 */

const meta: Meta<typeof WalletConnect> = {
  title: "Wallet/WalletConnect",
  component: WalletConnect,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof WalletConnect>;

/**
 * Default render — Freighter not detected or not connected.
 * The button label will be "Connect" or "Install Freighter" depending
 * on whether the extension is present in the browser running Storybook.
 */
export const Default: Story = {};
