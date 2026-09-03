import { NextResponse } from "next/server";
import { API_URL } from "@/lib/constants";
import { matchesQuery } from "@/lib/search/map";

type SearchHit = {
  id: string;
  name: string;
  type: string;
};

type ApiMilestone = {
  idx: number;
  title: string;
  description?: string | null;
  submitted?: boolean;
  approved?: boolean;
  paid?: boolean;
};

type ApiGrantDetail = {
  id: number;
  title: string;
  milestones?: ApiMilestone[];
};

export type MilestoneSearchResult = {
  id: string;
  grantId: string;
  milestoneIdx: number;
  title: string;
  grantTitle: string;
  status: string;
};

function milestoneStatus(m: ApiMilestone): string {
  if (m.paid) return "Paid";
  if (m.approved) return "Approved";
  if (m.submitted) return "Submitted";
  return "Pending";
}

async function fetchGrantMilestones(grantId: string, q: string): Promise<MilestoneSearchResult[]> {
  const res = await fetch(`${API_URL}/grants/${grantId}`, { next: { revalidate: 30 } });
  if (!res.ok) return [];

  const json = (await res.json()) as { data?: ApiGrantDetail };
  const grant = json.data;
  if (!grant?.milestones?.length) return [];

  return grant.milestones
    .filter((m) =>
      matchesQuery(`${m.title} ${m.description ?? ""} ${grant.title}`, q),
    )
    .map((m) => ({
      id: `${grantId}-${m.idx}`,
      grantId,
      milestoneIdx: m.idx,
      title: m.title,
      grantTitle: grant.title,
      status: milestoneStatus(m),
    }));
}

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const q = searchParams.get("q")?.trim() ?? "";

  if (!q || q.length < 2) {
    return NextResponse.json({ milestones: [] });
  }

  try {
    const res = await fetch(`${API_URL}/search?q=${encodeURIComponent(q)}`, {
      next: { revalidate: 30 },
    });
    if (!res.ok) {
      return NextResponse.json({ milestones: [] }, { status: res.status });
    }

    const json = (await res.json()) as { data?: SearchHit[] };
    const hits = json.data ?? [];
    const results: MilestoneSearchResult[] = [];
    const seen = new Set<string>();

    for (const hit of hits.filter((row) => row.type === "milestone")) {
      const key = `proof-${hit.id}`;
      if (seen.has(key)) continue;
      seen.add(key);
      results.push({
        id: hit.id,
        grantId: "",
        milestoneIdx: 0,
        title: hit.name,
        grantTitle: "",
        status: "Submitted",
      });
    }

    const grantIds = new Set(
      hits.filter((row) => row.type === "grant").map((row) => String(row.id)),
    );

    if (grantIds.size === 0) {
      const grantsRes = await fetch(`${API_URL}/grants?limit=50&order=DESC&sortBy=updatedAt`, {
        next: { revalidate: 30 },
      });
      if (grantsRes.ok) {
        const grantsJson = (await grantsRes.json()) as { data?: Array<{ id: number; title: string }> };
        for (const grant of grantsJson.data ?? []) {
          if (matchesQuery(grant.title, q)) grantIds.add(String(grant.id));
        }
      }
    }

    for (const grantId of grantIds) {
      const fromGrant = await fetchGrantMilestones(grantId, q);
      for (const item of fromGrant) {
        if (seen.has(item.id)) continue;
        seen.add(item.id);
        results.push(item);
      }
    }

    return NextResponse.json({ milestones: results });
  } catch {
    return NextResponse.json({ milestones: [] }, { status: 500 });
  }
}
