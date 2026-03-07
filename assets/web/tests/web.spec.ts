import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

test("web ui: load and create/select project", async ({ page, request }) => {
  const unique = `pw-${Date.now()}`;
  const tmpPath = "/tmp/tmp_project";
  fs.mkdirSync(tmpPath, { recursive: true });
  fs.rmSync(path.join(tmpPath, ".project"), { recursive: true, force: true });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Code" })).toBeVisible();
  const createRes = await request.post("http://127.0.0.1:4173/api/projects", {
    data: {
      name: unique,
      description: "playwright e2e project",
      path: tmpPath,
      spec: "react, zustand",
      project_type: "code"
    }
  });
  expect(createRes.ok()).toBeTruthy();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Code" })).toBeVisible();

  const card = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
  await expect(card).toBeVisible();
  await expect(card).toContainText(/init|basic|work|wait/i);
  await card.click({ force: true });
  await expect(page.getByTestId("project-item-edit")).toBeVisible();
  await page.getByTestId("project-item-edit").click({ force: true });
  await page.getByTestId("edit-goal").fill("e2e-goal-updated");
  await page.getByTestId("edit-save").click({ force: true });
  const projectMd = path.join(tmpPath, ".project", "project.md");
  await expect
    .poll(() => fs.readFileSync(projectMd, "utf8"), {
      timeout: 10_000
    })
    .toContain("goal: e2e-goal-updated");

  const cfg = path.join(process.cwd(), "..", "..", "configs", "project.yaml");
  const raw = fs.readFileSync(cfg, "utf8");
  expect(raw).toContain(unique);
  expect(raw).toContain("project_type: code");
});
