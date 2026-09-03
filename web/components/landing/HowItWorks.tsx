"use client";

import React from "react";
import { motion } from "framer-motion";

const steps = [
  {
    title: "Define",
    description: "Grant owner sets title, budget, milestones, and reviewer council on-chain.",
  },
  {
    title: "Fund",
    description: "Community contributors deposit XLM into the escrow smart contract.",
  },
  {
    title: "Build",
    description: "Recipient executes the work and submits a proof-of-work URL per milestone.",
  },
  {
    title: "Review",
    description: "Reviewers examine proof and vote. Quorum triggers automatic payout.",
  },
  {
    title: "Resolve",
    description: "Disputes escalate to the global Council for final arbitration.",
  },
];

export const HowItWorks = () => {
  return (
    <section className="py-24 px-6 md:px-20 bg-bg-secondary/30">
      <div className="max-w-6xl mx-auto">
        <div className="mb-20 text-center">
          <h2 className="text-3xl md:text-4xl font-bold font-orbitron mb-4 tracking-wider">
            HOW IT WORKS
          </h2>
          <div className="h-1 w-20 bg-accent-primary mx-auto" />
        </div>

        <div className="relative">
          {/* Connecting Line - Desktop */}
          <div className="hidden md:block absolute top-12 left-[10%] right-[10%] h-[1px] border-t border-dashed border-border-color z-0" />
          
          <div className="grid grid-cols-1 md:grid-cols-5 gap-12 relative z-10">
            {steps.map((step, index) => (
              <motion.div
                key={index}
                initial={{ opacity: 0, y: 30 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.1, duration: 0.5 }}
                className="flex flex-col items-center text-center group"
              >
                <div className="w-24 h-24 rounded-full bg-accent-primary/5 border border-accent-primary/20 flex items-center justify-center mb-6 relative transition-all duration-300 group-hover:border-accent-primary group-hover:bg-accent-primary/10">
                  <span className="font-orbitron text-4xl font-black text-accent-primary/20 group-hover:text-accent-primary/40 transition-colors duration-300">
                    {index + 1}
                  </span>
                  <div className="absolute inset-0 rounded-full border border-accent-primary opacity-0 group-hover:opacity-100 group-hover:scale-110 transition-all duration-500" />
                </div>
                
                <h3 className="font-orbitron text-xl font-bold mb-3 text-text-primary uppercase tracking-wide">
                  {step.title}
                </h3>
                
                <p className="font-mono text-sm text-text-muted leading-relaxed max-w-[200px]">
                  {step.description}
                </p>

                {/* Connecting Line - Mobile */}
                {index < steps.length - 1 && (
                  <div className="md:hidden w-[1px] h-12 border-l border-dashed border-border-color my-4" />
                )}
              </motion.div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
};
