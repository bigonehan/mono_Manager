# info
name : tmux_split_verification_for_school_landing_page
description : tmux split verification for school landing page
spec : tmux, bash, html, css, javascript
path : /home/tree/project/rust-orc/.tmp_tmux_test2

# features
- workspace_bootstrap

# rules
- 모든 실행 검증은 tmux split 환경에서 재현 가능해야 한다.
- 스크립트는 bash 기준으로 동작해야 하며 실행 순서가 명확해야 한다.
- 결과물은 school landing page의 정적 웹 자산(html, css, javascript) 구조를 유지해야 한다.
- 검증 절차는 동일 입력에서 동일 결과를 내도록 결정적이어야 한다.

# constraints
- 작업 경로는 `/home/tree/project/rust-orc/.tmp_tmux_test2`로 고정한다.
- 기술 스택은 `tmux`, `bash`, `html`, `css`, `javascript` 범위를 벗어나지 않는다.
- 빌드 도구나 프레임워크를 추가 도입하지 않는다.
- 검증 단계에서 tmux 세션/패널 이름 충돌이 발생하지 않도록 고유 식별자를 사용한다.

# domains
## workspace_bootstrap
### states
- 초기 상태: 작업 디렉토리와 기본 파일 구조만 존재
- 준비 상태: tmux 세션과 split 패널 구성이 완료됨
- 실행 상태: bash 검증 스크립트가 패널에서 실행 중
- 완료 상태: landing page 출력 및 검증 로그 확인 완료
- 실패 상태: tmux 구성 오류, 스크립트 오류, 또는 출력 검증 실패 발생

### action
- 작업 디렉토리 존재 여부를 확인한다.
- tmux 세션을 생성하고 필요한 split 패널을 구성한다.
- html/css/javascript 파일을 생성 또는 갱신한다.
- bash 검증 명령을 각 패널에서 실행한다.
- 실행 로그와 최종 산출물을 점검해 성공/실패를 확정한다.

### rules
- 상태 전이는 `초기 -> 준비 -> 실행 -> 완료` 순서를 기본으로 한다.
- 실패 조건이 발생하면 즉시 `실패 상태`로 전이하고 원인 로그를 남긴다.
- 각 액션은 재실행 가능해야 하며 중복 실행 시 기존 상태를 안전하게 처리해야 한다.
- 도메인 규칙은 모두 `-` 리스트 형식으로 유지한다.

### constraints
- tmux 세션/패널 생성 명령은 비대화형으로 수행 가능해야 한다.
- 검증 명령은 bash 한정 문법으로 작성한다.
- 웹 자산은 정적 파일로 유지하고 런타임 서버 의존을 강제하지 않는다.
- 경로 참조는 프로젝트 루트 기준 상대/절대 경로를 혼용하지 않고 일관되게 사용한다.
