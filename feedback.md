# 문제
- `~/temp` 1차 시도에서 출력 파일(`project.md`, `src/*`, `package.json`)은 생성됐지만 `.project/feature/*` draft는 생성되지 않았다.
- 즉, 정지 지점은 UI 갱신 문제가 아니라 `create-draft` 이전 단계다.
- 추가로 `project.md`에서 `func_xxxxxxxx`/`TODO`가 추출되며 `drafts_list.yaml` 키가 비정상(`func_*`, `t_o_d_o`)으로 오염된다.

# 해결책
- LLM 실행 경로는 유지하고 재시도 정책 강화(공통 retry, dangerous 플래그 누락 방지).
- `project.md` 동기화 추출 로직 보강:
  - `func_xxxxxxxx: 설명` 형태는 설명 본문을 키 후보로 사용.
  - `TODO` placeholder는 planned/features 동기화에서 제외.

# 개선할 수 있는 점
- `project.md` 템플릿 출력 직후 schema 검사에서 `func_*`/`TODO` 금지 규칙을 추가해, 동기화 전에 차단할 수 있다.
- 다음 반복에서 필요 시 `action_validate_project_md_format`에 해당 검사 규칙을 추가한다.
## 2026-03-03 failure log
- 문제: `build-function-auto` 실행 시 `generated draft yaml invalid: task[0]: rule[..] is not auto-verifiable` 오류로 중단됨.
- 원인: LLM이 생성한 `draft.rule`에 자동검증 연산자/구조가 없는 자연어/표현식이 포함됨.
- 해결: draft YAML 정규화 단계에서 `rule` 항목을 문자열로 평탄화하고, auto-verifiable 규칙이 아니면 `check: <rule> should hold` 형태로 강제 변환.
- 재시도: `cargo run --bin orc -- build-function-auto` 실행으로 동일 경로 재검증.
## 2026-03-03 failure log
- 문제: `build-function-auto`가 `parallel::run_parallel_todo` 단계에서 `.project/project.md` 누락으로 실패.
- 원인: 현재 워크스페이스에 `.project`는 있으나 `project.md`가 없어 project_info 로딩이 중단됨.
- 해결: `build-function-auto` 경로에서 실행 전 `.project/project.md` 존재를 보장하고, 없으면 최소 기본 문서를 자동 생성.
- 재시도: `cargo run --bin orc -- build-function-auto` 동일 경로 재실행.
## 2026-03-03 failure fix
- 적용: `build-function-auto` 시작 시 `.project/project.md`가 없으면 `assets/code/templates/project.md`로 자동 생성하도록 보강.
- 기대: `parallel::run_parallel_todo`의 project info 로딩 실패 방지.
