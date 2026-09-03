"use client";

import React from "react";
import { Rocket, Coins, ShieldCheck } from "lucide-react";
import { Card } from "../ui/Card";

const features = [
  {
    title: "Create Grants",
    description: "Define your project scope, set milestones, and establish a reviewer council. Your escrow is created on-chain the moment you submit.",
    icon: Rocket,
  },
  {
    title: "Fund Projects",
    description: "Contribute XLM directly to grant escrows. Track your funding history and watch milestones unlock in real time.",
    icon: Coins,
  },
  {
    title: "Verify Milestones",
    description: "Reviewers examine submitted proof-of-work URLs and vote on approval. Quorum reached = automatic on-chain payout.",
    icon: ShieldCheck,
  },
];

export const FeatureCards = () => {
  return (
    <section className="py-24 px-6 md:px-20">
      <div className="max-w-6xl mx-auto">
        <div className="mb-16 text-center md:text-left">
          <h2 className="text-3xl md:text-4xl font-bold font-orbitron mb-4 tracking-tight">
            PROTOCOL MODULES
          </h2>
          <p className="font-mono text-text-muted text-sm md:text-base max-w-xl">
            Three interlocking systems that power trustless grant distribution
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {features.map((feature, index) => (
            <Card key={index} className="relative overflow-hidden group">
              {/* Left accent stripe */}
              <div className="absolute left-0 top-0 bottom-0 w-1 bg-accent-primary transform scale-y-0 group-hover:scale-y-100 transition-transform duration-300" />
              
              <div className="mb-6 inline-flex p-3 border border-accent-secondary/30 bg-accent-secondary/5 rounded-none">
                <feature.icon className="w-8 h-8 text-accent-secondary" />
              </div>
              
              <h3 className="text-xl font-bold font-orbitron mb-4 text-text-primary group-hover:text-accent-primary transition-colors duration-300">
                {feature.title}
              </h3>
              
              <p className="font-mono text-sm text-text-muted leading-relaxed">
                {feature.description}
              </p>
            </Card>
          ))}
        </div>
      </div>
    </section>
  );
};
