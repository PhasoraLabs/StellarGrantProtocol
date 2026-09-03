"use client";

import { motion } from "framer-motion";
import { StarField } from "@/components/landing/StarField";
import { Button } from "@/components/ui/Button";

export default function NotFound() {
  return (
    <div className="relative min-h-screen bg-bg-primary flex items-center justify-center overflow-hidden">
      <StarField />
      <motion.div
        className="relative z-10 flex flex-col items-center text-center px-6"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6, ease: "easeOut" }}
      >
        <h1
          className="font-orbitron font-black text-accent-primary leading-none"
          style={{ fontSize: "clamp(5rem, 10vw, 12rem)" }}
        >
          404
        </h1>

        <p className="mt-4 font-mono text-sm uppercase tracking-[0.3em] text-text-muted">
          Signal Lost
        </p>

        <p className="mt-6 max-w-md font-mono text-sm text-text-secondary leading-relaxed">
          The grant or page you&apos;re looking for doesn&apos;t exist or has been removed.
        </p>

        <div className="mt-10 flex flex-wrap items-center justify-center gap-4">
          <Button href="/grants" variant="ghost">
            ← Back to Grants
          </Button>
          <Button href="/" variant="primary">
            Home
          </Button>
        </div>
      </motion.div>
    </div>
  );
}
