# 기본 규칙
- `project.md/spec`에 맞춰 hello world 최소 실행 환경을 만든다.
- `spec`만 기준으로 최소 초기 실행 골격을 만든다.
# 구현 분기 
1. `spec`을 기준으로 사용 언어& 프레임 워크를 파악한다. 
2. 언어별로 다음 목적에 따른 초기화 환경을 구축한다. 
## rust
-  `cargo run` 시 `hello world` 출력
## react
- react/next/vite 계열: 실행 시 화면에 `hello world` 렌더
4) 필요한 경우 `package.json`, 엔트리 파일, 빌드 설정 파일을 생성/수정한다.
5) `spec`에 라이브러리명이 있으면 `package.json` 의 dependency/devDependency에 반드시 반영하고 프로젝트내 의존성에 포함한다 
7) `spec`에 계산기 구현을 암시하는 정보가 있을 때만 최소 계산기 구조(`app/page.tsx`, `store/calc.ts` 또는 동등 파일)를 생성한다.
8) `.gitignore`가 없으면 생성하고, 있으면 유지하면서 `*.png` ignore 규칙을 반드시 포함한다.
9) 기존 파일이 있으면 덮어쓰기 전에 현재 구조와 충돌하지 않게 최소 변경으로 반영한다.
10) 출력은 첫 줄에 `BOOTSTRAP_DONE:`로 시작하는 한 줄 요약을 반드시 포함한다.
11) 설명은 짧게, 코드블록 없이 텍스트로만 출력한다.
12) auto 모드에서는 질문/확인 요청 없이 spec만 기준으로 스스로 판단해 진행한다.
