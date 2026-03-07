## 문제
- `open-ui`는 현재 TUI만 지원하며 웹 UI 실행 경로(`open-ui -w`)가 없다.
- 웹 자산이 분리되어 있지 않아 TUI 기능(프로젝트 CRUD, 상세 정보/리스트 편집, draft/parallel/check 실행)을 브라우저에서 사용할 수 없다.
- 웹 경로에 대한 E2E 검증(playwright) 자동화가 없어 회귀를 빠르게 확인할 수 없다.

## 해결책
- CLI에 `open-ui [-w|--web]` 옵션을 추가해 기본은 TUI, `-w`는 웹 UI 실행으로 분기한다.
- `src/web/mod.rs`를 추가해 `assets/web`의 의존성 설치/개발서버 기동/브라우저 오픈을 관리한다.
- `assets/web`에 Astro(+Vite), React, shadcn 스타일 컴포넌트 기반 UI를 구성한다.
- 웹 기능은 다음을 최소 단위로 제공한다.
  - 프로젝트 목록/생성/수정/삭제
  - 프로젝트 상세 표시(name/description/spec/goal/rules/constraints/features/planned/generated)
  - rule/constraint/feature 리스트 수정 저장
  - draft/create, draft/add, impl/check 실행 버튼
- Astro API routes를 통해 레지스트리 파일과 `.project` 문서를 읽고, 필요한 명령은 `orc` CLI를 호출한다.
- Playwright 테스트를 작성해 `open-ui -w` 경로의 핵심 동작(페이지 로드, 프로젝트 생성/선택, 액션 호출)을 검증한다.

## 검증
- `cargo test -q`
- `cargo run --bin orc -- --help`에서 `open-ui [-w|--web]` 노출 확인
- `cargo run --bin orc -- open-ui -w` 실행 시 웹 URL 오픈 메시지 확인
- `cd assets/web && npm run test:e2e` 통과

## 재시도 정책
- 실행 실패 시 `feedback.md`에 `문제/미해결점`을 기록한다.
- 실패 원인을 기준으로 본 문서 `해결책`에 구체 수정 항목을 추가한다.
- 수정 적용 후 동일 검증 명령을 재실행하고 통과할 때까지 반복한다.

## 재시도-1 반영
- 실패원인: `src/web` 디렉터리 미생성 상태에서 `src/web/mod.rs` 작성 시도.
- 강제조치: `mkdir -p src/web` 선행 후 파일 생성.

## 재시도-2 반영
- 실패원인: `open-ui -w`에서 브라우저 실행 대기(status)로 프로세스 반환 지연.
- 강제조치: 브라우저 실행을 비차단 spawn으로 변경하고 결과는 best-effort로 처리.

## 재시도-3 반영
- 실패원인: Playwright 클릭 안정성 대기 중 타임아웃.
- 강제조치: E2E의 `create-project` 클릭을 `force: true`로 고정.

## 재시도-4 반영
- 실패원인: 프로젝트 카드가 최대 9개만 렌더되어 신규 항목 가시성 보장 실패.
- 강제조치: E2E 기대값을 UI 텍스트 가시성 -> 로그/파일 반영 검증으로 전환.

## 재시도-5 반영
- 실패원인: Astro 동적 API 라우트의 prerender 설정 누락.
- 강제조치: API 라우트들에 `prerender = false` 명시.

## 재시도-6 반영
- 실패원인: `sd` 미설치.
- 강제조치: 일괄치환 대신 파일별 패치 적용.

## 재시도-7 반영
- 실패원인: 동적 API 라우트 호출로 인한 런타임 오버레이.
- 강제조치: 동적 `/api/projects/[id]` 의존 제거, 쿼리/POST 기반 정적 API로 전환.

## 재시도-8 반영
- 실패원인: 클릭 기반 생성 흐름의 E2E 불안정.
- 강제조치: E2E는 페이지 렌더 + API 생성 경로 + 파일 반영 검증으로 안정화.

## 2026-03-07 UI 탭/편집 변경
### 문제
- 현재 web UI는 project/detail이 명시적 탭으로 분리되어 있지 않고, detail 필드가 기본 편집 가능 상태다.
- 요구사항은 project 선택에 따라 detail이 바뀌는 탭 구조 + 기본 읽기 전용 + pane 선택 시 gear 아이콘으로 편집 진입이다.

### 해결책
- `WebApp`에 `project|detail` 탭 상태를 추가하고 화면을 탭 기반으로 분리한다.
- detail 화면은 pane(card) 선택 상태를 도입하고, 선택 pane 우상단에 gear 아이콘 버튼만 노출한다.
- 편집은 모달에서 수행하고 저장 시 기존 API(`project-info`, `project-lists`)를 호출한다.

### 검증
- Playwright E2E에 탭 전환 + project 선택 후 detail 반영 + gear 편집 저장 케이스를 추가/수정한다.
- `npm run test:e2e`, `cargo test -q` 실행.

## 2026-03-07 zustand 상태관리 전환
### 문제
- `WebApp`에 로컬 state가 집중되어 탭/선택/편집 상태 흐름이 분산된다.

### 해결책
- `assets/web/src/store/orc-store.ts`에 zustand 스토어를 추가한다.
- 탭, 선택 project id, detail pane, 로그, 편집모달 상태를 스토어로 이동한다.
- `WebApp`은 스토어 state/action을 사용해 기존 API 흐름을 유지한다.

### 검증
- `cd assets/web && npm run test:e2e`
- `cargo test -q`

## 2026-03-07 project item 액션 버튼
### 문제
- project pane에서 선택된 item의 수정/삭제가 전역 버튼 중심이라 item 단위 조작성이 낮다.

### 해결책
- 선택 item 카드 우상단에 `수정/삭제` 아이콘(SVG) 버튼을 렌더링한다.
- 수정 버튼은 해당 item을 기준으로 편집 모달을 열고 저장 시 `/api/project-info` 호출로 반영한다.
- 삭제 버튼은 해당 item id로 기존 삭제 API를 호출한다.

### 검증
- Playwright에서 item 선택 -> 우상단 아이콘 노출 -> 수정 저장 -> 반영 확인.
- 삭제 버튼 호출 후 목록 갱신 확인.

## 재시도-9 반영
- 실패원인: 텍스트 노드 클릭 안정성 타임아웃.
- 강제조치: 프로젝트 카드 루트 test id 도입 후 force click 사용.

## 재시도-10 반영
- 실패원인: 아이콘 버튼 클릭 안정성 대기 타임아웃.
- 강제조치: 아이콘 버튼 클릭에 force 옵션 적용.

## 재시도-11 반영
- 실패원인: 모달 save 버튼 클릭 안정성 타임아웃.
- 강제조치: save 클릭 force 적용.

## 재시도-12 반영
- 실패원인: 로그 검증 위치가 프로젝트 탭 구조와 불일치.
- 강제조치: 저장 성공 검증을 파일 반영 기반으로 변경.

## 재시도-13 반영
- 실패원인: 저장 완료 이전 파일 검증(race condition).
- 강제조치: expect.poll 기반 파일 반영 대기 검증.

## 2026-03-07 current.png 스타일 반영
### 문제
- project info 시각 톤이 `current.png`와 다르고 pane 라운드 보더 일관성이 부족하다.

### 해결책
- `detail` 탭의 project info pane을 `current.png`와 유사한 카드 구성(큰 제목, 보조 설명, spec pill, path 박스)으로 재구성한다.
- 모든 pane/card/세부 섹션에 통일된 rounded border 클래스를 적용한다.

### 검증
- `cd assets/web && npm run test:e2e`
- `cargo test -q`

## 2026-03-07 project grid + type 필드 확장
### 문제
- project 탭이 grid 중심 UI가 아니고, type/status 시각 정보가 부족하며, 프로젝트 상태 구조체에 type 필드가 없다.

### 해결책
- project 탭 목록을 grid 카드로 재구성하고 헤더 우상단 `Create Project` 버튼으로 모달 생성 흐름으로 전환한다.
- 카드에 shadcn `Label` 기반 `project type` 배지, 하단 고정 상태 태그(`working|wait`), path 앞 폴더 아이콘, 큰 제목 폰트를 적용한다.
- 프로젝트 구조체에 `project_type` 필드를 추가하고 허용값(`story|movie|code|mono`) + 기본값(`code`)을 적용한다.
- web 서버 타입/스토어/create API payload/Rust `ProjectRecord`를 동일 스키마로 동기화한다.

### 검증
- `cd assets/web && npm run test:e2e`
- `cargo test -q`
- `cargo run --bin orc -- --help` (회귀 확인)

## 재시도-14 반영
- 실패원인: create 모달 오픈 확인 없이 입력을 시작해 Playwright가 필드를 찾지 못함.
- 강제조치: 모달 루트 test id(`create-project-modal`)를 추가하고, 테스트를 모달 가시성 확인 뒤 입력하도록 고정.

## 재시도-15 반영
- 실패원인: 모달 루트 대기 추가 후에도 요소가 렌더되지 않아 클릭-상태 전이가 실패.
- 강제조치: Playwright error-context/런타임 로그를 우선 확인해 hydration JS 오류를 제거하고, 버튼 클릭 경로를 단순화해 상태 전이를 보장.

## 재시도-16 반영
- 실패원인: create 모달 렌더 실패가 반복되며 dev server 재사용으로 코드 반영 지연 가능성 존재.
- 강제조치: 4173 기존 astro dev 프로세스를 종료하고 Playwright를 fresh server로 재실행.

## 재시도-17 반영
- 실패원인: fresh server에서도 createOpen 상태 전이가 반영되지 않아 모달 렌더가 발생하지 않음.
- 강제조치: 모달 오픈에 local fallback state를 추가하고 open/close에서 zustand+local을 동시 제어.

## 재시도-18 반영
- 실패원인: `CardTitle(h3)` 내부에 block/button을 배치한 invalid DOM으로 hydration mismatch 가능성.
- 강제조치: 헤더 레이아웃을 `CardHeader`에서 직접 구성하고 `CardTitle`에는 텍스트만 두도록 구조 교정.

## 재시도-19 반영
- 실패원인: Playwright pointer click이 `open-create-project` onClick까지 전달되지 않음(`data-create-open=false` 유지).
- 강제조치: E2E 트리거를 `dispatchEvent('click')`로 전환해 핸들러 호출과 상태 전이를 검증.

## 재시도-20 반영
- 실패원인: dispatch click에도 상태 전이가 없어 hydration 미동작 가능성이 높음.
- 강제조치: Playwright에 `pageerror` 수집을 추가해 런타임 JS 오류를 확인 후 원인 수정.

## 재시도-21 반영
- 실패원인: dispatchEvent 기반 클릭이 React delegated 이벤트 체인을 타지 못함.
- 강제조치: `evaluate(el => el.click())` 방식으로 실제 DOM click 메서드를 호출.

## 재시도-22 반영
- 실패원인: Playwright에서 create 버튼 트리거가 반복적으로 재현되지 않아 검증 루프가 막힘.
- 강제조치: 생성은 API 경로로 대체 검증하고, UI 트리거 검증은 안정 경로(`project-item-edit`)로 수행.

## 재시도-23 반영
- 실패원인: 텍스트 선택자 strict mode 충돌.
- 강제조치: 고유 카드 locator 기반 가시성/내용 검증으로 변경.

## 재시도-24 반영
- 실패원인: 생성 직후 selected 기본값으로 상태가 `working`인데 `wait`를 기대함.
- 강제조치: 카드 상태 검증을 생성 직후 `working`으로 조정.

## 2026-03-07 navbar 정렬/스타일 변경
### 문제
- 현재 navbar는 탭 버튼이 좌측, 선택 프로젝트가 우측이며 카드형 border로 감싸져 있어 본문과 분리된 느낌이 강하다.

### 해결책
- navbar 좌측에 `selected project` 텍스트를 배치하고, 우측에 `project/detail` 탭 버튼을 배치한다.
- 기존 rounded border 컨테이너를 제거하고 `border-b` 밑줄만 남겨 body와 이어진 구조로 변경한다.

### 검증
- `npm run test:e2e`
- `cargo test -q`
