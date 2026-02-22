`orc create <project_name> [path] [description]`를 읽고 -> `configs/project.yaml`, `<path>/.project/` 생성/갱신
`orc add-func`를 읽고 -> `<path>/.project/project.md`를 입력으로 질의응답 처리
질의응답 결과를 읽고 -> `<path>/.project/feature/<feature_name>/draft.yaml` 생성
신규 feature 이름을 읽고 -> `<path>/.project/drafts_list.yaml`의 `planned` 항목 생성/갱신
`orc run-parallel-build-code`를 읽고 -> `<path>/.project/feature/*/draft.yaml` 기준으로 코드 파일 처리 및 `.project/log.md` 실패 로그 갱신
`orc init`를 읽고 -> 현재 폴더명 기준 `configs/project.yaml` 등록 및 `.project/project.md` 초안 생성
`orc plan-project`를 읽고 -> 대화형 입력(name/description/spec/goal/rule) 기반으로 `.project/project.md`의 `# info`, `## rule`, `## features` 1차 정리 생성
`orc detail-project`를 읽고 -> 추가 질문 답변 기반으로 `.project/project.md`의 `## features` 확장 및 나머지 섹션 빈 항목 보강
`.project/project.md`의 spec이 `react`를 포함하면 -> `package.json`, `src/main.tsx`, `src/App.tsx`, `index.html`, `tsconfig.json`, `vite.config.ts` 생성/갱신
`orc run-parallel-build-code`를 읽고 -> `.project/feature/*/draft.yaml` 기준 React 코드 파일 처리 및 `.project/log.md` 실패 로그 기록
`orc detail-project`를 읽고 -> 계산기 요구사항(사칙연산/에러처리/입력흐름)을 `.project/project.md`의 `## features`, `# Flow`, `# Verification`에 추가/보강
`orc draft-create calculator`를 읽고 -> `.project/feature/calculator/draft.yaml` 생성 및 계산기 task 초안 작성
`orc draft-add calculator "연산 우선순위/소수점 처리"`를 읽고 -> `.project/feature/calculator/draft.yaml`에 계산기 세부 task 추가
`orc run-parallel-build-code`를 읽고 -> `src/App.tsx`, `src/components/Calculator.tsx`, `src/hooks/useCalculator.ts` 생성/갱신 및 `.project/log.md` 실패 로그 기록
`orc init`를 읽고 -> 현재 폴더명으로 프로젝트 등록 후 `.project/project.md` 초안 생성
`orc plan-project`를 읽고 -> 대화형 입력으로 React Native 단어암기 앱의 `# info`, `## rule`, `## features` 1차 설계 생성
`orc detail-project`를 읽고 -> 학습세트/퀴즈/복습주기 요구사항을 반영해 `.project/project.md`의 features/flow/verification 상세화
`orc draft-create vocabMemorize`를 읽고 -> `.project/feature/vocabMemorize/draft.yaml` 생성
`orc draft-add vocabMemorize "오답노트, spaced repetition, 로컬 저장"`를 읽고 -> `.project/feature/vocabMemorize/draft.yaml`에 세부 task 추가
`orc run-parallel-build-code`를 읽고 -> `App.tsx`, `src/screens/*`, `src/features/vocab/*`, `src/storage/*` 생성/갱신 및 `.project/log.md` 실패 로그 기록
