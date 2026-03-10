import type { APIRoute } from "astro";
import { appendCheckFeedback } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const result = appendCheckFeedback(String(body.id ?? ""), {
      screenshotPath: String(body.screenshotPath ?? ""),
      message: String(body.message ?? "")
    });
    return new Response(JSON.stringify(result), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
