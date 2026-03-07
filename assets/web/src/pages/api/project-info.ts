import type { APIRoute } from "astro";
import { saveProjectInfo } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const detail = saveProjectInfo(String(body.id), {
      name: String(body.name || "").trim(),
      description: String(body.description || "").trim(),
      spec: String(body.spec || "").trim(),
      goal: String(body.goal || "").trim()
    });
    return new Response(JSON.stringify({ detail }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
