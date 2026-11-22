"use client";

import React, { useState, useRef, useEffect } from "react";

export interface TooltipProps {
  content: string;
  children: React.ReactNode;
  position?: "top" | "bottom" | "left" | "right";
  delay?: number;
}

export function Tooltip({
  content,
  children,
  position = "top",
  delay = 200,
}: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const triggerRef = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<NodeJS.Timeout | undefined>(undefined);

  const handleMouseEnter = () => {
    timeoutRef.current = setTimeout(() => {
      setIsVisible(true);
    }, delay);
  };

  const handleMouseLeave = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    setIsVisible(false);
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  const positionClasses: Record<string, string> = {
    top: "bottom-full left-1/2 -translate-x-1/2 mb-2",
    bottom: "top-full left-1/2 -translate-x-1/2 mt-2",
    left: "right-full top-1/2 -translate-y-1/2 mr-2",
    right: "left-full top-1/2 -translate-y-1/2 ml-2",
  };

  const arrowClasses: Record<string, string> = {
    top: "top-full left-1/2 -translate-x-1/2 border-t-bg-surface border-t-8 border-l-transparent border-l-8 border-r-transparent border-r-8",
    bottom: "bottom-full left-1/2 -translate-x-1/2 border-b-bg-surface border-b-8 border-l-transparent border-l-8 border-r-transparent border-r-8",
    left: "left-full top-1/2 -translate-y-1/2 border-l-bg-surface border-l-8 border-t-transparent border-t-8 border-b-transparent border-b-8",
    right: "right-full top-1/2 -translate-y-1/2 border-r-bg-surface border-r-8 border-t-transparent border-t-8 border-b-transparent border-b-8",
  };

  return (
    <div
      ref={triggerRef}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      className="relative inline-block"
    >
      {children}
      {isVisible && (
        <div
          role="tooltip"
          className={`absolute z-50 w-max max-w-xs whitespace-normal bg-bg-surface border border-border-color rounded px-3 py-2 font-mono text-xs text-text-muted ${positionClasses[position]} shadow-lg`}
        >
          {content.split("\n").map((line, i) => (
            <div key={i}>{line}</div>
          ))}
          <div
            aria-hidden="true"
            className={`absolute h-0 w-0 ${arrowClasses[position]}`}
          />
        </div>
      )}
    </div>
  );
}
