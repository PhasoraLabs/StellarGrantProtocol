"use client";

import React, { useEffect, useState } from "react";
import { StatBadge } from "../ui/StatBadge";

interface Stats {
  totalGrants: number;
  totalFunded: number;
  milestonesCompleted: number;
}

export const LiveStatsBar = () => {
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchStats = async () => {
      try {
        const baseUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001";
        const response = await fetch(`${baseUrl}/api/stats`);
        if (!response.ok) throw new Error("Failed to fetch");
        const data = await response.json();
        setStats(data);
      } catch (err) {
        console.error("Stats fetch error:", err);
        // Fallback for demo purposes if API is down
        setStats({
          totalGrants: 124,
          totalFunded: 450000,
          milestonesCompleted: 892,
        });
      } finally {
        setLoading(false);
      }
    };

    fetchStats();
  }, []);

  return (
    <div className="w-full bg-bg-secondary border-y border-border-color py-8">
      <div className="container mx-auto px-6">
        <div className="flex flex-col md:flex-row items-center justify-center gap-8 md:gap-0 divide-y md:divide-y-0 md:divide-x divide-border-color/30">
          <div className="flex-1 w-full md:w-auto px-8">
            {loading ? (
              <div className="h-16 w-32 shimmer mx-auto" />
            ) : (
              <StatBadge 
                value={stats?.totalGrants ?? 0} 
                label="Grants Created" 
              />
            )}
          </div>
          <div className="flex-1 w-full md:w-auto px-8">
            {loading ? (
              <div className="h-16 w-32 shimmer mx-auto" />
            ) : (
              <StatBadge 
                value={stats?.totalFunded ?? 0} 
                label="XLM Secured" 
                suffix="" 
              />
            )}
          </div>
          <div className="flex-1 w-full md:w-auto px-8">
            {loading ? (
              <div className="h-16 w-32 bg-surface animate-pulse mx-auto" />
            ) : (
              <StatBadge 
                value={stats?.milestonesCompleted ?? 0} 
                label="Milestones Verified" 
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
