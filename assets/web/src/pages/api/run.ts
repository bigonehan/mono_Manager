import type { APIRoute } from "astro";
import { runOrcAction } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const output = runOrcAction(String(body.id), String(body.action), body.payload);
    return new Response(JSON.stringify({ output }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
