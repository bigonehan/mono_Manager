# plan
- 현재 web ui에서 스크린샷으로 확인하는 부분이 왜 작동하지 않는지 원인을 좁힌다.
- manager session은 직접 구현/QA/check/브라우저 실행을 하지 않고 worker로만 진행한다.
- impl worker가 수정과 관련 테스트를 수행하고 dev server를 유지한 채 실제 URL을 보고한다.
- qa worker가 회수한 URL에 실제 접속해 스크린샷 확인 흐름을 검증한다.
- check worker가 요구사항 대응 기준으로 점검한다.
- improve worker가 blocking 문제만 다시 분류한다.
- manager가 `job.md`와 trace를 재확인한 뒤 종료 게이트를 통과한다.

# input
- `~/ai/codex/AGENTS.override.md`를 먼저 읽어야 한다.
- `/home/tree/ai/skills/orc_manager/SKILL.md`를 사용해야 한다.
- 먼저 plan mode로 계획을 확정해야 한다.
- plan 확정 직후 현재 세션을 manager session으로 고정해야 한다.
- manager session은 직접 구현/점검/브라우저 실행을 하지 않아야 한다.
- impl, qa, check, improve 역할을 분리해야 한다.
- worker 위임은 기존 tmux worker wrapper인 `orc worker-*` 경로로만 해야 한다.
- impl worker는 dev server를 유지한 채 완료 메시지로 URL을 회수해야 한다.
- qa/check worker를 별도로 열어 실제 접속 검증과 점검을 수행해야 한다.
- manager는 worker 결과만 믿지 말고 `job.md`를 재확인한 뒤 `stage_manager_reverified`까지 기록해야 한다.
- 위 절차를 어기면 수정 후 같은 절차를 다시 반복해야 한다.
- 중간 승인 없이 끝까지 진행해야 한다.
- 현재 web ui에서 스크린샷으로 확인하는 부분이 작동하지 않는 문제를 해결해야 한다.

# output
- web ui의 스크린샷 확인 기능이 다시 동작한다.
- impl worker가 실제 healthcheck를 통과한 dev URL을 보고한다.
- qa worker가 실제 접속으로 스크린샷 선택/미리보기 또는 관련 확인 흐름을 검증한다.
- check worker가 요구사항과 검증 누락을 점검한다.
- manager가 `stage_manager_reverified`를 남기고 종료 게이트를 통과한다.

# keep
- manager session은 직접 구현, QA, check, 브라우저 실행을 하지 않는다.
- impl, qa, check, improve는 분리된 worker session으로 유지한다.
- 변경 범위는 스크린샷 확인 문제와 그 검증 경로 중심으로 제한한다.

# add
- 스크린샷 확인 실패 증상과 성공 조건 잠금
- impl worker 수정 결과와 dev URL 근거
- 실제 접속 기반 QA artifact와 check report
- manager 재검증 trace와 완료 기록

# forbid
- manager session이 직접 `orc impl_*`, `orc check_*`, dev server 실행, 브라우저 검증을 수행하는 것
- worker 역할을 같은 session에 섞는 것
- mock/fixture 성공만으로 스크린샷 확인 완료를 선언하는 것
- worker 보고만 믿고 `job.md`와 trace 재확인 없이 종료하는 것
- 관련 없는 UI 리팩터링이나 전면 정리를 수행하는 것

# symptom
- 현재 web ui에서 체크/스크린샷 영역의 확인 흐름이 기대대로 작동하지 않는다.
- 사용자가 스크린샷을 선택하거나 열어도 실제 확인이 되지 않는 경로가 있다.

# success
- 실제 web ui에서 스크린샷 확인 흐름이 다시 동작한다.
- QA 결과에 `symptom reproduced`, `symptom cleared`, `negative-check passed`가 남는다.
- manager가 worker 보고와 별개로 `job.md`와 trace를 다시 확인한 뒤 완료 처리한다.

# hard gate
## requirement_lock
- manager session은 직접 구현/점검/브라우저 실행을 하지 않는다.
- worker 위임은 `orc worker-create`, `orc worker-send`, `orc worker-wait`, `orc worker-close`로만 한다.
- impl, qa, check, improve는 분리된 worker session으로 유지한다.
- 스크린샷 확인 문제는 실제 web ui 동작 기준으로 해결해야 한다.
## forbidden_substitutions
- API 응답만 정상이라고 UI 확인 완료로 간주하지 않는다.
- screenshot 파일 존재만으로 미리보기 동작 성공으로 간주하지 않는다.
- fixture/mock 결과만으로 실제 접속 검증을 대체하지 않는다.
- worker done 메시지만 보고 완료 처리하지 않는다.
## verification_examples
- md 저장 != 메모리 유지
- 재시작 != reload
- 실제 e2e != fixture, mock, real-equivalent

# verify axes
- render
- mutation
- persistence
- re-entry
- negative-check

# check
## input_output_checklist
- 스크린샷 확인 문제 지적이 실제 원인 수정과 재검증으로 이어져야 한다.
- plan 확정 이후 manager session 고정 요구가 실제 worker 분리와 trace 기록으로 이어져야 한다.
- impl worker의 dev server 유지 요구가 실제 URL 회수와 healthcheck 통과 결과로 이어져야 한다.
- qa/check 분리 요구가 실제 접속 검증과 별도 점검 리포트로 이어져야 한다.
- 중간 승인 없이 끝까지 진행 요구가 blocker 해소 전 종료 금지로 이어져야 한다.
## keep_checklist
- manager session이 직접 구현/점검/브라우저 실행을 하지 않았는지 확인해야 한다.
- 역할별 worker session 분리가 마지막까지 유지됐는지 확인해야 한다.
- `job.md`가 manager와 worker의 공통 source of truth로 유지됐는지 확인해야 한다.
## add_checklist
- 스크린샷 확인 실패 원인과 수정 결과가 남아야 한다.
- 실제 dev URL, QA artifact, check report, improve 분류 결과가 남아야 한다.
- manager 재검증과 completion trace가 남아야 한다.
## forbid_checklist
- worker 결과만 보고 종료하지 않았는지 반증해야 한다.
- fixture/mock만으로 완료 처리하지 않았는지 반증해야 한다.
- blocker가 남은 상태에서 종료하지 않았는지 반증해야 한다.

# task
## planned
- web_ui_screenshot_check_fix
## work
- impl worker identified the root cause in successful `check_orc_code` screenshot cleanup and added retained-copy support for web UI consumption.
- impl worker wired manual check execution to request retained screenshots into the project `.project/screenshot` directory.
- qa worker verified on `http://127.0.0.1:4275` that screenshot card render, preview open, feedback append, and reload/re-entry all work with a real browser session.
- check worker reran targeted `rc` tests and `astro check`, then confirmed the new retention path is connected in both Rust and web server code.
- improve worker initially reported a bookkeeping blocker from unchecked `job.md` items, and manager resolved it by revalidating and updating the completion state.
## verify
- [x] symptom reproduced
- [x] symptom cleared
- [x] re-entry verified
- [x] negative-check passed
## complete
- web_ui_screenshot_check_fix
## fail

# problems


# check evidence
- [x] plan fixed and job locked
- [x] preflight trace passed
- [x] impl worker report with dev url
- [x] qa worker real or real-equivalent report
- [x] check worker report
- [x] improve worker classification
- [x] manager reverified job and trace
