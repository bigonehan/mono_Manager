import type { APIRoute } from "astro";

export const prerender = false;

const features = [
  "Project CRUD (create/update/delete/select)",
  "Detail fields (name/description/spec/goal)",
  "Rules/Constraints/Features list editing",
  "Plan/Drafts panels (planned/generated)",
  "create_code_draft, add_code_draft, impl_code_draft",
  "check_code_draft -a, check_draft"
];

export const GET: APIRoute = async () => {
  return new Response(JSON.stringify({ features }), {
    headers: { "content-type": "application/json" }
  });
};
