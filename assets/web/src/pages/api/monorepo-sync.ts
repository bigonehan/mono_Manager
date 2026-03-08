import { syncMonorepoProjects } from "@/server/orc";

export async function POST(): Promise<Response> {
  try {
    const data = syncMonorepoProjects();
    return new Response(JSON.stringify(data), {
      status: 200,
      headers: { "content-type": "application/json" }
    });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error) }), {
      status: 500,
      headers: { "content-type": "application/json" }
    });
  }
}
