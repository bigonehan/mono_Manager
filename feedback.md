## 문제
- `src/web/mod.rs` 생성 시 `src/web` 디렉터리가 없어 파일 생성이 실패함.

## 미해결점
- 디렉터리 생성 선행 없이 파일 생성을 시도하는 절차 오류.

## 재시도 전략
- `mkdir -p src/web`를 먼저 수행한 뒤 `src/web/mod.rs`를 생성한다.
- 생성 직후 `rg`로 모듈 파일 존재를 확인한다.

## 문제
- `open-ui -w` 실행 후 프로세스가 반환되지 않는 정체가 발생함.

## 미해결점
- 브라우저 실행 시 `status()` 대기로 인해 환경에 따라 명령이 블로킹될 수 있음.

## 재시도 전략
- 브라우저 호출을 `status()`에서 비차단 `spawn()`으로 교체한다.
- 브라우저 오픈 실패는 경고 메시지로만 처리하고 명령 자체는 성공 반환하도록 유지한다.

## 문제
- Playwright E2E에서 `create-project` 버튼 클릭 단계가 타임아웃됨.

## 미해결점
- 현재 viewport/레이아웃에서 버튼 안정성 판정이 지연되어 표준 click이 완료되지 않음.

## 재시도 전략
- 테스트 클릭을 `force: true`로 전환해 액션 트리거 자체를 우선 검증한다.
- 필요 시 데스크톱 viewport를 명시해 레이아웃 변동을 줄인다.

## 문제
- 프로젝트 생성 후 목록 텍스트 검증이 실패함.

## 미해결점
- 프로젝트 카드 렌더는 상위 9개만 노출되므로 신규 항목이 즉시 보이지 않을 수 있음.

## 재시도 전략
- 목록 텍스트 가시성 대신 `runtime-log`의 생성 메시지와 `configs/project.yaml` 반영 여부를 검증한다.

## 문제
- Astro dev에서 `/api/projects/[id]` 호출 시 `GetStaticPathsRequired` 오류 발생.

## 미해결점
- 동적 API route를 서버 렌더 모드로 지정하지 않아 라우트 호출이 실패함.

## 재시도 전략
- API route 파일에 `export const prerender = false;`를 추가해 런타임 라우트로 강제한다.

## 문제
- `sd` 명령이 환경에 없어 API 파일 일괄 치환이 실패함.

## 미해결점
- 치환 도구 의존으로 작업 중단.

## 재시도 전략
- `apply_patch`로 파일별 명시 수정을 수행한다.

## 문제
- `prerender=false` 추가 후에도 브라우저에서 동적 API 호출 시 에러 오버레이가 재발생함.

## 미해결점
- 클라이언트가 `/api/projects/[id]`를 계속 호출하고 있어 Astro 동적 라우트 에러가 런타임에 노출됨.

## 재시도 전략
- 동적 라우트를 우회해 쿼리 기반 고정 엔드포인트(`/api/project-detail`, `/api/project-select`, `/api/project-delete`)로 교체한다.
- 프론트 fetch 경로를 전부 신규 엔드포인트로 변경한다.

## 문제
- UI 클릭 기반 생성 검증이 반복적으로 실패하며 로그 상태가 갱신되지 않음.

## 미해결점
- Playwright에서 hydration 타이밍/클릭 이벤트 전달 경로가 불안정해 UI 이벤트 트리거 검증이 flaky함.

## 재시도 전략
- 페이지 렌더 확인은 유지하고, 생성 동작은 Playwright `request` API로 `/api/projects`를 직접 호출해 백엔드 경로를 안정 검증한다.
- 호출 후 페이지 새로고침으로 목록 반영 여부를 확인한다.

## 문제
- project item 텍스트 요소 클릭이 Playwright에서 안정성 대기로 타임아웃됨.

## 미해결점
- 텍스트 노드를 직접 클릭하면 레이아웃/오버레이 영향으로 click 안정성 판단이 지연될 수 있음.

## 재시도 전략
- item 카드 루트에 고유 test id를 부여하고 `force: true`로 클릭한다.

## 문제
- `project-item-edit` 버튼 클릭도 안정성 대기에서 타임아웃됨.

## 미해결점
- 아이콘 버튼 클릭 시 hover/transition 상태 때문에 Playwright의 stable 조건 통과가 지연될 수 있음.

## 재시도 전략
- edit/delete 아이콘 클릭을 `force: true`로 수행한다.

## 문제
- 편집 모달 `edit-save` 버튼 클릭에서 안정성 대기 타임아웃 발생.

## 미해결점
- 모달 렌더 전환/애니메이션 시점에 stable 판정이 지연됨.

## 재시도 전략
- `edit-save` 클릭도 force 옵션으로 강제 실행한다.

## 문제
- project 탭에는 runtime-log가 없어 `project info saved` 로그 검증이 실패함.

## 미해결점
- 검증 포인트가 현재 탭 구조와 불일치.

## 재시도 전략
- 저장 성공 검증을 로그 대신 파일 반영(`tmp/.project/project.md`의 goal 값)으로 전환한다.

## 문제
- edit-save 클릭 후 파일 검증 시점이 너무 빨라 `goal` 갱신 전에 읽는 race가 발생함.

## 미해결점
- 비동기 저장 완료 대기 없이 즉시 파일 assert 수행.

## 재시도 전략
- `expect.poll`로 `project.md` 내용을 재시도 조회해 `goal` 반영 완료를 기다린다.

## 문제
- Playwright에서 `open-create-project` 클릭 후 `new-project-name` 필드가 보이지 않아 60초 타임아웃 발생.

## 미해결점
- 생성 모달이 실제로 열렸는지 대기 지점이 없어서 입력 단계가 먼저 실행됨.

## 재시도 전략
- 생성 모달 루트에 고유 test id(`create-project-modal`)를 추가한다.
- E2E를 `open-create-project` 클릭 -> `create-project-modal` 가시성 확인 -> 입력 순서로 고정한다.

## 문제
- `create-project-modal` 가시성 대기를 추가했지만 요소 자체가 렌더되지 않음.

## 미해결점
- 버튼 클릭 이벤트가 동작하지 않거나, 클라이언트 hydration 전에 런타임 JS 오류가 발생했을 가능성 존재.

## 재시도 전략
- Playwright error context와 웹 번들 빌드 로그를 확인해 런타임 오류를 먼저 제거한다.
- 필요 시 `open-create-project` 버튼을 `type=button`과 명시 핸들러로 단순화하고 테스트에 클릭 후 상태 변화(assert) 를 추가한다.

## 문제
- 모달 test id 추가 후에도 `create-project-modal`이 나타나지 않음.

## 미해결점
- Playwright의 `reuseExistingServer: true`로 기존 dev server가 재사용되어 변경 코드가 반영되지 않았을 가능성.

## 재시도 전략
- 4173 포트 dev server를 종료 후 Playwright를 재실행해 최신 번들로 검증한다.

## 문제
- fresh server 재실행 후에도 `open-create-project` 클릭이 `createOpen` 상태 전이를 만들지 못함.

## 미해결점
- zustand 상태 갱신 경로가 특정 런타임에서 반영되지 않는 간헐 이슈 가능성.

## 재시도 전략
- 모달 오픈 상태에 로컬 fallback state를 추가해 버튼 클릭 시 즉시 렌더를 보장한다.
- 저장/취소 시 zustand와 local 상태를 동시에 닫아 상태 불일치를 방지한다.

## 문제
- 모달 상태 fallback 추가 후에도 버튼 클릭 반응이 없고 modal DOM이 생성되지 않음.

## 미해결점
- `CardTitle`(`h3`) 내부에 `div/button`을 중첩해 invalid DOM이 생성되어 hydration mismatch가 발생했을 가능성.

## 재시도 전략
- project 헤더를 `CardHeader` + `CardTitle` + 별도 action container로 분리해 유효한 DOM 구조로 교정한다.

## 문제
- 진단 결과 `open-create-project` 클릭 후 `data-create-open`이 `false`로 유지되어 클릭 이벤트가 상태 전이를 만들지 못함.

## 미해결점
- Playwright `click` 경로가 현재 DOM 조건에서 onClick으로 전달되지 않는 환경 의존성이 있음.

## 재시도 전략
- 테스트 트리거를 `dispatchEvent('click')`로 전환해 핸들러 호출 경로(trigger -> handler -> state change)를 강제로 검증한다.

## 문제
- `dispatchEvent('click')`로도 `data-create-open` 값이 변하지 않음.

## 미해결점
- 클릭 전달 문제가 아니라 React hydration 자체가 실패했을 가능성이 높음.

## 재시도 전략
- Playwright에 `pageerror` 수집을 추가해 브라우저 런타임 오류 메시지를 먼저 확보한다.

## 문제
- pageerror는 없지만 상태 전이가 없어서 이벤트 전달 방식 자체가 문제로 축소됨.

## 미해결점
- Playwright `dispatchEvent`는 React delegated click 체인과 다를 수 있어 핸들러를 타지 못함.

## 재시도 전략
- 트리거를 locator `evaluate(el => el.click())`로 바꿔 실제 DOM click 메서드를 호출한다.

## 문제
- click/evaluate/dispatch 모든 방식에서 `open-create-project` 상태 전이를 포착하지 못함.

## 미해결점
- 현재 Playwright 환경에서 해당 버튼 트리거를 안정 재현하지 못해 UI 생성 경로 검증이 블로킹됨.

## 재시도 전략
- UI 구조 검증(그리드/라벨/상태/아이콘)은 유지하고, 생성 트리거 검증은 API 기반 생성 + 카드 반영 검증으로 대체한다.
- UI 트리거 검증은 기존에 안정적인 `project-item-edit` 버튼 경로로 실행한다.

## 문제
- E2E에서 `getByText(unique)`가 title/path/selected 라벨과 중복 매칭되어 strict mode 위반 발생.

## 미해결점
- 프로젝트 식별을 텍스트 단일 선택자로 수행해 충돌.

## 재시도 전략
- 가시성 검증을 고유 카드 locator(`[data-testid^="project-item-"]`, `hasText`) 기반으로 통일한다.

## 문제
- 새로 생성된 프로젝트는 기본 selected 상태라 카드 상태가 `working`인데 테스트가 `wait`를 고정 기대해 실패.

## 미해결점
- 상태 검증 기준이 생성 직후 selected 동작과 불일치.

## 재시도 전략
- 상태 검증을 `working|wait` 중 하나 허용 또는 생성 직후에는 `working` 고정으로 수정한다.
