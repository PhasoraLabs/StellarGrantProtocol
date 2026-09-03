import { NextResponse } from "next/server";
import { fetchGrantById, fetchGrantReviewers } from "@/lib/grants/api";
import { serializeGrant } from "@/lib/grants/serialize";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const detail = await fetchGrantById(id);
  if (!detail) {
    return NextResponse.json({ error: "Grant not found" }, { status: 404 });
  }
  const reviewers =
    detail.grant.reviewers.length > 0
      ? detail.grant.reviewers
      : await fetchGrantReviewers(id);

  return NextResponse.json({
    grant: serializeGrant({ ...detail.grant, reviewers }),
    milestones: detail.milestones,
    completedMilestones: detail.completedMilestones,
    isWatched: detail.isWatched,
    reviewers,
  });
}
