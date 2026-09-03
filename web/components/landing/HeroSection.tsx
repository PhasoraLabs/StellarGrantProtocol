"use client";

import React from "react";
import { motion, Variants } from "framer-motion";
import { Button } from "../ui/Button";

export const HeroSection = () => {
  const containerVariants: Variants = {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        staggerChildren: 0.15,
        delayChildren: 0.2,
      },
    },
  };

  const itemVariants: Variants = {
    hidden: { opacity: 0, y: 30 },
    visible: { opacity: 1, y: 0, transition: { duration: 0.6, ease: "easeOut" } },
  };

  return (
    <section className="relative min-height-[100vh] flex flex-col justify-center items-start px-6 md:px-20 py-24 overflow-hidden">
      <motion.div
        variants={containerVariants}
        initial="hidden"
        animate="visible"
        className="max-w-4xl z-10"
      >
        <motion.h1 
          variants={itemVariants}
          className="text-[clamp(2.5rem,8vw,5rem)] font-black leading-tight mb-6"
        >
          <span className="block text-text-primary">FUND WHAT MATTERS.</span>
          <span className="block text-accent-primary">PROVE WHAT WORKS.</span>
        </motion.h1>

        <motion.p 
          variants={itemVariants}
          className="font-mono text-text-muted text-lg md:text-xl max-w-2xl mb-10 leading-relaxed"
        >
          A decentralized, milestone-based grant protocol built on the Stellar blockchain. 
          Transparent funding. Verified outcomes.
        </motion.p>

        <motion.div variants={itemVariants} className="flex flex-wrap gap-4">
          <Button variant="primary" href="/grants">
            Browse Grants
          </Button>
          <Button variant="ghost" href="/grants/create">
            Create a Grant
          </Button>
        </motion.div>
      </motion.div>

      {/* Decorative Element: Hexagonal Grid / Circuit Pattern */}
      <div className="absolute bottom-0 right-0 w-full h-full pointer-events-none opacity-[0.06] z-0">
        <svg width="100%" height="100%" viewBox="0 0 800 800" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <pattern id="hexagons" width="50" height="43.4" patternUnits="userSpaceOnUse" patternTransform="scale(2) rotate(0)">
              <path d="M25 0 L50 14.4 L50 43.4 L25 57.8 L0 43.4 L0 14.4 Z" fill="none" stroke="var(--accent-secondary)" strokeWidth="1" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#hexagons)" />
          {/* Subtle circuit lines */}
          <path d="M600 200 L700 200 L750 250 M700 200 L700 300 L650 350" stroke="var(--accent-primary)" strokeWidth="1" fill="none" />
          <circle cx="750" cy="250" r="4" fill="var(--accent-primary)" />
          <circle cx="650" cy="350" r="4" fill="var(--accent-primary)" />
        </svg>
      </div>

      {/* Bottom Divider Line */}
      <div className="absolute bottom-0 left-0 w-full h-[1px] bg-border-color opacity-50" />
    </section>
  );
};
