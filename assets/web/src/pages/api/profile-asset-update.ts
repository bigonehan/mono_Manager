import type { APIRoute } from "astro";
import { updateProfileAssetFile } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const result = updateProfileAssetFile({
      type: body.type,
      section: body.section,
      name: body.name,
      content: body.content
    });
    return new Response(JSON.stringify(result), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
