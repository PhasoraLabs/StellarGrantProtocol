"use client";

import Link from "next/link";
import { useQueries } from "@tanstack/react-query";
import { GrantCard, grantCardVariants, grantListVariants } from "@/components/grants/GrantCard";
import { useWatchlist } from "@/hooks/useWatchlist";
import { motion, useReducedMotion } from "framer-motion";
import { mapStatusToNumber } from "@/lib/grants/api";

type GrantApiResponse = {
  grant: {
    id: string | number;
    title: string;
    status: string | number;
    budget: string | bigint;
    funded: string | bigint;
    deadline: string | bigint;
    token?: string;
    owner?: string;
    recipient?: string;
  };
};

function mapToCardGrant(data: GrantApiResponse["grant"]) {
  return {
    id: Number(data.id),
    title: data.title,
    status: mapStatusToNumber(data.status),
    funded: typeof data.funded === "bigint" ? data.funded : BigInt(data.funded ?? "0"),
    budget: typeof data.budget === "bigint" ? data.budget : BigInt(data.budget ?? "0"),
    deadline: typeof data.deadline === "bigint" ? data.deadline : BigInt(data.deadline ?? "0"),
    token: data.token,
    owner: data.owner ?? data.recipient,
  };
}

export function WatchedGrantsPanel() {
  const { watchedIds, remove } = useWatchlist();
  const prefersReduced = useReducedMotion();

  const queries = useQueries({
    queries: watchedIds.map((id) => ({
      queryKey: ["grant", id],
      queryFn: async (): Promise<GrantApiResponse> => {
        const res = await fetch(`/api/grants/${id}`);
        if (!res.ok) throw new Error(`Failed to load grant ${id}`);
        return res.json() as Promise<GrantApiResponse>;
      },
      staleTime: 60_000,
    })),
  });

  const isLoading = queries.some((query) => query.isLoading);
  const grants = queries
    .map((query, index) => ({
      id: watchedIds[index]!,
      data: query.data,
      error: query.error,
    }))
    .filter((entry) => entry.data?.grant);

  if (watchedIds.length === 0) {
    return (
      <div className="rounded-[4px] border border-border-color bg-surface/60 p-10 text-center space-y-3">
        <p className="text-text-muted text-sm">
          You&apos;re not watching any grants. Browse grants and click ☆ Watch to add them here.
        </p>
        <Link href="/grants" className="inline-block text-sm text-accent-primary hover:underline">
          Browse grants →
        </Link>
      </div>
    );
  }

  if (isLoading && grants.length === 0) {
    return (
      <div className="grid gap-4 md:grid-cols-2">
        {watchedIds.map((id) => (
          <div key={id} className="shimmer h-52 rounded-[4px]" />
        ))}
      </div>
    );
  }

  return (
    <motion.div
      className="grid gap-4 md:grid-cols-2"
      variants={grantListVariants}
      initial={prefersReduced ? {} : "hidden"}
      animate="visible"
    >
      {grants.map(({ id, data }) => (
        <motion.div key={id} variants={prefersReduced ? {} : grantCardVariants}>
          <Link href={`/grants/${id}`}>
            <GrantCard
              grant={mapToCardGrant(data!.grant)}
              showWatchlistBadge
              watchlistGrantId={id}
              onRemoveFromWatchlist={() => remove(id)}
            />
          </Link>
        </motion.div>
      ))}
    </motion.div>
  );
}
