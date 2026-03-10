import type { APIRoute } from "astro";
import { readCheckScreenshot } from "@/server/orc";

export const prerender = false;

export const GET: APIRoute = async ({ url }) => {
  try {
    const id = String(url.searchParams.get("id") ?? "");
    const name = String(url.searchParams.get("name") ?? "");
    const screenshot = readCheckScreenshot(id, name);
    return new Response(new Uint8Array(screenshot.body), {
      headers: {
        "content-type": screenshot.contentType,
        "cache-control": "no-store"
      }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), {
      status: 400,
      headers: { "content-type": "application/json" }
    });
  }
};
