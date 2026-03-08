import type { APIRoute } from "astro";
import { loadProjectDetail, updateProject } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const project = updateProject(String(body.id), { selected: true });
    const detail = loadProjectDetail(project.id);
    return new Response(JSON.stringify({ project, detail }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
