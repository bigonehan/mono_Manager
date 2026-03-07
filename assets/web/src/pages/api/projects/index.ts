import type { APIRoute } from "astro";
import { createProject, listProjects, loadProjectDetail } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async () => {
  return new Response(JSON.stringify({ projects: listProjects() }), {
    headers: { "content-type": "application/json" }
  });
};

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const project = createProject({
      name: String(body.name || "").trim(),
      description: String(body.description || "").trim(),
      projectPath: String(body.path || "").trim(),
      spec: String(body.spec || "").trim(),
      projectType: String(body.project_type || "code").trim() as "story" | "movie" | "code" | "mono"
    });
    const detail = loadProjectDetail(project.id);
    return new Response(JSON.stringify({ project, detail }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
