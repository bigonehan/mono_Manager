## 2026-02-19 - 작업한일
- Rust 기반 HTTP+JSON 콜백 서버/병렬 워커(codex exec) 오케스트레이터 구현
- run-parallel(n, msgs[]) 명령과 client 결과 전송 함수 추가

## 2026-02-19 - 작업한일
- 병렬 codex exec 통합 테스트 추가: 각 워커 5초 대기 후 apple 출력, HTTP JSON 콜백 전송 검증

## 2026-02-19 - 작업한일
- main 기본 실행에서 서버와 병렬 워커를 동시에 수행하도록 오케스트레이션 경로 추가
- request_messages 기본값 4개(숫자이름/한국 관광도시/현재 일본 시각/내일 서울 행사) 반영

## 2026-02-19 - 작업한일
- run-test 명령 추가: 서버 실행 + 기본 request_messages 병렬 전송 + 수신 결과 출력
- request_messages 추가 함수(flow_add_request_message)와 병렬 전달 함수(stage_send_request_messages_parallel) 분리

## 2026-02-19 - 작업한일
- codex exec stdout을 서버 수신 로그/최종 요약 출력(response)으로 표시하도록 보강

## 2026-02-19 - 작업한일
- codex exec 인자 오류 수정(--prompt 제거, positional prompt 사용)
- 응답 로그에 stderr/exit_code 출력 추가로 codex 실행 실패 원인 가시화

## 2026-02-19 - 작업한일
- 메시지 전송+응답수집 함수(stage_send_message_and_receive_response)와 전송만 후 종료 함수(stage_send_message_only_and_exit) 분리
- run-parallel/run-test에 --send-only 모드 추가

## 2026-02-19 - 작업한일
- send-only 모드도 codex exec를 실제 실행 후 종료 신호(codex_finished, exit_code) 전송하도록 수정
- codex 실행 공통 함수(stage_execute_codex) 도입 및 send-only 종료 신호 테스트 추가

## 2026-02-19 - 작업한일
- 생성 함수 목록을 functions.txt에 기능설명 : 함수 이름 형식으로 기록

## 2026-02-19 - 작업한일
- template/prompt/run_task_with_msg_postpix.txt 추가 및 stage_send_message_and_receive_response에서 postpix 프롬프트 결합 적용
- postpix 로드/결합 함수 추가(flow_load_run_task_with_msg_postpix_prompt, flow_build_prompt_with_postpix) 및 functions.txt 갱신

## 2026-02-19 - 작업한일
- postpix 템플릿을 3줄 고정 포맷(SUMMARY/RESULT/REPORT)으로 강화
- codex exec에서 --output-last-message(-o) 파일을 사용해 최종 메시지만 수집하도록 수정

## 2026-02-19 - 작업한일
- codex가 네트워크 호출을 수행하지 않도록 prompt/postpix 지시를 수정하고 서버 전송은 worker 함수(stage_send_worker_result_to_server)로 일원화
- codex 응답에서 SUMMARY/RESULT/REPORT만 추출(flow_extract_postpix_lines), 성공 로그에서 stderr 숨김 처리

## 2026-02-19 - 작업한일
- ratatui/crossterm 기반 working pane UI 추가(src/compoents): 요청 기능/결과값/상태 3컬럼 실시간 표시
- index % 2 행 배경색 교차 적용, 상태 이모지(준비/진행중/완료) 적용, run-test에서 UI 이벤트 연동

## 2026-02-19 - 작업한일
- functions.yaml 추가: file 객체 아래 items(name, description) 리스트 구조로 함수 목록 정리

## 2026-02-19 - 작업한일
- template/function.yaml 스키마 템플릿 추가(name/description 키 기반 인식 규칙 포함)

## 2026-02-19 - 작업한일
- functions.yaml/items 포맷을 "name, description" 단일 문자열 리스트로 전환
- template/function.yaml 파싱 규칙(첫 번째 콤마 분리)으로 스키마 갱신

## 2026-02-19 - 작업한일
- functions.yaml/items 포맷을 "description : name" 문자열 리스트로 전환
- template/function.yaml 파싱 규칙을 첫 번째 콜론(:) 분리 기준으로 갱신

## 2026-02-19 - 작업한일
- template/function.yaml 하단 샘플을 주석 블록으로 전환하고 "출력 예시" 라벨 추가

## 2026-02-19 - 작업한일
- postpix 프롬프트 로더를 경로 탐색+환경변수(ORCHESTRA_POSTPIX_PROMPT_PATH)+내장 기본값 fallback 구조로 보강
- 실행 위치가 달라도 run_task_with_msg_postpix.txt 미발견 에러로 중단되지 않도록 수정

## 2026-02-19 - 작업한일
- template/ 디렉터리를 include_dir로 바이너리에 내장하고 postpix 로더가 내장 템플릿을 기본 사용하도록 변경

## 2026-02-19 - 작업한일
- configs/style.yaml 생성: basic.primary=#121010, secondary=#695656, background=#FAE3DE
- AGENTS.md 생성: UI 작업 시 색상은 configs/style.yaml 참조 규칙 추가

## 2026-02-19 - 작업한일
- configs/style.yaml에 layout(margin/padding/pane_width_percent) 추가
- working pane이 style.yaml을 읽어 색상/패딩/마진을 적용하고 가로 50% 영역만 차지하도록 변경

## 2026-02-19 - 작업한일
- working pane 상태 표시를 enum 기반 문자열(⬤ 준비/⬤ 진행/⬤ 완료)로 변경
- 배경색 적용 행의 텍스트를 흰색(Color::White)으로 고정

## 2026-02-19 - 작업한일
- working pane 상태 표시를 텍스트 없이 심볼만 사용(준비=⯈, 진행=⯀, 완료=⬤)하도록 변경

## 2026-02-19 - 작업한일
- 입력 질문 함수 모듈(src/input/question.rs) 추가: question 문자열 입력, y/n/숫자 검증, auto/time(기본 false/0) 처리
- auto=true 시 입력 대기 없이 yes/1/나비 자동 응답, time은 0 또는 1~60 범위 검증

## 2026-02-19 - 작업한일
- configs/style.yaml에 symbol.state(ready/running/done) 추가
- working pane 상태 심볼을 코드 하드코딩 대신 style.yaml(symbol.state) 로드값으로 표시하도록 변경

## 2026-02-19 - 작업한일
- working pane 전체 배경 스타일 제거
- 리스트 행은 index % 2 조건으로만 background 적용(해당 행 흰색 글자), 기본 행은 primary 글자색 적용

## 2026-02-19 - 작업한일
- configs/style.yaml background 색상 값을 #E6E6E6으로 변경

## 2026-02-19 - 작업한일
- working pane의 모든 텍스트 색상을 primary로 통일(교차 배경 유지)

## 2026-02-19 - 작업한일
- working pane 상태 컬럼 폭 축소(12%) 및 상태 심볼 오른쪽 정렬 적용

## 2026-02-19 - 작업한일
- working pane 결과 컬럼은 SUMMARY/RESULT 문구 대신 값만 표시하도록 파싱 로직 추가(flow_extract_result_value_for_ui)
- REPORT answer 값을 우선 사용하고 없으면 RESULT 값을 사용하는 규칙 적용

## 2026-02-19 - 작업한일
- template/ 디렉터리를 assets/로 재구성: assets/templates/function.yaml, assets/prompts/run_task_with_msg_postpix.txt
- 코드 경로 및 내장 리소스(include_dir) 경로를 assets 기준으로 갱신

## 2026-02-19 - 작업한일
- 병렬 워커 실행 바이너리를 config 기반으로 전환: CLI(--codex-bin) 미지정 시 configs/app.yaml의 ai 사용, 최종 fallback은 codex
- configs/app.yaml 추가(ai: codex), 관련 해석 함수(flow_resolve_ai_bin, flow_load_ai_bin_from_config) 구현

## 2026-02-19 - 작업한일
- 병렬 codex exec 호출에 --dangerously-bypass-approvals-and-sandbox 옵션 추가(대화형 승인 자동 처리)

## 2026-02-19 - 작업한일
- configs/app.yaml의 ai를 model/auto 구조로 전환하고(auto bool), auto=true일 때 codex exec에 승인 자동처리 인수 추가
- AI 실행 옵션 해석을 flow_resolve_ai_options로 재구성(CLI model > config model > codex)

## 2026-02-19 - 작업한일
- assets/templates/todos.yaml 및 assets/prompts/run_todos.txt 추가
- 병렬 codex 프롬프트 생성 시 run_todos.txt에 todos.yaml 템플릿 내용을 삽입해 함께 전달하도록 구현

## 2026-02-19 - 작업한일
- AGENTS.md에 YAML 처리 규칙 추가(문서/예시 주석 처리, 실행용 YAML 순수 데이터 유지, 템플릿/실행 파일 분리)

## 2026-02-19 - 작업한일
- run_todos 전용 파싱 함수(flow_parse_todos_prompt_template) 추가: {{...}} 플레이스홀더를 todos.yaml 본문으로 치환

## 2026-02-19 - 작업한일
- run_todos 플레이스홀더를 긴 문구에서 {{body}}로 단축
- 프롬프트 파일/상수/테스트 치환 기준을 동일하게 갱신

## 2026-02-19 - 작업한일
- 루트 todos.yaml 생성: 테스트용 todo 5개 항목(name/type/scope/rule/step) 추가

## 2026-02-19 - 작업한일
- AGENTS.md에 함수 네이밍 기준 추가(flow_는 오케스트레이션 전용, 메시지 동작은 send_ 우선)

## 2026-02-19 - 작업한일
- send_add_request_message를 add_request_message로 리네이밍하고 참조/문서 목록 갱신

## 2026-02-19 - 작업한일
- currentProject=test 기준으로 run-test가 ./project/test/tasks.yaml blueprint를 읽어 run_tasks_parallel로 항목별 병렬 처리하도록 연결
- read_blueprint/project/test/tasks.yaml 경로 구조 반영 및 tasks.yaml 키 일치(tasks:)로 정리

## 2026-02-19 - 작업한일
- run_todos {{body}} 치환 대상을 todos 템플릿 전체가 아닌 현재 병렬 task item 1개로 변경
- flow_build_prompt_with_postpix에 task item 본문 전달 인수 추가 및 관련 테스트 갱신

## 2026-02-19 - 작업한일
- 로컬/전역 AGENTS 및 AGENTS.override 분석으로 flow_ 접두사 확산 원인 확인
- /home/tree/ai/codex/AGENTS.override.md Function Naming Rule을 오케스트레이션 전용으로 명확화
- 프로젝트 /home/tree/project/orchestra/AGENTS.md 네이밍 규칙을 강화해 flow_ 사용 범위를 명시

## 2026-02-19 - 작업한일
- 외부 실행용 바이너리를 `orc`로 고정(Cargo autobins 비활성화 + bin 엔트리 설정)
- `run-test`에서 `project/test/tasks.yaml` 우선, 없으면 `project/test/tasks.ymal`까지 자동 탐색하도록 보강

## 2026-02-19 - 작업한일
- configs/app.yaml에 project.path 설정 추가(기본값 ".")
- blueprint 경로 해석을 resolve_project_base_path/resolve_project_dir/resolve_blueprint_file_path 공통 함수로 통일
- tasks.yaml 조회 기준을 project.path 기반으로 변경(기존 ./project/<name> 하위 유지)

## 2026-02-19 - 작업한일
- test용 blueprint 파일 project/test/tasks.yaml을 사용자 지정 항목으로 교체

## 2026-02-19 - 작업한일
- run-test UI를 menu_function으로 래핑하고 작업 목록 pane + pane_task_spec( tasks.yaml ) 2패널 구조로 확장
- 좌/우 화살표로 pane 포커스 전환 기능 추가
- pane_task_spec 포커스에서 Enter로 편집 모드 진입, 입력/백스페이스/엔터 반영 시 실제 tasks.yaml 파일에 즉시 저장되도록 구현(Esc로 저장 후 편집 종료)

## 2026-02-19 - 작업한일
- 2패널 UI 활성/비활성 대비를 강화: 활성 pane 제목/테두리에 강조 배경색 적용, 비활성 pane은 단색 테두리로 구분
- working 영역 레이아웃을 중앙 고정 폭에서 전체 가로폭(작은 margin만 유지) 사용으로 변경
- pane_width_percent 미사용 필드 정리

## 2026-02-19 - 작업한일
- pane_task_spec를 원문 텍스트 편집에서 task 카드 리스트 UI로 전환
- 카드 선택(Up/Down) 후 Enter로 Form 모드 진입, name/type/scope/rule/step 필드 단위 편집 지원
- Form 모드에서 Enter로 필드 편집 시작/완료, Esc로 리스트 복귀, 저장 시 실제 tasks.yaml에 즉시 반영

## 2026-02-19 - 작업한일
- run-test blueprint 기본 파일명을 `tasks.yaml`에서 `todos.yaml`로 전환
- 탐색 순서를 `todos.yaml` 우선으로 변경하고 기존 `tasks.yaml/tasks.ymal`은 fallback 호환 유지
- test blueprint 파일을 `project/test/todos.yaml`로 리네임

## 2026-02-19 - 작업한일
- run-test 자동 실행을 제거하고 working pane 포커스 상태에서 Enter 입력 시에만 run_tasks_parallel이 시작되도록 제어 채널 추가
- UI(menu_function -> stage_run_working_pane)에서 run start 신호를 main으로 전달하도록 시그니처/흐름 갱신

## 2026-02-19 - 작업한일
- assets/templates/spec.yaml 문법 오류 수정(feature:[]/tasks[]/depends on 등)
- tasks/features 구조를 유효한 YAML 키/리스트/들여쓰기로 정규화
- 템플릿 설명은 주석으로 유지하고 실제 파싱 가능한 기본값("", [])으로 정리

## 2026-02-19 - 작업한일
- run-test 시작 시 request 입력 모달 pane 추가(멀티라인 텍스트 입력 + 취소/확인 버튼)
- 사용자 입력 처리 함수 `set_request_function` 구현(Tab/Enter/Esc/방향키)
- 확인 시 입력 라인을 메시지 목록으로 전송하고, run-test 실행 전 해당 입력이 add-msg로 반영되도록 메인 흐름 연동

## 2026-02-19 - 작업한일
- `set_requset_function` 동작을 요구사항 기준으로 재구성: 입력창 표시 + 확인 버튼 대기 + 확인 시 파싱 함수 호출
- `parsing_request_function` 추가: `#`->name(필수), `>`->step(선택), `-`->rule(선택) 규칙으로 멀티 아이템 입력을 tasks 배열로 파싱
- 확인 시 파싱된 tasks를 pane spec에 반영하고 todos.yaml에 즉시 저장, 작업 목록 pane도 새 task 개수에 맞춰 갱신

## 2026-02-19 - 작업한일
- 왼쪽 영역을 세로 2분할로 확장: 상단 `project_spec`, 하단 `pane_task_spec`
- `project_spec`에 name/framework/rule/domain/feature 표시 추가
- task spec YAML 구조를 project 메타 필드(name/framework/rule/features) + tasks 형태로 확장하고 기존 `todos` 키는 alias 호환 유지

## 2026-02-19 - 작업한일
- UI 전역 키 처리에 `q/Q` 종료 단축키 추가
- run-test 대기 구간에서 시작 신호 없이 UI가 종료되면 에러 대신 정상 종료하도록 보정

## 2026-02-19 - 작업한일
- request 입력 확인 시 tasks 전체 교체 대신 파싱된 item을 기존 tasks 뒤에 append하도록 수정
- 저장 후 상태 메시지에 append된 item 수를 표시하도록 개선

## 2026-02-19 - 작업한일
- request 입력 모달의 취소/확인 버튼 상태 표현을 배경색 + 흰색 글자로 변경
- 비선택 버튼은 기본 글자색 유지로 대비 강화

## 2026-02-19 - 작업한일
- pane_task_spec 리스트 렌더를 다중 카드에서 단일 선택 객체 뷰로 변경
- Up/Down 선택 인덱스의 name을 기준으로 todos 전체에서 일치 task를 조회해 표시하도록 구현
- 표시 포맷을 todos.yaml 객체 형태(name/type/scope/rule/step)로 맞춤

## 2026-02-19 - 작업한일
- Prompt_Todos에 Codex 실행 지시어 추가: functional-code-structure 스킬 적용, todo 작성 규칙(rule/step 반영, depends_on 명시), todos.yaml append 원칙 반영

## 2026-02-19 - 작업한일
- 오른쪽 영역을 세로 2분할로 변경: 상단 `todos` pane, 하단 축약 `working` pane
- `todos` pane은 선택 item(name 매칭)의 todo 객체를 표시하도록 구성(이전 패턴 참고)
- `working` pane은 목록 대신 상태 요약(ready/running/done) 중심으로 축약 렌더

## 2026-02-19 - 작업한일
- `p` 키(make_todos_spec) 실행을 백그라운드 job으로 전환하고 진행 상태/오류를 표시하는 소형 팝업 pane 추가
- 진행 단계(resolving spec/read/build prompt/run codex/parse) 실시간 표시 및 실패 시 에러 메시지 표시
- 완료 시 append 결과를 todos에 반영하고 팝업에서 Esc/Enter로 닫을 수 있도록 처리

## 2026-02-19 - 작업한일
- YAML 조회 경로를 CWD 기준 `./.project/<project>/`로 통일
- run-test 시작 시 `.project/<project>` 자동 보정 로직 추가(legacy `project/<project>`의 todos/spec 마이그레이션)
- make_todos_spec의 spec.yaml 탐색 fallback을 제거하고 `.project` 인접 경로만 사용하도록 고정
- `.project/test/todos.yaml`, `.project/test/spec.yaml` 초기 파일 생성

## 2026-02-19 - 작업한일
- CWD가 `.project/<project>` 또는 `.project`일 때 base path를 workspace root로 정규화하는 로직 추가
- `.project/test/.project/test/...`처럼 경로가 중복 붙는 버그 수정

## 2026-02-19 - 작업한일
- `get_root_dir`/`find_git_root_dir` 추가: 기본은 CWD, 상위에 `.git`이 있으면 git root를 루트로 사용
- 실행 시작 시 git 저장소가 없으면 기존 `input_ask_question`(YesNo)으로 `jj git init --colocate` 실행 여부를 질문하도록 구현
- project base 경로 해석을 config.path 의존 대신 루트 디렉터리 기반으로 단순화

## 2026-02-19 - 작업한일
- blueprint yaml 파일을 찾지 못하면 `root/.project/<project>/todos.yaml`을 자동 생성하도록 변경
- 초기 파일은 실행 가능한 빈 템플릿(`tasks: []`)으로 생성

## 2026-02-19 - 작업한일
- pane_task_spec를 단일 상세뷰 대신 카드 리스트로 렌더링하도록 유지/보강
- 선택 인덱스 기준 가시 범위를 계산해 여러 카드가 한 번에 보이도록 하고, 상/하 스크롤 힌트(^ more, v more)와 visible 범위를 표시

## 2026-02-19 - 작업한일
- task -> todos 생성 프롬프트에 scope 비어있을 때 파일 경로를 LLM이 추론해 scope를 채우는 규칙 추가
- 추론 scope를 프로젝트 루트 기준 상대 경로로 작성하도록 규칙 추가

## 2026-02-19 - 작업한일
- git 루트 탐색을 "가장 바깥"이 아닌 "현재 경로 기준 가장 가까운" 루트 반환으로 수정
- 중첩 git 디렉터리 환경에서 가까운 루트를 선택하는 단위 테스트 추가

## 2026-02-19 - 작업한일
- git 루트 탐색 상한을 /home/tree로 설정해 /home/tree 자체는 루트 후보에서 제외
- /home/tree/.git가 있어도 반환되지 않도록 경계 테스트 추가

## 2026-02-19 - 작업한일
- pane_task_spec 타이틀을 task로 변경
- task pane에서 Enter 입력 시 set_request_function(요청 입력 팝업) 호출로 변경
- 기존 task 수정(form) 진입은 E 키로 유지, P 키는 make_todos_spec 실행 유지

## 2026-02-19 - 작업한일
- task pane에 pane/item 선택 상태 분리(TaskListFocus) 추가
- Task pane 상태에서 Enter는 task 입력 모드(set_request_function) 열기, item 활성 상태 Enter는 item 수정(form) 진입으로 키 동작 분리
- Down으로 item 활성화/순회, Up 최상단에서 pane 상태 복귀 동작 추가

## 2026-02-19 - 작업한일
- set_request_function 입력창에 세로 스크롤 적용(Paragraph.scroll)
- 입력창에서 PgUp/PgDn으로 수동 스크롤, 입력/삭제 시 자동 하단 추적 동작 추가

## 2026-02-19 - 작업한일
- run-test UI의 task pane 저장 경로를 todos.yaml이 아닌 spec.yaml로 고정
- task 입력/수정 결과가 spec.yaml에 반영되도록 경로 연결 수정

## 2026-02-19 - 작업한일
- task pane의 YAML 파싱에서 todos alias 제거
- task pane이 spec.yaml의 tasks 항목만 데이터 소스로 사용하도록 고정

## 2026-02-19 - 작업한일
- task pane 데이터 소스를 spec.yaml(tasks)로 고정하고 todos pane은 todos.yaml(tasks/todos) 전용 로드로 분리
- P(make_todos_spec) 실행 결과를 spec.yaml이 아닌 todos.yaml에 append/save 하도록 수정
- todos pane 매칭 로직을 spec 선택 task 이름 기준으로 todos.yaml item을 찾도록 조정

## 2026-02-19 - 작업한일
- P(make_todos_spec) 실행 시 spec.yaml의 tasks item을 순회하면서 task 단위로 LLM 호출하도록 변경
- 각 task별 생성 결과를 누적해 todos.yaml에 append 저장하도록 동작 보정

## 2026-02-19 - 작업한일
- P 실행 시 spec.yaml tasks item별 LLM 호출을 병렬 처리로 전환
- 각 item 결과를 임시 YAML 파일로 저장 후 전체 완료 시 병합하여 todos.yaml에 append
- 병합 완료/실패 시 임시파일 정리(cleanup) 추가로 중복 쓰기 및 잔여 임시파일 방지

## 2026-02-19 - 작업한일
- todos 생성 프롬프트에 step 설계 사고 절차(도메인 최소단위/상태변화/조건식/변수/단일작업 변수변화) 1~5단계 추가
- 코드 내 make_todos 프롬프트와 assets/prompts/Prompt_Todos.txt를 동일 규칙으로 동기화

## 2026-02-19 - 작업한일
- todos 생성 프롬프트에 구체화 규칙 추가(검증/대기/직렬화/저장/전송/후처리 단계)
- rule/step을 상태 전이 관점으로 세분화하도록 지시 추가(선택->검증->입력대기->분기->저장/전송->완료)
- 추상 scope를 실제 파일 후보로 구체화하도록 규칙 강화

## 2026-02-19 - 작업한일
- P 실행 파이프라인에 spec tasks 보강 단계 추가(type/scope/depends_on)
- 보강 단계는 codex 단일 호출로 전체 task를 처리하고 spec.yaml에 저장 후 todos 생성 단계로 진행
- scope 채움 규칙(domain 후보 우선, 미적합 시 utilts/<snake_case>.ts), depends_on 의존관계 추론 규칙 프롬프트 추가
- TaskSpecItem에 depends_on 필드 추가 및 todos 생성 단계 연동

## 2026-02-19 - 작업한일
- todos.yaml 실행 템플릿을 순수 YAML(`tasks: []`)로 정규화하고, todos.yaml 미존재/빈파일 시 템플릿으로 초기화하도록 수정
- make_todos 프롬프트에 todos 템플릿 본문과 키 스키마(name/type/depends_on/scope/state/rule/step) 명시
- 생성 YAML 파서를 tasks/todos 둘 다 수용하도록 보강하고 TaskSpecItem에 state 필드 추가
- 병렬 LLM 처리 완료 후 임시파일 개수/존재 검증 추가, 병합 저장 전 YAML 문법 검증 추가

## 2026-02-19 - 작업한일
- todos 프롬프트에 스키마별 역할 정의(name/type/depends_on/scope/state/rule/step) 추가
- 템플릿 주석 제거로 줄어든 의미를 코드 프롬프트와 Prompt_Todos 양쪽에서 역할 설명으로 보강

## 2026-02-19 - 작업한일
- make_todo_spec 성공 시 진행 팝업을 자동 닫도록 수정해 완료 후 정지처럼 보이던 상태 개선
- 병렬 codex 호출 출력 파일명 충돌 방지를 위해 output 경로에 원자 시퀀스+thread id+nanos를 포함하도록 보강

## 2026-02-19 - 작업한일
- 포커스 체인을 task->todos->working으로 확장하고, todos에서 Down으로 working 선택/working 활성 시 영역 확대(todos 축소) 적용
- working pane 내용을 todos item name 리스트 표시로 변경, working 활성 상태에서 P 키로 병렬 실행 트리거하도록 변경
- spec.features.domain을 리스트로 전환(문자열/리스트 호환 파싱), todos item에 domain 필드 추가
- make_todos 프롬프트에 domain 후보(spec.features.domain) 전달 및 domain 필드 작성 규칙/역할 정의 추가
- enforce-spec 단계에 templates/Prompt_domain.txt(domain_create skill 포함) 연동 및 domain/scope/depends_on 보강 규칙 강화
- 병렬 작업 완료 후 codex 후속 점검(리팩토링 평가) 실행 후 spec.features.feature에 도메인 기반 기능 문자열 append 로직 추가
- todos/spec 템플릿 스키마 업데이트(spec.features 구조화, task domain 속성 반영)

## 2026-02-19 - 작업한일
- CLI 명령 추가: build-spec(대화형 spec 생성), fill-spec-from-input(input.txt -> spec.tasks), check-last(codex 점검 + jj refactor change)
- project/task/todos/working 포커스 체인 및 하단 검은색 단축키 바 추가, working 활성 시 영역 확대/목록 표시 적용
- project pane 활성 상태에서 a 키로 auto 모드(make_todos -> 완료 후 run 자동 시작) 추가
- todos item에 domain 속성 추가 및 prompts에 domain 역할/선택 규칙(spec.features.domain 후보 전달) 반영
- enforce-spec 프롬프트를 templates/Prompt_domain.txt 우선 로드로 변경하고 domain_create skill 지시를 템플릿에 반영
- 병렬 완료 후 codex로 전체 코드 점검/리팩토링 평가 후 spec.features.feature 자동 append 단계 추가

## 2026-02-19 - 작업한일
- CLI 명령 체계를 요청 명칭으로 정리(show-ui, make-spec, fill-spec, make-todos, run-paralles)
- run-test는 show-ui로 제공하고 alias로 기존 run-test도 허용
- assets/templates/todos.yaml에 속성 설명 주석을 복구(실행 데이터는 tasks: [] 유지)
- Prompt_domain.txt를 assets/prompts/Prompt_domain.txt로 이동하고 참조 경로를 전부 갱신
- make-todos CLI 추가: spec.yaml 기반 Codex 생성 -> todos.yaml append 저장

## 2026-02-19 - 작업한일
- README.md 신규 작성: 프로젝트 개요, 빌드 방법, 주요 CLI 명령(show-ui/make-spec/fill-spec/make-todos/run-paralles/check-last) 안내 추가
