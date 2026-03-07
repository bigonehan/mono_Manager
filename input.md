# ORC Web UI 요구사항 (TUI 기능 매핑)

## 목표
- `orc open-ui -w`로 웹 UI를 실행한다.
- 웹 UI는 TUI의 핵심 기능을 동일한 실행 경로로 제공한다.

## TUI 기능 추출 결과
- 프로젝트 관리
  - 프로젝트 목록 조회
  - 프로젝트 생성(`name/description/spec/path`)
  - 프로젝트 수정(`name/description/path`)
  - 프로젝트 삭제
- 상세/문서 편집
  - `project.md`의 `name/description/spec/goal` 표시
  - `rules`/`constraints` 편집
  - `drafts_list.yaml`의 `features` 편집
  - `planned/planned_items` 표시
  - 생성된 feature draft 디렉터리 표시
- 실행 액션
  - `create_code_draft` 실행
  - `add_code_draft` 실행(요청 입력 기반)
  - `impl_code_draft` 실행
  - `check_code_draft -a` 실행
  - `check_draft` 실행

## 웹 구현 요구
- assets 폴더 하위에 웹 소스를 분리해 관리한다.
- 기술 스택: Astro + Vite + shadcn 스타일 컴포넌트(React)
- API route를 통해 파일/CLI 액션을 수행한다.
- 최소 화면 구성
  - 좌측: 프로젝트 목록/생성/수정/삭제
  - 우측: 상세 정보 + rules/constraints/features/planned/generated + 실행 버튼
  - 하단: 실행 로그

## 검증
- Playwright E2E로 아래를 확인한다.
  - 홈 로드
  - 프로젝트 생성 후 목록에 반영
  - 프로젝트 선택 후 상세 패널 표시
  - 실행 버튼 호출 시 로그 업데이트
