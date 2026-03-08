import type { APIRoute } from "astro";
import { browseProjectDirs } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async ({ url }) => {
  try {
    const path = String(url.searchParams.get("path") ?? "");
    const data = browseProjectDirs(path);
    return new Response(JSON.stringify(data), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
