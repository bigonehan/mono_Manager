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

test("web ui: domain badge click refreshes source-derived feature list", async ({ page, request }) => {
  const unique = `pw-domain-${Date.now()}`;
  const tmpPath = `/tmp/${unique}`;
  trackPathForCleanup(tmpPath);
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(path.join(tmpPath, "src"), { recursive: true });

  await createProjectForTest(request, {
    name: unique,
    description: "domain pane verification",
    path: tmpPath,
    spec: "react, domain",
    project_type: "code"
  });

  fs.writeFileSync(
    path.join(tmpPath, ".project", "project.md"),
    [
      "# info",
      "name: domain-e2e",
      "description: domain feature sync",
      "spec: react",
      "goal: validate domains",
      "",
      "# rules",
      "- ",
      "",
      "# constraints",
      "- ",
      "",
      "# features",
      "- ",
      "",
      "# domains",
      "## accounts",
      "### description",
      "- accounts domain",
      "### feature",
      "- ",
      "",
      "## orders",
      "### description",
      "- orders domain",
      "### feature",
      "- ",
      ""
    ].join("\n"),
    "utf8"
  );
  fs.writeFileSync(path.join(tmpPath, "src", "accounts_service.ts"), "export function accountsLogin() { return true; }\n", "utf8");

  await page.goto("/");
  const card = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
  await expect(card).toBeVisible();
  await card.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  const domainsPane = page.getByTestId("detail-pane-domains");
  await expect(domainsPane).toBeVisible();
  await domainsPane.getByText("accounts").first().click();
  await expect(domainsPane.getByText("accountsLogin", { exact: false })).toBeVisible();
});

test("web ui: mono detail domains pane renders project domains", async ({ page, request }) => {
  const unique = `pw-mono-domain-${Date.now()}`;
  const monoDomainName = `billing-${Date.now()}`;
  const tmpPath = path.join("/home/tree/home/apps", unique);
  const monoDomainRoot = path.join("/home/tree/home/packages/domains", monoDomainName);
  trackPathForCleanup(tmpPath);
  trackPathForCleanup(monoDomainRoot);
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.rmSync(monoDomainRoot, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });
  fs.mkdirSync(path.join(monoDomainRoot, "src"), { recursive: true });
  fs.writeFileSync(
    path.join(monoDomainRoot, "src", `${monoDomainName}.ts`),
    `export function ${monoDomainName.replace(/-/g, "_")}Sync() { return "ok"; }\n`,
    "utf8"
  );

  await createProjectForTest(request, {
    name: unique,
    description: "mono domain pane verification",
    path: tmpPath,
    spec: "mono, domains",
    project_type: "mono"
  });

  await page.goto("/");
  const card = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
  await expect(card).toBeVisible();
  await card.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  const domainsPane = page.getByTestId("detail-pane-domains");
  await expect(domainsPane).toContainText(monoDomainName);
  await domainsPane.getByText(monoDomainName).first().click();
  await expect(domainsPane).toContainText("Sync");
});

test("web ui: drafts pane uses add/build actions and stage lock overlays", async ({ page, request }) => {
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
  await expect(page.getByRole("button", { name: "add" })).toBeVisible();
  await expect(page.getByRole("button", { name: /^build$/i })).toHaveCount(0);
  await expect(page.getByTestId("generate-job-and-drafts")).toHaveCount(0);
  await expect(page.getByTestId("open-draft-pane-settings")).toBeVisible();
  await expect(page.getByTestId("draft-action-build")).toBeVisible();
  await expect(
    page
      .getByTestId("drafts-raw-pane")
      .locator('[data-testid="drafts-raw-editor-voice"]')
  ).toHaveCount(0);
  const topRightActions = page.getByTestId("requirements-top-right-actions");
  await expect(topRightActions).toBeVisible();
  const [reqBox, topBox, plusBox] = await Promise.all([
    requirementsContainer.boundingBox(),
    topRightActions.boundingBox(),
    page.getByTestId("open-requirement-modal").boundingBox()
  ]);
  expect(reqBox).not.toBeNull();
  expect(topBox).not.toBeNull();
  expect(plusBox).not.toBeNull();
  if (reqBox && topBox && plusBox) {
    expect(topBox.y).toBeLessThan(reqBox.y + reqBox.height * 0.35);
    expect(topBox.x + topBox.width).toBeGreaterThan(reqBox.x + reqBox.width * 0.8);
    expect(plusBox.y).toBeGreaterThan(reqBox.y + reqBox.height);
  }
  const [buildPencilBox, editPencilBox] = await Promise.all([
    page.getByTestId("draft-action-build").boundingBox(),
    page.getByTestId("open-draft-pane-settings").boundingBox()
  ]);
  expect(buildPencilBox).not.toBeNull();
  expect(editPencilBox).not.toBeNull();
  if (buildPencilBox && editPencilBox) {
    expect(Math.abs(buildPencilBox.y - editPencilBox.y)).toBeLessThanOrEqual(2);
  }
  await expect(page.getByTestId("draft-work-pane-lock-overlay")).toBeVisible();
  await expect(page.getByTestId("check-pane-lock-overlay")).toBeVisible();
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
  await expect(page.getByTestId("draft-work-pane-lock-overlay")).toHaveCount(0);

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
  await expect(page.getByRole("button", { name: "retry_red_items" })).toBeVisible();
  await expect(page.getByRole("button", { name: "finalize_green_items" })).toBeVisible();
  const [workPaneBox, workActionsBox] = await Promise.all([
    page.getByTestId("draft-work-pane").boundingBox(),
    page.getByTestId("work-pane-review-actions").boundingBox()
  ]);
  expect(workPaneBox).not.toBeNull();
  expect(workActionsBox).not.toBeNull();
  if (workPaneBox && workActionsBox) {
    expect(workActionsBox.y).toBeGreaterThan(workPaneBox.y + workPaneBox.height * 0.65);
    expect(workActionsBox.x + workActionsBox.width).toBeGreaterThan(workPaneBox.x + workPaneBox.width * 0.75);
  }

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
      "planned: []",
      "worked: []",
      "complete:",
      "  - manual_review",
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
    await expect(page.getByTestId("edit-goal")).toHaveValue("init");
    await page.waitForTimeout(120);
    await page.getByTestId("edit-goal-voice").click();
    await expect(page.getByTestId("edit-goal")).toHaveValue("init 이 파일은 조금 더 다듬자");
    await page.getByRole("button", { name: /cancel/i }).click();

    fs.writeFileSync(
      path.join(tmpPath, ".project", "drafts.yaml"),
      [
        "draft:",
        "  - name: voice_ready",
        "    check:",
        "      - check pane input voice button enabled",
        "planned: []",
        "worked: []",
        "complete:",
        "  - voice_ready",
        "failed: []"
      ].join("\n"),
      "utf8"
    );

    await page.reload();
    const detailCard = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
    await expect(detailCard).toBeVisible();
    await detailCard.click({ force: true });

    await page.getByTestId("tab-detail").click({ force: true });
    await expect(page.getByTestId("check-feedback-input-voice")).toBeEnabled();
    await page.getByTestId("check-feedback-input-voice").click();
    await expect(page.getByTestId("check-feedback-input")).toHaveValue("");
    await page.waitForTimeout(120);
    await page.getByTestId("check-feedback-input-voice").click();
    await expect(page.getByTestId("check-feedback-input")).toHaveValue(/(테스트 음성 입력|다듬자)/);
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
    await expect(page.getByRole("button", { name: "add" })).toBeVisible();
    await expect(page.getByTestId("generate-job-and-drafts")).toHaveCount(0);
    await expect(page.getByTestId("open-draft-pane-settings")).toBeVisible();
    await expect(page.getByTestId("detail-auto-button")).toBeEnabled();
    await expect(page.getByTestId("detail-test-button")).toBeEnabled();
    await expect(page.getByTestId("draft-action-build")).toBeEnabled();
    await expect(page.getByRole("button", { name: "delete-job-md" })).toBeEnabled();
    await expect(page.getByRole("button", { name: "delete-drafts-yaml" })).toBeEnabled();
    await expect(page.getByRole("button", { name: "open-requirement-modal" })).toBeEnabled();
    await expect(page.getByTestId("open-draft-pane-settings")).toBeEnabled();
    const [searchInputBox, searchIconBox] = await Promise.all([
      page.getByRole("textbox", { name: "detail-sidebar-search" }).boundingBox(),
      page.getByTestId("detail-sidebar-search-icon").boundingBox()
    ]);
    expect(searchInputBox).not.toBeNull();
    expect(searchIconBox).not.toBeNull();
    if (searchInputBox && searchIconBox) {
      const iconCenterX = searchIconBox.x + searchIconBox.width / 2;
      expect(iconCenterX).toBeGreaterThan(searchInputBox.x + searchInputBox.width * 0.75);
      expect(iconCenterX).toBeLessThan(searchInputBox.x + searchInputBox.width);
    }

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


test("web ui: detail desktop aligns sidebar and main pane shells for code/mono", async ({ page, request }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const screenshotDir = path.join(process.cwd(), "..", "..", ".agents", "artifacts");
  fs.mkdirSync(screenshotDir, { recursive: true });

  const cases: Array<{ type: "code" | "mono"; name: string; root: string }> = [
    { type: "code", name: `pw-align-code-${Date.now()}`, root: `/tmp/pw-align-code-${Date.now()}` },
    { type: "mono", name: `pw-align-mono-${Date.now()}`, root: path.join("/home/tree/home/apps", `pw-align-mono-${Date.now()}`) }
  ];

  for (const c of cases) {
    trackPathForCleanup(c.root);
    fs.rmSync(c.root, { recursive: true, force: true });
    fs.mkdirSync(c.root, { recursive: true });

    await createProjectForTest(request, {
      name: c.name,
      description: `${c.type} alignment e2e`,
      path: c.root,
      spec: "react, layout",
      project_type: c.type
    });

    await page.goto("/");
    const card = page.locator(`[data-testid^="project-item-"]`, { hasText: c.name }).first();
    await expect(card).toBeVisible();
    await card.click({ force: true });
    await page.getByTestId("tab-detail").click({ force: true });

    await expect(page.getByTestId("detail-sidebar-shell")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("detail-sidebar-card")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("detail-main-shell")).toBeVisible({ timeout: 20_000 });
    await expect
      .poll(
        async () => {
          const [sidebarCardBox, projectPaneBox] = await Promise.all([
            page.getByTestId("detail-sidebar-card").boundingBox(),
            page.getByTestId("detail-main-shell").boundingBox()
          ]);
          if (!sidebarCardBox || !projectPaneBox) return false;
          const topDiff = Math.abs(sidebarCardBox.y - projectPaneBox.y);
          return topDiff <= 8;
        },
        { timeout: 20_000 }
      )
      .toBeTruthy();

    await page.screenshot({ path: path.join(screenshotDir, `mono-detail-shell-align-${c.type}.png`), fullPage: true });
  }
});
