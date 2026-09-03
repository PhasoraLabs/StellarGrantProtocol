"use client";

interface GlobalErrorProps {
  error: Error;
  reset: () => void;
}

export default function GlobalError({ error, reset }: GlobalErrorProps) {
  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          padding: 0,
          backgroundColor: "#050A14",
          color: "#E8EDF5",
          fontFamily: "monospace",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          minHeight: "100vh",
        }}
      >
        <div
          style={{
            border: "1px solid rgba(239, 68, 68, 0.4)",
            backgroundColor: "#0D1628",
            padding: "2.5rem",
            maxWidth: "480px",
            width: "100%",
            textAlign: "center",
          }}
        >
          <h1
            style={{
              fontSize: "1.25rem",
              fontWeight: "bold",
              letterSpacing: "0.1em",
              textTransform: "uppercase",
              marginBottom: "1rem",
              color: "#E8EDF5",
            }}
          >
            Critical Error
          </h1>
          <p
            style={{
              fontSize: "0.85rem",
              color: "#6B7280",
              marginBottom: "1.5rem",
              wordBreak: "break-word",
            }}
          >
            {error.message ?? "The application encountered an unrecoverable error."}
          </p>
          <button
            type="button"
            onClick={reset}
            style={{
              backgroundColor: "#F59E0B",
              color: "#050A14",
              border: "none",
              padding: "0.75rem 2rem",
              fontFamily: "monospace",
              fontSize: "0.85rem",
              fontWeight: "bold",
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              cursor: "pointer",
            }}
          >
            Reset
          </button>
        </div>
      </body>
    </html>
  );
}
