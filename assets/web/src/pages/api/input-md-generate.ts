import type { APIRoute } from "astro";
import { generateInputMdFromMessage } from "@/server/orc";

export const prerender = false;

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = await request.json();
    const id = String(body.id ?? "");
    const message = String(body.message ?? "");
    const { detail, output } = generateInputMdFromMessage(id, message);
    return new Response(JSON.stringify({ detail, output }), {
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), { status: 400 });
  }
};
