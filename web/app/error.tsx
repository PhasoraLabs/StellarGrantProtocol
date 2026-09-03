"use client";

import { useEffect } from "react";
import { motion } from "framer-motion";
import { AlertTriangle } from "lucide-react";
import Link from "next/link";
import { StarField } from "@/components/landing/StarField";

interface ErrorPageProps {
  error: Error & { digest?: string };
  reset: () => void;
}

export default function ErrorPage({ error, reset }: ErrorPageProps) {
  useEffect(() => {
    console.error("[ErrorBoundary]", error);
  }, [error]);

  const message = error.message?.slice(0, 200) ?? "An unexpected error occurred.";

  return (
    <div className="relative min-h-screen bg-bg-primary flex items-center justify-center overflow-hidden">
      <StarField />
      <motion.div
        className="relative z-10 flex flex-col items-center text-center px-6 max-w-lg w-full"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: "easeOut" }}
      >
        <div className="border border-danger/40 bg-surface p-8 w-full">
          <AlertTriangle className="mx-auto mb-4 text-danger" size={40} />

          <h1 className="font-orbitron text-xl font-bold text-text-primary mb-3">
            Something went wrong
          </h1>

          <p className="font-mono text-sm text-text-muted break-words mb-4">
            {message}
          </p>

          {error.digest && (
            <pre className="mb-6 bg-bg-primary border border-border-color px-3 py-2 font-mono text-xs text-text-muted text-left overflow-x-auto">
              digest: {error.digest}
            </pre>
          )}

          <div className="flex flex-wrap items-center justify-center gap-3">
            <button
              type="button"
              onClick={reset}
              className="font-orbitron text-sm font-bold uppercase tracking-wider bg-accent-primary text-bg-primary px-6 py-3 hover:opacity-90 transition-opacity"
            >
              Try Again
            </button>
            <Link
              href="/grants"
              className="font-orbitron text-sm font-bold uppercase tracking-wider border border-accent-primary text-accent-primary px-6 py-3 hover:bg-accent-primary hover:text-bg-primary transition-colors"
            >
              Go to Grants
            </Link>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
