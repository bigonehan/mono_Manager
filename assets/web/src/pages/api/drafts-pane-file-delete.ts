import type { APIRoute } from "astro";
import { deleteDraftPaneFile } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const id = String(body.id ?? "");
    const target = String(body.target ?? "");
    if (target !== "job" && target !== "drafts" && target !== "input") {
      throw new Error("target must be job or drafts");
    }
    const normalizedTarget = target === "input" ? "job" : target;
    const result = deleteDraftPaneFile(id, normalizedTarget as "job" | "drafts");
    return new Response(JSON.stringify(result), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
