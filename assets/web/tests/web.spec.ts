import { expect, test, type APIRequestContext } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { installMockSpeechRecognition } from "./helpers/mock-speech-recognition";

type CreateProjectPayload = {
  name: string;
  description: string;
  path: string;
  spec: string;
  project_type: "code" | "mono";
};

const trackedProjectIds = new Set<string>();
const trackedPaths = new Set<string>();

function trackPathForCleanup(targetPath: string): void {
  trackedPaths.add(targetPath);
}

async function createProjectForTest(request: APIRequestContext, payload: CreateProjectPayload) {
  trackPathForCleanup(payload.path);
  const response = await request.post("http://127.0.0.1:4175/api/projects", {
    data: payload
  });
  expect(response.ok()).toBeTruthy();
  const body = (await response.json().catch(() => ({}))) as { project?: { id?: string } };
  const projectId = body.project?.id;
  if (typeof projectId === "string" && projectId.length > 0) {
    trackedProjectIds.add(projectId);
  }
  return response;
}

test.afterEach(async ({ request }) => {
  for (const projectId of trackedProjectIds) {
    try {
      await request.post("http://127.0.0.1:4175/api/project-delete", {
        data: { id: projectId }
      });
    } catch {
      // ignore cleanup errors in teardown
    }
  }
  trackedProjectIds.clear();

  const cleanupTargets = Array.from(trackedPaths).sort((a, b) => b.length - a.length);
  for (const targetPath of cleanupTargets) {
    fs.rmSync(targetPath, { recursive: true, force: true });
  }
  trackedPaths.clear();
});

test("web ui: load and create/select project", async ({ page, request }) => {
  const unique = `pw-${Date.now()}`;
  const tmpPath = "/tmp/tmp_project";
  trackPathForCleanup(tmpPath);
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Code" })).toBeVisible();
  await createProjectForTest(request, {
    name: unique,
    description: "playwright e2e project",
    path: tmpPath,
    spec: "react, zustand",
    project_type: "code"
  });
  await page.reload();
  await expect(page.getByRole("heading", { name: "Code" })).toBeVisible();

  const card = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
  await expect(card).toBeVisible();
  await expect(card).toContainText(/wait|work|complete|auto/i);
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

test("web ui: drafts pane uses green chevron trigger and icon-only delete layout", async ({ page, request }) => {
  const unique = `pw-drafts-${Date.now()}`;
  const tmpPath = `/tmp/${unique}`;
  trackPathForCleanup(tmpPath);
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });

  await createProjectForTest(request, {
    name: unique,
    description: "drafts pane verification",
    path: tmpPath,
    spec: "react, draft",
    project_type: "code"
  });

  await page.goto("/");
  const card = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
  await expect(card).toBeVisible();
  await card.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  await page.route("**/api/run", async (route) => {
    const body = route.request().postDataJSON() as { action?: string } | null;
    if (body?.action === "impl_draft") {
      await new Promise((resolve) => setTimeout(resolve, 600));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ output: "impl queued" })
      });
      return;
    }
    await route.continue();
  });

  const draftPane = page.getByTestId("draft-pane");
  await expect(draftPane).toBeVisible();
  const requirementsContainer = page.getByTestId("requirements-container");
  await expect(requirementsContainer).toBeVisible();
  await expect(page.getByTestId("requirements-scroll")).toBeVisible();
  await expect(page.getByRole("button", { name: "open-requirement-modal" })).toBeVisible();
  await expect(page.getByRole("button", { name: "open-message-job-modal" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "save-drafts-yaml" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "delete-job-md" })).toBeVisible();
  await expect(page.getByRole("button", { name: "delete-drafts-yaml" })).toBeVisible();
  await expect(page.getByTestId("generate-job-and-drafts")).toBeVisible();
  await expect(page.getByTestId("open-draft-pane-settings")).toBeVisible();
  const topRightActions = page.getByTestId("requirements-top-right-actions");
  await expect(topRightActions).toBeVisible();
  const [reqBox, topBox, plusBox, generateBox] = await Promise.all([
    requirementsContainer.boundingBox(),
    topRightActions.boundingBox(),
    page.getByTestId("open-requirement-modal").boundingBox(),
    page.getByTestId("generate-job-and-drafts").boundingBox()
  ]);
  expect(reqBox).not.toBeNull();
  expect(topBox).not.toBeNull();
  expect(plusBox).not.toBeNull();
  expect(generateBox).not.toBeNull();
  if (reqBox && topBox && plusBox && generateBox) {
    expect(topBox.y).toBeLessThan(reqBox.y + reqBox.height * 0.35);
    expect(topBox.x + topBox.width).toBeGreaterThan(reqBox.x + reqBox.width * 0.8);
    expect(plusBox.y).toBeGreaterThan(reqBox.y + reqBox.height);
    expect(generateBox.y).toBeGreaterThan(reqBox.y + reqBox.height);
  }
  await page.getByRole("button", { name: "open-requirement-modal" }).click();
  await expect(page.getByRole("heading", { name: "요구사항 추가" })).toBeVisible();
  await page
    .getByPlaceholder("## 기능 이름\n- 기능(옵션)\n> 순서(옵션)\n\n## 다른 기능\n- 규칙")
    .fill(["## login", "- 접근성 유지", "> 로그인 폼 노출", "", "## profile", "- 읽기 화면 제공", "> 프로필 편집 이동"].join("\n"));
  await page.getByRole("button", { name: "저장" }).click();
  await expect(page.getByRole("heading", { name: "요구사항 추가" })).toHaveCount(0);
  const jobPath = path.join(tmpPath, "job.md");
  await expect
    .poll(() => fs.existsSync(jobPath), {
      timeout: 10_000
    })
    .toBeTruthy();
  const jobRawBeforeSync = fs.readFileSync(jobPath, "utf8");
  await page.getByTestId("generate-job-and-drafts").click();
  await expect
    .poll(() => fs.readFileSync(jobPath, "utf8"), { timeout: 10_000 })
    .toBe(jobRawBeforeSync);

  await expect(page.getByText(/no requirement blocks/i)).toHaveCount(0);
  const firstRequirementCard = page.locator('[data-testid="requirements-scroll"] > div').first();
  await firstRequirementCard.hover();
  await expect(page.getByTestId("delete-requirement-item-0")).toBeVisible();
  await expect(page.getByTestId("draft-work-pane")).toBeVisible();
  const paneIsOutsideDraftCard = await page.evaluate(() => {
    const draftCard = document.querySelector('[data-testid="draft-pane"]');
    const workPane = document.querySelector('[data-testid="draft-work-pane"]');
    if (!draftCard || !workPane) return false;
    return !draftCard.contains(workPane);
  });
  expect(paneIsOutsideDraftCard).toBeTruthy();
  await expect(page.getByTestId("draft-work-list")).toBeVisible();
  await expect(page.getByTestId("draft-work-detail")).toBeVisible();
  await expect(page.getByTestId("draft-item-card-login")).toBeVisible();
  await page.getByTestId("draft-item-card-login").click();
  await expect(page.getByTestId("draft-work-detail")).toContainText("login");

  const leftHeight = await page.getByTestId("draft-work-list").evaluate((node) => Math.round(node.getBoundingClientRect().height));
  const rightHeight = await page.getByTestId("draft-work-detail").evaluate((node) => Math.round(node.getBoundingClientRect().height));
  expect(Math.abs(leftHeight - rightHeight)).toBeLessThanOrEqual(2);

  const draftsPath = path.join(tmpPath, ".project", "drafts.yaml");
  await expect
    .poll(() => fs.existsSync(jobPath), {
      timeout: 10_000
    })
    .toBeTruthy();
  await expect
    .poll(() => fs.existsSync(draftsPath), {
      timeout: 10_000
    })
    .toBeTruthy();

  const jobRaw = fs.readFileSync(jobPath, "utf8");
  const draftsRaw = fs.readFileSync(draftsPath, "utf8");
  expect(jobRaw).toContain("## login");
  expect(jobRaw).toContain("## profile");
  expect(jobRaw).toContain("## planned");
  expect(jobRaw).toContain("- login");
  expect(jobRaw).toContain("- profile");
  expect(draftsRaw).toContain("name: login");
  expect(draftsRaw).toContain("name: profile");

  const loginDot = page.getByTestId("draft-item-status-dot-login");
  await expect(loginDot).toHaveClass(/bg-red-500/);

  await page.getByTestId("draft-work-impl").click();
  await expect(page.getByTestId("draft-work-running-overlay")).toBeVisible();
  await expect(loginDot).toHaveClass(/bg-amber-500/);
  await expect(page.getByTestId("draft-work-running-overlay")).toBeHidden({ timeout: 3000 });
});

test("web ui: check pane renders draft subject and appends screenshot feedback", async ({ page, request }) => {
  const unique = `pw-check-${Date.now()}`;
  const tmpPath = `/tmp/${unique}`;
  trackPathForCleanup(tmpPath);
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });

  await createProjectForTest(request, {
    name: unique,
    description: "check pane verification",
    path: tmpPath,
    spec: "react, check",
    project_type: "code"
  });

  fs.writeFileSync(
    path.join(tmpPath, ".project", "drafts.yaml"),
    [
      "draft:",
      "  - name: manual_review",
      "    check:",
      "      - 메인 흐름이 정상 동작해야 한다",
      "      - 상세 pane에서 report modal이 열려야 한다",
      "planned:",
      "  - manual_review",
      "worked: []",
      "complete: []",
      "failed: []"
    ].join("\n"),
    "utf8"
  );
  fs.writeFileSync(
    path.join(tmpPath, ".project", "feedback.md"),
    ["# 결과", "- rc done", "", "# 미해결", "- 없음", "", "# 보완", "- 초기 보완"].join("\n"),
    "utf8"
  );
  fs.mkdirSync(path.join(tmpPath, ".project", "screenshot"), { recursive: true });
  fs.writeFileSync(
    path.join(tmpPath, ".project", "screenshot", "capture-one.png"),
    Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9WlAbwAAAABJRU5ErkJggg==",
      "base64"
    )
  );

  await page.goto("/");
  await page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first().click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  await expect(page.getByTestId("check-pane")).toBeVisible();
  await expect(page.getByTestId("check-pane-subject")).toContainText("manual_review");
  await expect(page.getByTestId("check-step-pane")).toContainText("메인 흐름이 정상 동작해야 한다");
  await expect(page.getByTestId("check-report-button")).toBeEnabled();

  await page.getByTestId("check-report-button").click();
  await expect(page.getByTestId("feedback-report-modal")).toContainText("초기 보완");
  await page.getByRole("button", { name: "취소" }).click();

  await page.getByTestId("check-screenshot-card-capture-one.png").click();
  await page.getByTestId("check-feedback-input").fill("버튼 여백을 정리한다");
  await page.getByTestId("check-feedback-add").click();

  await expect
    .poll(() => fs.readFileSync(path.join(tmpPath, ".project", "feedback.md"), "utf8"), {
      timeout: 10_000
    })
    .toContain(`{${path.join(tmpPath, ".project", "screenshot", "capture-one.png")}}에서 버튼 여백을 정리한다`);
});

test("web ui: voice input updates single-line and multiline text fields", async ({ page, request }) => {
  const unique = `pw-voice-${Date.now()}`;
  const tmpPath = `/tmp/${unique}`;
  trackPathForCleanup(tmpPath);
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });

  await installMockSpeechRecognition(page, [["이 파일은", "파일은", "파일은 조금 더 다듬자"], "테스트 음성 입력"]);

  let projectId = "";
  try {
    const createRes = await createProjectForTest(request, {
      name: unique,
      description: "voice input verification",
      path: tmpPath,
      spec: "react, voice",
      project_type: "code"
    });
    const createBody = (await createRes.json()) as { project?: { id?: string } };
    projectId = createBody.project?.id ?? "";

    await page.goto("/");
    const card = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
    await expect(card).toBeVisible();

    await card.click({ force: true });
    await expect(page.getByTestId("project-item-edit")).toBeVisible();
    await page.getByTestId("project-item-edit").click({ force: true });
    await expect(page.getByTestId("edit-goal-voice")).toBeEnabled();
    await page.getByTestId("edit-goal-voice").click();
    await expect(page.getByTestId("edit-goal")).toHaveValue("init 이 파일은 조금 더 다듬자");
    await page.getByRole("button", { name: /cancel/i }).click();

    await page.getByTestId("tab-detail").click({ force: true });
    await expect(page.getByTestId("check-feedback-input-voice")).toBeEnabled();
    await page.getByTestId("check-feedback-input-voice").click();
    await expect(page.getByTestId("check-feedback-input")).toHaveValue("테스트 음성 입력");
  } finally {
    if (projectId) trackedProjectIds.add(projectId);
  }
});

test("web ui: auto modal starts immediately, locks current detail, and keeps project auto state while browsing", async ({
  page,
  request
}) => {
  const autoProject = `pw-auto-${Date.now()}`;
  const otherProject = `${autoProject}-other`;
  const autoPath = `/tmp/${autoProject}`;
  const otherPath = `/tmp/${otherProject}`;
  trackPathForCleanup(autoPath);
  trackPathForCleanup(otherPath);
  fs.rmSync(autoPath, { recursive: true, force: true });
  fs.rmSync(otherPath, { recursive: true, force: true });
  fs.mkdirSync(autoPath, { recursive: true });
  fs.mkdirSync(otherPath, { recursive: true });

  for (const [name, projectPath] of [
    [autoProject, autoPath],
    [otherProject, otherPath]
  ] as const) {
    await createProjectForTest(request, {
      name,
      description: "auto ui verification",
      path: projectPath,
      spec: "react, zustand",
      project_type: "code"
    });
  }

  await page.goto("/");
  const autoCard = page.locator(`[data-testid^="project-item-"]`, { hasText: autoProject }).first();
  const otherCard = page.locator(`[data-testid^="project-item-"]`, { hasText: otherProject }).first();
  await autoCard.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  await page.getByTestId("detail-auto-button").click();
  await expect(page.getByText("요청 메시지")).toBeVisible();
  expect(await page.getByRole("button", { name: "확인" }).count()).toBe(0);
  await page.getByPlaceholder("요청 내용을 입력하세요").fill("자동");
  await page.getByRole("button", { name: "요청하기" }).click();

  await expect(page.getByText("요청 메시지")).toHaveCount(0);
  await expect(page.getByTestId("detail-auto-indicator")).toContainText("auto 중");
  await expect(page.getByTestId("detail-auto-button")).toBeDisabled();
  await expect(page.getByTestId("detail-test-button")).toBeDisabled();
  await expect(page.getByTestId("draft-action-build")).toBeDisabled();

  await page.getByTestId("tab-project").click({ force: true });
  await expect(autoCard).toContainText("auto");
  await expect(page.getByTestId("tab-project")).toBeVisible();

  await otherCard.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });
  await expect(page.getByTestId("detail-project-name")).toContainText(otherProject);

  await page.getByTestId("tab-project").click({ force: true });
  await expect
    .poll(async () => (await autoCard.textContent()) ?? "", { timeout: 10_000 })
    .toMatch(/auto|complete/i);
});

test("web ui: all projects expose detail actions and buttons stay operable", async ({ page, request }) => {
  const base = `pw-buttons-${Date.now()}`;
  const projects = [
    { name: `${base}-a`, path: `/tmp/${base}-a` },
    { name: `${base}-b`, path: `/tmp/${base}-b` }
  ];
  for (const project of projects) {
    trackPathForCleanup(project.path);
    fs.rmSync(project.path, { recursive: true, force: true });
    fs.mkdirSync(project.path, { recursive: true });
    await createProjectForTest(request, {
      name: project.name,
      description: "button coverage verification",
      path: project.path,
      spec: "react, button",
      project_type: "code"
    });
  }

  await page.goto("/");

  for (const project of projects) {
    await page.getByTestId("tab-project").click({ force: true });
    const projectCard = page.locator(`[data-testid^="project-item-"]`, { hasText: project.name }).first();
    await expect(projectCard).toBeVisible();
    await projectCard.click({ force: true });
    await page.getByTestId("tab-detail").click({ force: true });

    await expect(page.getByTestId("detail-project-name")).toContainText(project.name);
    await expect(page.getByRole("button", { name: "delete-job-md" })).toBeVisible();
    await expect(page.getByRole("button", { name: "delete-drafts-yaml" })).toBeVisible();
    await expect(page.getByTestId("generate-job-and-drafts")).toBeVisible();
    await expect(page.getByTestId("open-draft-pane-settings")).toBeVisible();
    await expect(page.getByTestId("detail-auto-button")).toBeEnabled();
    await expect(page.getByTestId("detail-test-button")).toBeEnabled();
    await expect(page.getByTestId("draft-action-build")).toBeEnabled();
    await expect(page.getByRole("button", { name: "delete-job-md" })).toBeEnabled();
    await expect(page.getByRole("button", { name: "delete-drafts-yaml" })).toBeEnabled();
    await expect(page.getByRole("button", { name: "open-requirement-modal" })).toBeEnabled();
    await expect(page.getByTestId("open-draft-pane-settings")).toBeEnabled();

    await page.getByTestId("detail-auto-button").click();
    await expect(page.getByText("요청 메시지")).toBeVisible();
    await page.getByRole("button", { name: /cancel/i }).click();
    await expect(page.getByText("요청 메시지")).toHaveCount(0);

    const featureName = `${project.name.replace(/[^a-zA-Z0-9_-]/g, "_")}_feature`;
    await page.getByRole("button", { name: "open-requirement-modal" }).click();
    await expect(page.getByRole("heading", { name: "요구사항 추가" })).toBeVisible();
    await page
      .getByPlaceholder("## 기능 이름\n- 기능(옵션)\n> 순서(옵션)\n\n## 다른 기능\n- 규칙")
      .fill([`## ${featureName}`, "- 버튼 검증", "> detail pane 반영"].join("\n"));
    await page.getByRole("button", { name: "저장" }).click();
    await expect(page.getByRole("heading", { name: "요구사항 추가" })).toHaveCount(0);
    await expect(page.getByText(featureName)).toBeVisible();
    await page.getByTestId("generate-job-and-drafts").click();

    const jobPath = path.join(project.path, "job.md");
    const draftsPath = path.join(project.path, ".project", "drafts.yaml");
    await expect
      .poll(() => fs.existsSync(jobPath), {
        timeout: 10_000
      })
      .toBeTruthy();
    await expect
      .poll(() => fs.existsSync(draftsPath), {
        timeout: 10_000
      })
      .toBeTruthy();
    await expect
      .poll(() => fs.readFileSync(jobPath, "utf8"), { timeout: 10_000 })
      .toContain(`## ${featureName}`);
    const normalizedDraftName = featureName.replace(/-/g, "_");
    await expect
      .poll(() => fs.readFileSync(draftsPath, "utf8"), { timeout: 10_000 })
      .toContain(`name: ${normalizedDraftName}`);
  }
});
