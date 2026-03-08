import type { APIRoute } from "astro";
import { stopParallelBuild } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const result = stopParallelBuild(String(body.id ?? ""));
    return new Response(JSON.stringify(result), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
