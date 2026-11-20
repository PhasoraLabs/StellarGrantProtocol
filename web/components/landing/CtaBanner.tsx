"use client";

import React from "react";
import { Button } from "../ui/Button";

export const CtaBanner = () => {
  return (
    <section className="relative py-24 px-6 md:px-20 bg-bg-secondary border-t-2 border-accent-primary overflow-hidden">
      {/* Watermark Watermark */}
      <div className="absolute inset-0 flex items-center justify-center pointer-events-none z-0">
        <span className="font-orbitron text-[15vw] font-black text-white/[0.02] tracking-[0.2em] select-none">
          STELLAR
        </span>
      </div>

      <div className="relative z-10 max-w-4xl mx-auto text-center">
        <h2 className="text-4xl md:text-6xl font-bold font-orbitron mb-6 tracking-tight text-text-primary">
          READY TO LAUNCH?
        </h2>
        
        <p className="font-mono text-text-muted text-lg md:text-xl mb-12 max-w-2xl mx-auto">
          Join the protocol. Fund a grant, build something meaningful, or join as a reviewer.
        </p>

        <div className="flex flex-wrap justify-center gap-6">
          <Button variant="primary" href="/grants" className="w-full sm:w-auto">
            Get Started
          </Button>
          <Button variant="ghost" href="/docs" className="w-full sm:w-auto">
            Read the Docs
          </Button>
        </div>
      </div>
    </section>
  );
};
