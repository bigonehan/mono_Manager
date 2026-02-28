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
