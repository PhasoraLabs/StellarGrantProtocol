import type { Meta, StoryObj } from "@storybook/react";
import { StellarGrantsProvider } from "@/components/StellarGrantsProvider";

const meta: Meta<typeof StellarGrantsProvider> = {
  title: "Components/StellarGrantsProvider",
  component: StellarGrantsProvider,
  parameters: { backgrounds: { default: "dark" } },
};

export default meta;
type Story = StoryObj<typeof StellarGrantsProvider>;

export const Default: Story = {
  args: {
    children: (
      <div style={{ padding: "24px", color: "#E8EDF5", fontFamily: "monospace" }}>
        <h3 style={{ fontFamily: "var(--font-orbitron, sans-serif)", marginBottom: 8 }}>
          StellarGrantsProvider
        </h3>
        <p>Provider is initialized with default configuration.</p>
        <p style={{ marginTop: 12, fontSize: 12, color: "#6B7FA3" }}>
          Log level: warn (silent in production)
        </p>
      </div>
    ),
  },
};

export const WithDebug: Story = {
  args: {
    debug: true,
    children: (
      <div style={{ padding: "24px", color: "#E8EDF5", fontFamily: "monospace" }}>
        <h3 style={{ fontFamily: "var(--font-orbitron, sans-serif)", marginBottom: 8 }}>
          Debug Mode Enabled
        </h3>
        <p>Provider is configured with debug logging enabled.</p>
        <p style={{ marginTop: 12, fontSize: 12, color: "#6B7FA3" }}>
          Log level: debug
        </p>
        <p style={{ marginTop: 8, fontSize: 12, color: "#6B7FA3" }}>
          All SDK operations will output verbose logs to the console.
        </p>
      </div>
    ),
  },
};

export const WithCustomLogLevel: Story = {
  args: {
    logLevel: "info",
    children: (
      <div style={{ padding: "24px", color: "#E8EDF5", fontFamily: "monospace" }}>
        <h3 style={{ fontFamily: "var(--font-orbitron, sans-serif)", marginBottom: 8 }}>
          Custom Log Level
        </h3>
        <p>Provider is configured with custom log level.</p>
        <p style={{ marginTop: 12, fontSize: 12, color: "#6B7FA3" }}>
          Log level: info
        </p>
        <p style={{ marginTop: 8, fontSize: 12, color: "#6B7FA3" }}>
          SDK will output info messages and above.
        </p>
      </div>
    ),
  },
};

export const WithChildren: Story = {
  args: {
    children: (
      <div style={{ padding: "24px" }}>
        <div
          style={{
            backgroundColor: "#1a1f2e",
            borderRadius: "4px",
            padding: "16px",
            marginBottom: "16px",
            color: "#E8EDF5",
            fontFamily: "monospace",
          }}
        >
          <p style={{ margin: 0, marginBottom: 8, fontWeight: "bold" }}>Child Component 1</p>
          <p style={{ margin: 0, fontSize: 12, color: "#6B7FA3" }}>
            Consumes StellarGrants context via useStellarGrants()
          </p>
        </div>
        <div
          style={{
            backgroundColor: "#1a1f2e",
            borderRadius: "4px",
            padding: "16px",
            marginBottom: "16px",
            color: "#E8EDF5",
            fontFamily: "monospace",
          }}
        >
          <p style={{ margin: 0, marginBottom: 8, fontWeight: "bold" }}>Child Component 2</p>
          <p style={{ margin: 0, fontSize: 12, color: "#6B7FA3" }}>
            Shares SDK instance with sibling components
          </p>
        </div>
        <div
          style={{
            backgroundColor: "#1a1f2e",
            borderRadius: "4px",
            padding: "16px",
            color: "#E8EDF5",
            fontFamily: "monospace",
          }}
        >
          <p style={{ margin: 0, marginBottom: 8, fontWeight: "bold" }}>Child Component 3</p>
          <p style={{ margin: 0, fontSize: 12, color: "#6B7FA3" }}>
            All children use the same contract client and logger
          </p>
        </div>
      </div>
    ),
  },
};

export const LoadingState: Story = {
  args: {
    debug: true,
    children: (
      <div style={{ padding: "24px", color: "#E8EDF5", fontFamily: "monospace" }}>
        <h3 style={{ fontFamily: "var(--font-orbitron, sans-serif)", marginBottom: 8 }}>
          Loading State
        </h3>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "8px",
            marginTop: "16px",
          }}
        >
          <div
            style={{
              width: "16px",
              height: "16px",
              borderRadius: "50%",
              border: "2px solid #00D9FF",
              borderTopColor: "transparent",
              animation: "spin 1s linear infinite",
            }}
          />
          <span>Initializing provider...</span>
        </div>
        <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
      </div>
    ),
  },
};

export const ErrorState: Story = {
  args: {
    children: (
      <div style={{ padding: "24px" }}>
        <div
          style={{
            backgroundColor: "rgba(255, 59, 48, 0.1)",
            border: "1px solid rgba(255, 59, 48, 0.3)",
            borderRadius: "4px",
            padding: "16px",
            color: "#FF3B30",
            fontFamily: "monospace",
          }}
        >
          <p style={{ margin: 0, marginBottom: 8, fontWeight: "bold" }}>Error</p>
          <p style={{ margin: 0, fontSize: 12 }}>
            Failed to initialize StellarGrantsProvider: Contract client unavailable
          </p>
          <p style={{ margin: 12, fontSize: 12, color: "#6B7FA3" }}>
            Please ensure you have an active Stellar connection and try again.
          </p>
        </div>
      </div>
    ),
  },
};
