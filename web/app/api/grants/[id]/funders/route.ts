import { NextResponse } from "next/server";
import { fetchGrantFunders } from "@/lib/grants/api";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const funders = await fetchGrantFunders(id);
  return NextResponse.json({
    funders: funders.map((f) => ({
      address: f.address,
      amount: f.amount.toString(),
    })),
  });
}
