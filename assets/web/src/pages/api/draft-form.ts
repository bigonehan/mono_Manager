import type { APIRoute } from "astro";
import { loadDraftFormTemplate } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async ({ url }) => {
  try {
    const type = url.searchParams.get("type") || "code";
    const draft = loadDraftFormTemplate(type);
    return new Response(JSON.stringify({ draft }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
