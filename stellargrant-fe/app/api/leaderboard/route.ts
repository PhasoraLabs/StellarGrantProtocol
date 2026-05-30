import { proxyGet } from "@/lib/apiProxy";

export async function GET(request: Request) {
  return proxyGet(request, "/leaderboard", { revalidate: 300 });
}
