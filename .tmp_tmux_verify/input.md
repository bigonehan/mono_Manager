# workspace_bootstrap
- 모든 검증은 `/home/tree/project/rust-orc/.tmp_tmux_verify` 경로 기준으로 시작한다.
- `tmux`와 `bash` 가용성이 확인되지 않으면 후속 검증을 진행하지 않는다.
- 경로 불일치나 필수 도구 누락은 즉시 실패로 판정한다.
- `./.project/project.md` 단일 문서 기준 정의를 유지하고 `./project/project.md`는 참조하지 않는다.
> 작업 경로 일치 여부를 확인한다.
> `tmux`, `bash` 설치 및 실행 가능 여부를 점검한다.
> 초기화 상태를 `workspace_ready` 또는 `bootstrap_failed`로 기록한다.

# worker_pane_split_verification
- 검증용 tmux 세션 식별자는 전체 절차에서 일관되게 유지한다.
- 세션이 없으면 생성 후 단일 pane 상태에서 분할 검증을 시작한다.
- pane 분할 후 pane 개수 증가, 레이아웃, active pane, target pane 지정 상태를 함께 확인한다.
- 대상 pane 선택은 명시적 pane id 또는 index 기반으로 수행한다.
- 기존 사용자 tmux 세션은 파괴하지 않는다.
> 검증 세션을 생성 또는 재사용한다.
> 분할 전 pane 개수와 active pane을 수집한다.
> 지정된 방식으로 pane을 분할하고 레이아웃과 pane index를 확인한다.
> target pane 지정 성공 여부를 판정한다.

# worker_output_verification
- 명령 전송과 출력 캡처는 동일 target pane 기준으로 수행한다.
- 출력 검증은 사전 정의된 키워드 또는 패턴 매칭으로 판정한다.
- 표준 출력과 표준 오류를 구분해 관찰 가능하게 확인한다.
- 출력 누락 시 재시도 횟수와 타임아웃 기준을 적용하고 무한 대기를 금지한다.
- 결과는 성공/실패와 근거 텍스트를 함께 남긴다.
> target pane으로 워커 명령을 전송한다.
> 해당 pane 출력 스트림을 캡처한다.
> 기대 패턴 매칭으로 `output_verified` 또는 `output_mismatch`를 판정한다.
> 출력 미전달 여부를 근거와 함께 기록한다.

# reproducible_cli_checks
- 검증 절차는 동일 입력에 동일 판정 기준을 적용하는 재실행 가능한 명령 순서로 고정한다.
- 자동화 가능한 CLI 흐름을 우선하고 수동 조작 의존을 최소화한다.
- 결과 판정은 관찰 가능한 tmux 상태와 출력 로그를 근거로 한다.
- 실패 원인은 분할 실패, 타깃 pane 지정 실패, 출력 미전달로 명확히 구분한다.
- 프로젝트 범위를 벗어나는 전역 설정 변경과 무관한 파일 변경은 금지한다.
> 고정된 명령 순서로 bootstrap, pane 분할, 출력 검증을 연속 실행한다.
> 동일 조건 재실행 시 판정 일관성을 확인한다.
> 실패 시 원인 분류와 근거 로그를 함께 기록한다.

