"use client";

import React, { useEffect, useState } from "react";
import { animate } from "framer-motion";

interface StatBadgeProps {
  value: number;
  label: string;
  prefix?: string;
  suffix?: string;
}

export const StatBadge = ({ value, label, prefix = "", suffix = "" }: StatBadgeProps) => {
  const [displayValue, setDisplayValue] = useState(0);

  useEffect(() => {
    const controls = animate(0, value, {
      duration: 1.5,
      ease: "easeOut",
      onUpdate: (latest) => setDisplayValue(Math.floor(latest)),
    });
    return () => controls.stop();
  }, [value]);

  return (
    <div className="flex flex-col items-center justify-center p-4">
      <div className="font-orbitron text-3xl md:text-4xl font-bold text-accent-primary mb-1">
        {prefix}{displayValue.toLocaleString()}{suffix}
      </div>
      <div className="font-mono text-[10px] md:text-xs text-text-muted uppercase tracking-[0.2em]">
        {label}
      </div>
    </div>
  );
};
