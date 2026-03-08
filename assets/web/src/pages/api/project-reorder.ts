import type { APIRoute } from "astro";
import { reorderProjects } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const ids = Array.isArray(body.ids) ? body.ids.map((v: unknown) => String(v)) : [];
    const projects = reorderProjects(ids);
    return new Response(JSON.stringify({ projects }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
