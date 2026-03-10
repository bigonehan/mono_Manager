# 기능 계획: task artifact reset + project feedback unification

## 범위
- orc의 새 작업이 시작될 때 `.project/check-process.md`, `.project/feedback.md`, `.project/screenshot`를 한 번만 초기화한다.
- web UI와 `rc` 결과 경로를 루트 `feedback.md`가 아니라 `.project/feedback.md`로 단일화한다.
- web check pane의 수동 check는 `orc`와 같은 공통 binary resolver를 사용해 `rc` 실행 경로를 결정한다.
- Playwright 음성 입력 mock은 `assets/web/tests/helpers`로 분리하고 기존 테스트가 helper를 재사용하게 바꾼다.

## 입출력
- 입력: `orc` top-level task 시작, web check pane의 manual rc check, screenshot feedback/report/retry, Playwright 음성 입력 테스트
- 출력: 이전 task 산출물 삭제 후 새 process log 시작, `.project/feedback.md` 갱신/조회, 공통 resolver 기반 `rc` 실행, helper 기반 음성 mock 테스트

## 영향 범위
- `src/main.rs`
- `src/code.rs`
- `src/bin/rc.rs`
- `assets/web/src/server/orc.ts`
- `assets/web/src/components/WebApp.tsx`
- `assets/web/src/pages/api/*.ts`
- `assets/web/tests/web.spec.ts`
- `assets/web/tests/helpers/mock-speech-recognition.ts`

## 검증
- `rg -n "check-process|project_feedback_path|feedback.md|resolveOrcCommandArgs|resolveRcCommandArgs|mock-speech-recognition" /home/tree/project/rust-orc/src /home/tree/project/rust-orc/assets/web/src /home/tree/project/rust-orc/assets/web/tests`
- `cd /home/tree/project/rust-orc/assets/web && npx tsc --noEmit`
- `cd /home/tree/project/rust-orc && cargo test`
- `cd /home/tree/project/rust-orc/assets/web && npm run test:e2e -- --grep "voice input"`
