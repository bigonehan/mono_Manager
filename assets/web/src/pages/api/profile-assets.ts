import type { APIRoute } from "astro";
import { loadProfileAssets } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async ({ url }) => {
  try {
    const type = url.searchParams.get("type") || "code";
    const assets = loadProfileAssets(type);
    return new Response(JSON.stringify({ assets }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
