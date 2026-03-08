import type { APIRoute } from "astro";
import { getRuntimeLogs } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async ({ url }) => {
  try {
    const id = url.searchParams.get("id") || "";
    const logs = getRuntimeLogs(id);
    return new Response(JSON.stringify({ logs }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
