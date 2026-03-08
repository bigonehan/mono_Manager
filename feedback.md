# Web Run-Dev Feedback (2026-03-08)

## 재현된 문제
- 대상: monorepo template `web/astro`, `web/blog`
- 증상: Web UI에서는 `bun run dev started`로 표시되지만, 링크 접속 시 빈 화면/미응답
- 재현 근거:
  - `bun run dev -- --port 4388 --host 127.0.0.1` 실행 로그에 실제 URL이 `http://127.0.0.1:4321/`로 출력됨
  - `bun run dev -- --port 4389 --host 127.0.0.1` 실행 로그에 실제 URL이 `http://127.0.0.1:4322/`로 출력됨
  - 즉, Web UI가 가정한 요청 포트(43xx)와 실제 dev 서버 포트가 불일치

## 원인
- 템플릿 스크립트(`astro dev --port 4321`)에 포트가 고정되어 있어, 외부에서 전달한 포트 인자가 실제 바인딩 포트와 다를 수 있었음.
- 기존 Web UI는 "요청 포트"를 기준으로 링크를 생성해 잘못된 주소를 표시함.

## 개선 사항
- `runProjectDev`에서 stdout/stderr 로그를 라인 단위로 파싱해 실제 URL을 추출하도록 수정
- URL 패턴 감지 후 `runUrlsByProject`에 저장
- `loadProjectDetail`에서 실행 중 프로젝트의 `dev_server_url`을 실제 감지 URL로 제공
- `run-dev` 응답에서 URL 감지를 최대 2.5초 대기 후 전달하여 버튼 상단 링크가 즉시 표시되도록 보완
- 프로세스 종료/오류/stop 시 URL 상태 정리

## 남은 문제 점검
- 현재 확인 기준에서 남은 미해결 문제 없음
- 검증:
  - `npm --prefix assets/web run test:e2e` 통과
  - 호출 경로 점검: `runDevServer -> /api/run-dev -> runProjectDev -> runtime url capture -> detail.dev_server_url -> UI link`
## 2026-03-08 run-dev investigation (template/web/next)
- 증상 재현: `test` 실행 후 표시된 URL(`http://127.0.0.1:4751`) 접속 불가
- 원인 1(템플릿 코드): Next app route 충돌
  - 에러: `You cannot use different slug names for the same dynamic path ('commentId' !== 'parentId')`
  - 충돌 경로: `app/api/comments/[commentId]/...` 와 `app/api/comments/[parentId]/replies/...`
- 조치 1: replies 경로를 `[commentId]/replies`로 통일해 Next 부팅 오류 해결
  - 수정 파일: `/home/tree/home/template/web/next/app/api/comments/[commentId]/replies/route.ts`
- 원인 2(웹 서버 실행기): Astro 템플릿에서 `bun run dev -- --port <p>`가 스크립트 기본 포트(`4321`)에 덮이지 않아 UI URL 포트와 실제 포트가 불일치
- 조치 2: `assets/web/src/server/orc.ts`의 dev 실행 명령 분기 보강
  - Next: `bunx next dev --port <p> --hostname 127.0.0.1`
  - Astro: `bunx astro dev --port <p> --host 127.0.0.1`
  - Generic: 기존 `bun run dev -- --port <p>` 유지
- 조치 3: `runProjectDev` 초기 실패 감지 추가
  - 시작 직후 프로세스 생존 여부 확인 후 조기 종료면 `running=false`로 응답
  - URL 미검출 시에도 생존 중이면 `http://127.0.0.1:<port>` fallback URL 사용
- 검증 결과
  - next: `http://127.0.0.1:4751` -> `HTTP/1.1 200 OK`
  - astro: `http://127.0.0.1:4752` -> `HTTP/1.1 200 OK`
  - blog: `http://127.0.0.1:4753` -> `HTTP/1.1 200 OK`
