import type { APIRoute } from "astro";
import { deleteRequirementItem } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const id = String(body.id ?? "");
    const index = Number(body.index);
    const { detail, output } = deleteRequirementItem(id, index);
    return new Response(JSON.stringify({ detail, output }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
