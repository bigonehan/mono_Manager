# plan
- 현재 web ui와 orc 기능 매핑을 전수 점검해 실제 명령, 상태, 파일 흐름과 어긋난 부분을 찾는다.
- 불일치가 있으면 UI, API, server mapping, store, 검증 코드를 수정해 실제 내부 과정을 반영하게 만든다.
- manager session은 직접 구현/점검/브라우저 실행을 하지 않고 impl, qa, check, improve worker로만 진행한다.
- impl worker는 dev server를 유지한 채 실제 healthcheck를 통과한 URL을 보고한다.
- qa/check/improve worker 결과와 `job.md` 재확인을 모두 통과할 때까지 반복한다.

# input
- `~/ai/codex/AGENTS.override.md`를 먼저 읽고 `/home/tree/ai/skills/orc_manager/SKILL.md`를 사용해야 한다.
- 먼저 plan mode로 계획을 확정해야 한다.
- plan 확정 직후 현재 세션을 manager session으로 고정해야 한다.
- manager session은 직접 구현, 점검, 브라우저 실행을 하지 않아야 한다.
- impl, qa, check, improve 역할은 반드시 tmux 새 session 또는 pane으로 분리해야 한다.
- worker 위임은 반드시 `orc send-tmux` 또는 기존 tmux worker wrapper로만 해야 한다.
- impl worker는 dev server를 유지한 채 완료 메시지로 URL을 회수해야 한다.
- qa/check worker를 별도로 열어 실제 접속 검증과 점검을 수행해야 한다.
- manager는 worker 결과만 믿지 말고 `job.md`를 재확인한 뒤 `stage_manager_reverified`까지 기록해야 한다.
- 위 절차를 어기면 수정 후 같은 절차를 다시 반복해야 한다.
- 중간 승인 없이 끝까지 진행해야 한다.
- 현재 web ui에서 제대로 orc 기능과 매핑이 안 되는 곳을 전부 확인하고 수정해야 한다.

# output
- 현재 web UI가 orc 기능, 실제 명령, 상태 전이, 파일 흐름을 반영하도록 코드가 수정된다.
- 실제 dev server URL이 impl worker에서 회수되고 QA worker가 그 URL로 실제 접속 검증을 완료한다.
- check worker가 `check-code` 기준으로 검증하고 manager가 `job.md`와 trace를 재확인한다.
- blocking 문제가 남지 않은 상태로 completion guard를 통과한다.

# keep
- 현재 세션은 manager 역할만 수행하고 직접 구현/QA/check/브라우저 실행을 하지 않는다.
- impl, qa, check, improve 역할은 서로 다른 worker session으로 유지한다.
- worker 결과 검증 전에 `job.md`를 source of truth로 유지한다.

# add
- 입력 대비 출력 매핑이 명시된 checklist
- UI와 내부 process 불일치 및 orc action mapping 분석 결과에 따른 실제 코드 수정
- 실제 dev server URL 회수와 healthcheck 근거
- 실제 접속 기반 QA 결과와 `check-code` 기반 check 결과
- manager 재검증 trace와 completion 기록

# forbid
- manager session이 직접 `orc impl_*`, `orc check_*`, dev server 실행, 브라우저 검증을 수행하는 것
- impl/qa/check/improve 역할을 같은 session에 섞는 것
- worker done 메시지만 믿고 `job.md` 재확인 없이 완료 처리하는 것
- fixture/mock만으로 UI 개선 완료를 선언하는 것
- 부분 개선만 하고 남은 blocker가 있는데 종료하는 것

# symptom
- 내부 process와 orc command surface는 바뀌었지만 front web UI가 그 변경을 충분히 반영하지 않았을 가능성이 높다.
- UI가 실제 명령/상태/파일 흐름과 어긋나면 사용자가 현재 process를 잘못 이해하거나 잘못 조작할 수 있다.

# success
- web UI가 현재 내부 process, orc 명령, 상태 변화, 실행 버튼/로그/API 결과를 일관되게 반영한다.
- 실제 접속 기준으로 핵심 UI 흐름이 동작하고 re-entry와 negative-check까지 검증된다.
- manager가 `stage_manager_reverified`를 남기고 completion guard를 통과한다.

# hard gate
## requirement_lock
- manager session은 직접 구현/점검/브라우저 실행을 하지 않는다.
- worker 위임은 `orc send-tmux` 또는 기존 tmux worker wrapper로만 한다.
- impl, qa, check, improve는 분리된 tmux session 또는 pane으로 유지한다.
- web ui의 orc 기능 매핑 누락은 전수 점검 후 남김없이 수정한다.
## forbidden_substitutions
- 부분 경로만 점검하고 전체 매핑 점검을 완료로 간주하지 않는다.
- manager가 직접 dev server나 브라우저를 다뤄 worker 역할을 대체하지 않는다.
- worker done 메시지만 보고 완료 처리하지 않는다.
- fixture/mock만으로 실제 접속 검증을 대체하지 않는다.
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
- 현재 web ui의 orc 기능 매핑 누락이 실제 코드 수정 또는 근거 있는 no-op 판단으로 이어져야 한다.
- plan 확정 이후 manager session 고정 요구가 실제 worker 분리 실행과 trace 기록으로 이어져야 한다.
- impl worker의 dev server 유지 요구가 실제 URL 회수와 healthcheck 통과 결과로 이어져야 한다.
- qa/check 분리 요구가 실제 접속 검증과 별도 점검 리포트로 이어져야 한다.
- 중간 승인 없이 끝까지 진행 요구가 blocker 해소 전 종료 금지로 이어져야 한다.
## keep_checklist
- manager session이 직접 구현/점검/브라우저 실행을 하지 않았는지 확인해야 한다.
- 역할별 worker session 분리가 마지막까지 유지됐는지 확인해야 한다.
- `job.md`가 manager와 worker의 공통 source of truth로 유지됐는지 확인해야 한다.
## add_checklist
- UI와 내부 process, orc action mapping 불일치 분석이 코드 수정 또는 근거 있는 no-op 판단으로 남아야 한다.
- 실제 dev URL, QA artifact, check report, improve 분류 결과가 모두 남아야 한다.
- manager 재검증과 completion trace가 남아야 한다.
## forbid_checklist
- worker 결과만 보고 종료하지 않았는지 반증해야 한다.
- fixture/mock만으로 완료 처리하지 않았는지 반증해야 한다.
- blocker가 남은 상태에서 final로 종료하지 않았는지 반증해야 한다.

# task
## planned
- web_ui_orc_mapping_alignment
## work
- manager orchestration for web ui orc mapping audit and fix
- impl worker corrected web ui labels, tooltips, and server outputs from legacy rc/build wording to `impl_orc_code` and `check_orc_code`
- qa worker verified real browser render, requirement save, job.md persistence, reload, and legacy rc text absence at `http://127.0.0.1:4176`
- check worker ran `npm run test:unit`, `npm run check`, and legacy-string guards
## verify
- [x] symptom reproduced
- [x] symptom cleared
- [x] re-entry verified
- [x] negative-check passed
## complete
- web_ui_orc_mapping_alignment
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
