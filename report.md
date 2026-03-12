# 구현 확인
- `input.md`와 `./.project/draft.yaml`가 없어 현재 요청 범위(`impl_code_draft`, `check_code_draft`, `rc` web smoke)를 기준으로 체크리스트를 재구성했다.
- `cargo test`를 실행했고 `src/main.rs` 38개, `src/bin/rc.rs` 11개 테스트가 통과했다.
- `rg -n "contains\\(|starts_with\\(|ends_with\\(" src`로 문자열/패턴 기반 하드코딩 판정 경로를 점검했다.

# 발견된 문제
- [높음] `src/code.rs:957`, `src/code.rs:1862`, `src/code.rs:3493`, `src/bin/rc.rs:1076`는 `contains("fail")`, `contains("error")`, `contains("exit")` 같은 substring으로 성공/실패를 최종 판정한다. 출력 언어, 로그 문맥, 도구 메시지 포맷이 바뀌면 false positive/negative가 생길 수 있다.
- [중간] `src/code.rs:1163`, `src/code.rs:1191`, `src/code.rs:1255`의 impl worker 검증은 shared baseline과 worker가 스스로 적은 `modified_files` 목록에 기대고 있어, 같은 워크스페이스에서 다른 worker가 바꾼 파일을 본인 변경처럼 통과시킬 수 있다.
- [중간] `src/code.rs:931`의 상태 저장은 프로세스 내부 `OnceLock<Mutex<()>>`만 사용하고 `plan.yaml`과 `drafts.yaml`를 별도 write 한다. tmux pane이나 별도 ORC 프로세스가 동시에 접근하면 lost update나 부분 기록 가능성이 남아 있다.
- [중간] `src/bin/rc.rs:682`, `src/bin/rc.rs:823`는 web smoke 준비를 `sleep 5`와 `vite -> 5173` 고정 규칙으로 처리한다. dev server startup 시간이 길거나 포트가 다르면 로그인 selector 문제를 고쳐도 여전히 잘못된 URL/타이밍으로 검증할 수 있다.
