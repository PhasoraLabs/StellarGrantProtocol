"use client";

import React from "react";

interface StatusDotProps {
  status: "active" | "pending" | "verified" | "rejected" | "inactive" | "success" | "danger";
  label: string;
}

export const StatusDot = ({ status, label }: StatusDotProps) => {
  const colors = {
    active: "bg-success",
    pending: "bg-warning",
    verified: "bg-accent-secondary",
    rejected: "bg-danger",
    inactive: "bg-gray-500",
    success: "bg-success",
    danger: "bg-danger",
  };

  return (
    <div className="flex items-center gap-2">
      <div className={`w-1.5 h-1.5 rounded-full ${colors[status]}`} />
      <span className="font-mono text-[10px] text-text-primary uppercase tracking-wider">
        {label}
      </span>
    </div>
  );
};
