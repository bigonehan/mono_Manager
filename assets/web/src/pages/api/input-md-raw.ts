import type { APIRoute } from "astro";
import { applyRawInputMd, saveRawInputMd } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const id = String(body.id ?? "");
    const raw = String(body.raw ?? "");
    const apply = Boolean(body.apply);
    if (apply) {
      const { detail, stages } = await applyRawInputMd(id, raw);
      return new Response(JSON.stringify({ detail, stages }), {
        headers: { "content-type": "application/json" }
      });
    }
    const detail = saveRawInputMd(id, raw);
    return new Response(JSON.stringify({ detail, stages: [] }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
