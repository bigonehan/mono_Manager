import type { APIRoute } from "astro";

export const prerender = false;

const features = [
  "Project CRUD (create/update/delete/select)",
  "Detail fields (name/description/spec/goal)",
  "Rules/Constraints/Features list editing",
  "Plan/Drafts panels (planned/generated)",
  "add_orc_drafts, impl_orc_code",
  "check_orc_code"
];

export const GET: APIRoute = async () => {
  return new Response(JSON.stringify({ features }), {
    headers: { "content-type": "application/json" }
  });
};
