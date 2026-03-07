import type { APIRoute } from "astro";
import { loadProjectDetail } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async ({ url }) => {
  try {
    const id = url.searchParams.get("id") || "";
    const detail = loadProjectDetail(id);
    return new Response(JSON.stringify({ detail }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
