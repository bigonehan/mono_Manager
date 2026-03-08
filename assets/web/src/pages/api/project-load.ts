import type { APIRoute } from "astro";
import { loadProjectDetail, loadProjectFromPath } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const loaded = loadProjectFromPath({
      projectPath: String(body.path || ""),
      createIfMissing: Boolean(body.create_if_missing),
      projectType: body.project_type
    });
    const detail = loadProjectDetail(loaded.project.id);
    return new Response(
      JSON.stringify({
        project: loaded.project,
        detail,
        created_project_meta: loaded.createdProjectMeta
      }),
      {
        headers: { "content-type": "application/json" }
      }
    );
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
