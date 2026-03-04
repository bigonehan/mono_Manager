# plan

## 문제
- `check_code_draft` 관련 산출 경로가 `.project/runtime` 중심으로 되어 있고, 사용자는 check-code 생성 폴더를 `./.project/reference`로 바꾸길 원한다.
- `report.md`는 현재 코드에서 문자열 하드코딩 형식으로 생성되며, 사용자는 템플릿(`assets/code/templates/report.md`) 기준으로 생성하고 섹션을 `# 구현 확인`, `# 발견된 문제`만 남기길 원한다.

## 해결책
1. `src/code.rs`의 `check_code_draft` 경로 및 debug tail 경로를 `./.project/reference`로 변경한다.
2. `src/main.rs`의 check-code runtime log 경로를 `./.project/reference/check-code.log`로 변경한다.
3. `assets/code/templates/report.md` 템플릿을 추가하고, `check_code_draft`가 템플릿을 읽어 치환 후 저장하도록 변경한다.
4. `check-code` skill 문서를 업데이트해 결과 보고 형식을 `# 구현 확인`, `# 발견된 문제` 중심으로 제한한다.

## 검증
- `check_code_draft` 실행 시 `.project/reference`가 생성되고 로그가 해당 경로에 기록된다.
- 생성되는 `report.md`는 템플릿 형식을 따르고 `# 구현 확인`, `# 발견된 문제` 두 섹션만 포함한다.
- `cargo check` 통과.
