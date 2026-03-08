import type { APIRoute } from "astro";
import { getBuildStatus } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async ({ url }) => {
  try {
    const id = String(url.searchParams.get("id") ?? "");
    const status = getBuildStatus(id);
    return new Response(JSON.stringify(status), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
