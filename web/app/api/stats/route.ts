import { proxyGet } from "@/lib/apiProxy";

export async function GET(request: Request) {
  return proxyGet(request, "/stats", { revalidate: 60 });
}
