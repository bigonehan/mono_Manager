# 기능 계획: detail drafts pane 파일 삭제 + add/modify 가드

## 범위
- detail page `drafts` 섹션에서 `input.md`, `drafts.yaml` 뷰 각각에 휴지통 버튼을 추가한다.
- 버튼 클릭 시 `y/n` 확인 후 `y`일 때만 대상 파일을 실제 삭제한다.
- 파일 삭제 뒤 add/modify 경로에서 파일 부재를 명시적으로 가드한다.

## 입출력
- 입력: detail 선택 프로젝트 id, 삭제 대상(`input` 또는 `drafts`)
- 출력: 대상 파일 삭제 결과 + 갱신된 project detail

## 검증
- API 직접 호출로 `input.md`/`drafts.yaml` 삭제가 실제 파일 시스템에 반영되는지 확인
- 삭제 이후 `add_draft` 실행 시 `drafts.yaml` 부재 가드 에러 확인
- 삭제 이후 `input-md-raw` 저장 시 `input.md` 부재 가드 에러 확인

## 범위 추가: auto_add_function
- 신규 CLI `auto_add_function <message>`를 추가하고, web의 auto 실행 경로를 기존 `auto`에서 `auto_add_function`으로 전환한다.
- `auto_add_function`은 message 기반 input.md/plan(draft) 생성, 병렬 구현+재시도+check 루프, 개선 codex 실행, `plan.md` 스냅샷 작성을 수행한다.
- web runtime 완료 시 project state를 `complete`로 설정해 UI 배지에 반영한다.
