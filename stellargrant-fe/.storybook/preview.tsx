import React from "react";
import type { Preview, Decorator } from "@storybook/react";
import "../app/globals.css";

/**
 * Wrap every story in the dark space-mission canvas with design-system fonts.
 * This mirrors the real app layout so components render identically.
 */
const withDarkBackground: Decorator = (Story) => (
  <div
    style={{
      fontFamily: "var(--font-ibm-plex-mono, monospace)",
      background: "#050A14",
      padding: "24px",
      minHeight: "100vh",
    }}
  >
    <Story />
  </div>
);

const preview: Preview = {
  decorators: [withDarkBackground],
  parameters: {
    backgrounds: {
      default: "dark",
      values: [
        { name: "dark", value: "#050A14" },
        { name: "surface", value: "#111D35" },
      ],
    },
    layout: "padded",
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },
  },
};

export default preview;
