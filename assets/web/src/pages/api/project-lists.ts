import type { APIRoute } from "astro";
import { saveLists } from "@/server/orc";

export const prerender = false;

function normalizeList(input: unknown): string[] {
  if (!Array.isArray(input)) {
    return [];
  }
  return input
    .map((v) => String(v || "").trim())
    .filter((v) => v.length > 0);
}

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const detail = saveLists(String(body.id), {
      rules: normalizeList(body.rules),
      constraints: normalizeList(body.constraints),
      features: normalizeList(body.features)
    });
    return new Response(JSON.stringify({ detail }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
