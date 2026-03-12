import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { installMockSpeechRecognition } from "./helpers/mock-speech-recognition";

test("web ui: load and create/select project", async ({ page, request }) => {
  const unique = `pw-${Date.now()}`;
  const tmpPath = "/tmp/tmp_project";
  fs.mkdirSync(tmpPath, { recursive: true });
  fs.rmSync(path.join(tmpPath, ".project"), { recursive: true, force: true });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Code" })).toBeVisible();
  const createRes = await request.post("http://127.0.0.1:4175/api/projects", {
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

test("web ui: episode pane ordering, read modal, and draft action disabled state", async ({ page, request }) => {
  const unique = `pw-episode-${Date.now()}`;
  const tmpPath = `/tmp/${unique}`;
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });

  await page.goto("/");
  const createRes = await request.post("http://127.0.0.1:4175/api/projects", {
    data: {
      name: unique,
      description: "episode ui verification",
      path: tmpPath,
      spec: "story, episode",
      project_type: "code"
    }
  });
  expect(createRes.ok()).toBeTruthy();

  await page.reload();
  const card = page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first();
  await card.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  await expect(page.getByTestId("draft-action-add")).toBeDisabled();
  await expect(page.getByTestId("draft-action-modify")).toBeDisabled();

  fs.writeFileSync(
    path.join(tmpPath, "input.md"),
    [
      "# Episode One",
      "- 감정선 유지 > 바닷가에서 시작해 관계 변화를 드러낸다",
      "",
      "첫 문단은 바닷가의 공기와 인물의 긴장을 보여준다.",
      "",
      "## 전개",
      "- 두 인물의 대화를 통해 갈등을 밀어 올린다"
    ].join("\n"),
    "utf8"
  );
  fs.writeFileSync(
    path.join(tmpPath, ".project", "drafts.yaml"),
    [
      "draft:",
      "  - name: episode_one",
      "    scope:",
      "      - story/episode-one.md",
      "planned:",
      "  - episode_one",
      "worked: []",
      "complete: []",
      "failed: []"
    ].join("\n"),
    "utf8"
  );

  await page.reload();
  await page.locator(`[data-testid^="project-item-"]`, { hasText: unique }).first().click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  const episodePane = page.getByTestId("episode-pane");
  const draftPane = page.getByTestId("draft-pane");
  const draftActionRow = page.getByTestId("draft-action-row");
  const episodeBox = await episodePane.boundingBox();
  const draftBox = await draftPane.boundingBox();
  const actionBox = await draftActionRow.boundingBox();
  expect(episodeBox).not.toBeNull();
  expect(draftBox).not.toBeNull();
  expect(actionBox).not.toBeNull();
  expect((episodeBox?.y ?? 0) < (draftBox?.y ?? 0)).toBeTruthy();
  expect((draftBox?.y ?? 0) < (actionBox?.y ?? 0)).toBeTruthy();

  await expect(page.getByTestId("draft-action-add")).toBeEnabled();
  await expect(page.getByTestId("draft-action-modify")).toBeEnabled();
  await page.getByRole("button", { name: "Episode One" }).click();
  await page.getByTestId("episode-read-button").click();
  await expect(page.getByTestId("episode-read-modal")).toBeVisible();
  await expect(page.getByTestId("episode-read-modal")).toContainText("Episode One");
  await expect(page.getByTestId("episode-read-modal")).toContainText("첫 문단은 바닷가의 공기와 인물의 긴장을 보여준다.");
});

test("web ui: check pane renders draft subject and appends screenshot feedback", async ({ page, request }) => {
  const unique = `pw-check-${Date.now()}`;
  const tmpPath = `/tmp/${unique}`;
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });

  const createRes = await request.post("http://127.0.0.1:4175/api/projects", {
    data: {
      name: unique,
      description: "check pane verification",
      path: tmpPath,
      spec: "react, check",
      project_type: "code"
    }
  });
  expect(createRes.ok()).toBeTruthy();

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
  fs.rmSync(tmpPath, { recursive: true, force: true });
  fs.mkdirSync(tmpPath, { recursive: true });

  await installMockSpeechRecognition(page, [["이 파일은", "파일은", "파일은 조금 더 다듬자"], "테스트 음성 입력"]);

  let projectId = "";
  try {
    const createRes = await request.post("http://127.0.0.1:4175/api/projects", {
      data: {
        name: unique,
        description: "voice input verification",
        path: tmpPath,
        spec: "react, voice",
        project_type: "code"
      }
    });
    expect(createRes.ok()).toBeTruthy();
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
    if (projectId) {
      await request.post("http://127.0.0.1:4175/api/project-delete", {
        data: { id: projectId }
      });
    }
    fs.rmSync(tmpPath, { recursive: true, force: true });
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
  fs.rmSync(autoPath, { recursive: true, force: true });
  fs.rmSync(otherPath, { recursive: true, force: true });
  fs.mkdirSync(autoPath, { recursive: true });
  fs.mkdirSync(otherPath, { recursive: true });

  for (const [name, projectPath] of [
    [autoProject, autoPath],
    [otherProject, otherPath]
  ] as const) {
    const createRes = await request.post("http://127.0.0.1:4175/api/projects", {
      data: {
        name,
        description: "auto ui verification",
        path: projectPath,
        spec: "react, zustand",
        project_type: "code"
      }
    });
    expect(createRes.ok()).toBeTruthy();
  }

  await page.goto("/");
  const autoCard = page.locator(`[data-testid^="project-item-"]`, { hasText: autoProject }).first();
  const otherCard = page.locator(`[data-testid^="project-item-"]`, { hasText: otherProject }).first();
  await autoCard.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });

  await page.getByTestId("detail-auto-button").click();
  await expect(page.getByText("요청 메시지")).toBeVisible();
  expect(await page.getByRole("button", { name: "확인" }).count()).toBe(0);
  await page.getByPlaceholder("요청 내용을 입력하세요").fill("auto flow test");
  await page.getByRole("button", { name: "요청하기" }).click();

  await expect(page.getByText("요청 메시지")).toHaveCount(0);
  await expect(page.getByTestId("detail-auto-indicator")).toContainText("auto 중");
  await expect(page.getByTestId("detail-auto-button")).toBeDisabled();
  await expect(page.getByTestId("detail-test-button")).toBeDisabled();
  await expect(page.getByTestId("draft-action-build")).toBeDisabled();

  await page.getByTestId("tab-project").click({ force: true });
  await expect(autoCard).toContainText("auto");
  await expect
    .poll(async () => (await autoCard.textContent()) ?? "", { timeout: 10_000 })
    .toContain("stage: ");

  await otherCard.click({ force: true });
  await page.getByTestId("tab-detail").click({ force: true });
  await expect(page.getByTestId("detail-project-name")).toContainText(otherProject);

  await page.getByTestId("tab-project").click({ force: true });
  await expect(autoCard).toContainText("auto");
  await expect
    .poll(async () => (await autoCard.textContent()) ?? "", { timeout: 10_000 })
    .toContain("complete");
});
