# 문제
- `npm --prefix assets/web run check` 실행 시 `@astrojs/check` 미설치로 interactive 설치 프롬프트가 떠서 비대화형 검증이 중단됐다.
- `cargo test`에 여러 test name 필터를 한 번에 넘겨 검증 명령이 usage error로 종료됐다.
- `PageEditor`에서 `impl_code_draft`를 다시 실행하니 draft 항목이 모두 `worked` 상태여서 `no drafts.yaml.planned item`으로 바로 skip됐다.
- `PageEditor`에서 `impl_code_draft`를 7개 feature 묶음 prompt로 재실행하니 150초 timeout 후 같은 긴 prompt 재시도만 반복되고 앱 파일은 생성되지 않았다.
- `rc`는 직접 `test` subcommand를 받지 않는데 `cargo run --bin rc -- test ...`로 호출해 usage error가 발생했다.

# 미해결점
- 현재 저장소 기본 상태만으로 `astro check`를 즉시 실행할 수 없다.
- web 변경 검증은 설치 프롬프트가 없는 대체 경로로 다시 확인해야 한다.
- `npm --prefix assets/web exec -- tsc --noEmit -p tsconfig.json`는 실행 cwd가 루트라 상대 `tsconfig.json`을 찾지 못했다.
- `impl_code_draft` 수정 검증은 단일 필터 재실행 또는 전체 `cargo test`로 다시 확인해야 한다.
- 중단된 `impl_code_draft` 재시도는 기존 `worked` 항목을 이어서 처리하도록 보정한 뒤 다시 실행해야 한다.
- `impl_code_draft`는 item 단위 짧은 prompt와 재시도 억제 경로로 다시 조정한 뒤 `PageEditor`에서 파일 생성 여부를 재확인해야 한다.
- smoke 절차 실검증은 `orc clit test ...` 브리지나 `rc check --mode ...`의 실제 허용 CLI 형식으로 다시 실행해야 한다.

#개선필요
- web 검증 기본 명령이 추가 설치 프롬프트 없이 동작하는지 사전 점검하거나, 저장소 기본 검증 명령을 `tsc --noEmit` 같은 비대화형 경로와 함께 문서화할 필요가 있다.
- `npm --prefix ... exec` 사용 시 도구가 참조하는 상대 경로 기준 디렉터리를 함께 고정하는 검증 예시가 필요하다.
- `cargo install --path /home/tree/project/rust-orc` 실행 시 같은 패키지명을 쓰는 다른 저장소의 실행 파일(`rw`)까지 교체 대상이 될 수 있어, 설치 검증 정책이나 패키지명 분리가 필요하다.
- `orc auto` 빈 폴더 경로는 메타 파일 조합이 바뀌면 다시 깨질 수 있으니, `.project`/`todo.md`만 있는 임시 디렉터리 기준 end-to-end 회귀 검증을 별도 명령이나 테스트로 고정할 필요가 있다.
- `check_code_draft`의 기본 `test_command()`는 non-Rust web 앱에서 `Cargo.toml not found`로만 끝나므로, `package.json`/`bun.lock` 기반 테스트 명령 탐지까지 연결할 필요가 있다.
- `rc` web smoke는 dev 서버가 5173 충돌 시 5174 등으로 이동해도 고정 URL을 계속 사용하므로, 서버 로그 기반 실제 포트 추적이 필요하다.
- SpeechRecognition 엔진별 result 배열 overlap 패턴이 더 다양할 수 있어, mock 시나리오 외에 실제 브라우저 이벤트 fixture를 몇 종류 확보해 회귀 세트를 넓힐 필요가 있다.
