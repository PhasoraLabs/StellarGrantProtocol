import { NextResponse } from "next/server";
import { API_URL } from "@/lib/constants";

type SearchHit = {
  id: string;
  name: string;
  type: string;
};

type LeaderboardEntry = {
  address: string;
  reputation: number;
  totalGrantsCompleted?: number;
};

export type ContributorSearchResult = {
  address: string;
  reputation_score: number;
  grants_completed: number;
};

async function fetchLeaderboardMap(): Promise<Map<string, LeaderboardEntry>> {
  const res = await fetch(`${API_URL}/leaderboard?limit=100`, {
    next: { revalidate: 60 },
  });
  if (!res.ok) return new Map();

  const json = (await res.json()) as { data?: LeaderboardEntry[] };
  const map = new Map<string, LeaderboardEntry>();
  for (const entry of json.data ?? []) {
    map.set(entry.address, entry);
  }
  return map;
}

function enrichContributor(
  address: string,
  leaderboard: Map<string, LeaderboardEntry>,
): ContributorSearchResult {
  const stats = leaderboard.get(address);
  return {
    address,
    reputation_score: stats?.reputation ?? 0,
    grants_completed: stats?.totalGrantsCompleted ?? 0,
  };
}

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const q = searchParams.get("q")?.trim() ?? "";
  const leaderboard = await fetchLeaderboardMap();

  if (!q) {
    const top = [...leaderboard.values()].slice(0, 6).map((entry) =>
      enrichContributor(entry.address, leaderboard),
    );
    return NextResponse.json({ contributors: top });
  }

  if (q.length < 2) {
    return NextResponse.json({ contributors: [] });
  }

  try {
    const res = await fetch(`${API_URL}/search?q=${encodeURIComponent(q)}`, {
      next: { revalidate: 30 },
    });
    if (!res.ok) {
      return NextResponse.json({ contributors: [] }, { status: res.status });
    }

    const json = (await res.json()) as { data?: SearchHit[] };
    const hits = (json.data ?? []).filter((row) => row.type === "contributor");

    const prefixMatches = [...leaderboard.values()].filter((entry) =>
      entry.address.toUpperCase().startsWith(q.toUpperCase()),
    );

    const addresses = new Set<string>();
    const contributors: ContributorSearchResult[] = [];

    for (const hit of hits) {
      if (addresses.has(hit.id)) continue;
      addresses.add(hit.id);
      contributors.push(enrichContributor(hit.id, leaderboard));
    }

    for (const entry of prefixMatches) {
      if (addresses.has(entry.address)) continue;
      addresses.add(entry.address);
      contributors.push(enrichContributor(entry.address, leaderboard));
    }

    return NextResponse.json({ contributors });
  } catch {
    return NextResponse.json({ contributors: [] }, { status: 500 });
  }
}
