import type { APIRoute } from "astro";
import { applyFormAddInput, applyRawRequirementInput } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const id = String(body.id ?? "");
    const raw = String(body.raw ?? "").trim();
    if (raw.length > 0) {
      const { detail, stages, parsedCount } = await applyRawRequirementInput(id, raw);
      return new Response(JSON.stringify({ detail, stages, parsedCount }), {
        headers: { "content-type": "application/json" }
      });
    }
    const items = Array.isArray(body.items)
      ? body.items.map((row: Record<string, unknown>) => ({
          title: String(row.title ?? ""),
          rule: String(row.rule ?? ""),
          step: String(row.step ?? "")
        }))
      : [];
    const { detail, stages } = await applyFormAddInput(id, items);
    return new Response(JSON.stringify({ detail, stages }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
