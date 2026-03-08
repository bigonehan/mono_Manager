import type { APIRoute } from "astro";
import { saveProjectMemo } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const detail = saveProjectMemo(String(body.id), String(body.memo ?? ""));
    return new Response(JSON.stringify({ detail }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
