"use client";

import React from "react";
import { Star } from "lucide-react";
import { Card } from "../ui/Card";
import { StatusDot } from "../ui/StatusDot";

const testimonials = [
  {
    address: "GBXYZ1S7QPKX3J2N5F6H4W8V9T0U1A4FT",
    quote: "StellarGrant replaced months of manual milestone tracking with trustless automation. Reviewers actually show up now.",
    rating: 5,
  },
  {
    address: "GCDEF2L4M5N6P7Q8R9S0T1U2V3W4B7KP",
    quote: "First time I've seen grant money released programmatically within seconds of approval. This is what Web3 should feel like.",
    rating: 5,
  },
  {
    address: "GHIJK3X2Y1Z0A9B8C7D6E5F4G3H2C2MR",
    quote: "The reputation system keeps contributors accountable. We've seen 94% milestone completion rates on funded grants.",
    rating: 5,
  },
];

const truncateAddress = (address: string) => {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

export const SocialProof = () => {
  return (
    <section className="py-24 px-6 md:px-20">
      <div className="max-w-6xl mx-auto">
        <div className="mb-16 text-center">
          <h2 className="text-3xl md:text-4xl font-bold font-orbitron mb-4 tracking-tight uppercase">
            BUILT FOR BUILDERS
          </h2>
          <div className="h-1 w-24 bg-accent-secondary mx-auto" />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {testimonials.map((t, index) => (
            <Card key={index} className="flex flex-col justify-between min-h-[250px]">
              <div>
                <div className="flex items-center justify-between mb-4">
                  <span className="font-mono text-xs text-accent-secondary">
                    {truncateAddress(t.address)}
                  </span>
                  <div className="flex gap-0.5">
                    {Array.from({ length: t.rating }).map((_, i) => (
                      <Star key={i} size={14} className="fill-accent-primary text-accent-primary" />
                    ))}
                  </div>
                </div>
                
                <p className="font-mono italic text-sm text-text-primary leading-relaxed mb-6">
                  &ldquo;{t.quote}&rdquo;
                </p>
              </div>

              <div className="pt-4 border-t border-border-color/30">
                <StatusDot status="verified" label="VERIFIED CONTRIBUTOR" />
              </div>
            </Card>
          ))}
        </div>
      </div>
    </section>
  );
};
