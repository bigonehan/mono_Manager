import type { APIRoute } from "astro";
import { saveDomains } from "@/server/orc";

export const prerender = false;

function normalizeDomains(input: unknown): Array<{ name: string; description: string; features: string[] }> {
  if (!Array.isArray(input)) {
    return [];
  }
  return input
    .map((domain) => {
      const row = (domain ?? {}) as Record<string, unknown>;
      const features = Array.isArray(row.features)
        ? row.features.map((feature) => String(feature || "").trim()).filter((feature) => feature.length > 0)
        : [];
      return {
        name: String(row.name || "").trim(),
        description: String(row.description || "").trim(),
        features
      };
    })
    .filter((domain) => domain.name.length > 0);
}

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const detail = saveDomains(String(body.id), {
      domains: normalizeDomains(body.domains)
    });
    return new Response(JSON.stringify({ detail }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
