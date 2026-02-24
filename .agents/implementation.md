# Implementation Doc

## Task
- bootstrap 실행 기준 보강
  - react/next/vite: 생성 파일에 `hello world` 렌더 보장
  - rust: `src/main.rs`에 `println!("hello world")` 보장 + `cargo run` 출력 확인 가능 상태
- /tmp 재현 기준으로 three fiber spec 반영 확인 및 누락 보정
- Project tab load preset 기능을 assets/presets/project.yaml과 연결
- preset 내용은 spec 라이브러리 기본 목록만 허용

## Files
- src/ui/mod.rs
- assets/presets/project.yaml (new)
- .agents/log.md

## Checks
- cargo test
- /tmp 재현 점검 (node/react)
  - `mkdir -p /tmp/orc-check-node && rm -rf /tmp/orc-check-node/*`
  - three fiber spec으로 bootstrap 실행
  - `/tmp/orc-check-node/package.json`에 `three`, `@react-three/fiber`, `@react-three/drei` 포함 확인
- /tmp 재현 점검 (rust)
  - `mkdir -p /tmp/orc-check-rust && rm -rf /tmp/orc-check-rust/*`
  - rust spec으로 bootstrap 실행
  - `/tmp/orc-check-rust/src/main.rs`에 `println!("hello world")` 포함 확인
  - `/tmp/orc-check-rust`에서 `cargo run` 실행 시 stdout `hello world` 확인
- Project tab에서 `l` 키 입력 시 create modal의 `spec` 필드가 `assets/presets/project.yaml` 첫 preset의 libraries 값으로 반영되는지 확인

## Preset Allowlist
- 허용 목록: `react`, `react-dom`, `next`, `vite`, `typescript`, `javascript`, `axios`, `zod`, `zustand`, `@tanstack/react-query`, `tailwindcss`, `three`, `@react-three/fiber`, `@react-three/drei`, `react-native`, `expo`, `rust`, `tokio`, `serde`, `serde_json`, `reqwest`, `axum`
- 허용 목록 외 값은 preset 로드 시 자동 제외
