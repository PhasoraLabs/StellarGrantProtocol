"use client";

/**
 * PageTransition
 *
 * Wraps page content in an AnimatePresence fade-slide animation.
 * Respects useReducedMotion() — when the user prefers reduced motion
 * the animation is an instant opacity fade (no translate).
 *
 * Usage:
 *   <PageTransition key={pathname}>
 *     {children}
 *   </PageTransition>
 */

import { motion, AnimatePresence, useReducedMotion } from "framer-motion";

interface PageTransitionProps {
  children: React.ReactNode;
  /** Unique key that triggers re-animation on route change */
  motionKey?: string;
}

const FULL_VARIANTS = {
  initial: { opacity: 0, y: 12 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -8 },
} as const;

const REDUCED_VARIANTS = {
  initial: { opacity: 0 },
  animate: { opacity: 1 },
  exit: { opacity: 0 },
} as const;

export function PageTransition({ children, motionKey }: PageTransitionProps) {
  const prefersReduced = useReducedMotion();
  const variants = prefersReduced ? REDUCED_VARIANTS : FULL_VARIANTS;

  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={motionKey}
        variants={variants}
        initial="initial"
        animate="animate"
        exit="exit"
        transition={{ duration: prefersReduced ? 0.15 : 0.25, ease: "easeOut" as const }}
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}
