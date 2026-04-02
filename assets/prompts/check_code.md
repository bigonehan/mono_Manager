---
name: check-code
description: "입력된 기능/목표 기준 체크리스트 생성 후 언어별 실행 검증을 수행하는 skill"
---

# Check Code

## 원칙
- 체크 항목 형식은 반드시 아래 한 줄 패턴만 사용한다:
- 입력 형식은 다음과 같다 ` ${입력} -> ${출력} : 기능설명`

# 테스트 항목 설정 
## job.md 반영 규칙
- 별도 체크리스트 파일은 만들지 않는다.
- 각 기능/목표를 검증 가능한 입력/출력 단위로 분해해 항목을 만든다.
- `job.md`를 먼저 읽고 `# task ## verify` 항목과 requirement를 기준으로 `# check` 체크리스트를 재구성한다.
- `# problems`에 있는 항목을 `# check`보다 먼저 처리한다.
- 검증 중 새로 발견한 문제는 `job.md#problems` 에 추가한다. 이때 항목은 `${입력} 되면 ${출력}~야 한다` 식으로 추가한다. 
- 관련된 유닛테스트를 돌리고, unit test가 없다면 생성해서 검증을 통과하는진 확인한다 
- `# problems`가 남아 있으면 문제 해결 -> `# check` 재생성 -> 재검증 루프를 반복한다.
- 검증 완료가 확인된 항목은 `job.md#task#verify`에서 `job.md#task#complete`로 이동한다.
- 해결 되지 못한 문제는 `job.md#problems`에 남긴다.
# 테스트 실행 
## 실행 검증 규칙
- 현재 코드 언어를 확인하고 아래 명령 체계를 사용한다.
- Rust:
  - `cargo test`
  - 필요 시 `cargo check`
- JS/TS 계열:
  - `vitest` 기반 테스트 실행
  - 라우팅/폼 제출 같은 사용자 흐름은 `playwright` E2E 실행
  - `playwright` 모듈 로딩 실패(`Cannot find module '@playwright/test'`, `Cannot find module 'playwright'`)는 검증 실패가 아니라 러너 준비 실패로 분류한다.
  - 위 실패가 나면 mono_Manager에 이미 설치된 workspace(`.../assets/web`)를 기준으로 같은 브라우저 검증 스크립트를 다시 실행한다.
  - 재실행 기본 명령은 `python /home/tree/project/mono_Manager/scripts/run_playwright_qa.py --web-root /home/tree/project/mono_Manager/assets/web -- <same command>` 를 사용한다.
  - 설치 workspace 기준 재실행에서도 같은 오류가 나면 그때만 실제 dependency 누락 또는 스크립트 문제로 판정한다.
- 구현 함수의 반환값이 실제 로직 없이 고정값인지 반드시 점검한다.
  - 예: `Ok(false)`, `Ok(true)`, `return false`, `return true` 같은 하드코딩 반환
  - 입력값/상태/외부결과를 사용하지 않는 고정 반환이면 검증 실패로 기록한다.
- 문자열/패턴 기반의 하드코딩 성공/실패 판정도 반드시 점검한다.
  - 예: `contains("Logout")`, `contains("success")`, `starts_with("ok")`, `ends_with("done")`
  - 위 조건이 입력/상태/외부결과 검증 없이 최종 판정 분기로 쓰이면 검증 실패로 기록한다.
  - 권장 점검 명령 예시:
    - `rg -n "contains\\(|starts_with\\(|ends_with\\(" src crates packages`

## 테스트 워크 플로우 
- 먼저 `drafts.yaml` 내부의 `item`들을 순회하면서 `drafts_item.yaml`에서 `constraints`기능/목표를 읽는다.
- 위 입력과 `job.md#task#verify`, `job.md#problems`를 바탕으로 검증 체크리스트를 구성하고 `.job.md#check`에 item 들을 더한다 
- 이때 `item` 입력 형식은 `draft_item.name`:`검사해볼 기능` 형식으로 작성한다.
- 검증 체크리스트에 알맞은 유닛테스트들을 생성한다. 
- 사용자 입력들이 필요한 경우 mock 객체를 생성해서 사용 
- 유넷 테스트 실행후 pass 확인 
- playwirght로(js/ts인 경우)로 headless 브라우저로 기능 실행후 `checklist`에 있는 기능을 수행하는지 스크린샷 
- 디자인이나 화면 ui 요청이 있는경우또한 playwirght로 실행후 스크린샷 캡쳐 
- QA 재실행 시에는 설치 workspace(`.../assets/web`)를 `cwd`로 사용하고 `NODE_PATH`와 `node_modules/.bin` 경로를 그 workspace 기준으로 연결한 상태에서 동일 스크립트를 실행해, 모듈 탐색 경로 문제와 실제 테스트 실패를 분리한다.

# 완료 처리 
- 완료 시 해결된 항목은 `.job.md#task#verify` 에서 `.job.md#task#complete`로 이동 
- 완료 시 미해결된 항목은 `.job.md#problems`에 유지 
- 미해결된 항목이 있는 경우 `# problems` 해결 -> `# check` 재생성 -> 재검증 루프를 반복함 
