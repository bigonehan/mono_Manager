import type { APIRoute } from "astro";
import { refreshDomainFeatures } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const id = String(body.id ?? "").trim();
    const domain = String(body.domain ?? "").trim();
    if (!id) {
      return new Response(JSON.stringify({ error: "id is required" }), { status: 400 });
    }
    const result = refreshDomainFeatures(id, domain || undefined);
    return new Response(JSON.stringify(result), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
