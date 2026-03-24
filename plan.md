## 문제
- drafts pane의 `>` 아이콘이 현재 requirement 추가/병합 경로(`job-md-generate`)를 타고 있어 역할이 섞여 있다.
- 사용자가 원하는 `>` 역할은 "현재 job.md requirement를 drafts.yaml로 변환"이며 requirement 추가는 별도 버튼이어야 한다.
- draft_item list/detail pane의 x축 정렬 기준선이 헤더 높이 차이로 어긋난다.

## 해결책
- 서버에 job.md requirement -> drafts.yaml 동기화 전용 API를 추가하고 `>` 버튼은 해당 API만 호출하도록 변경한다.
- requirement 추가는 `+` 버튼/모달 경로(`submitRequirementBlocks`)만 담당하도록 분리한다.
- work pane 좌/우 헤더 높이를 동일 고정하고 본문 높이도 동일치로 맞춰 x축 기준선을 일치시킨다.
- E2E에서 `>` 클릭 전후 `job.md` 불변 + `drafts.yaml` 갱신을 검증하고 pane 높이 차이를 검증한다.

## 검증
- `npm --prefix assets/web run test:unit`
- `npm --prefix assets/web exec -- tsc --noEmit -p assets/web/tsconfig.json`
- `npm --prefix assets/web run test:e2e`
- `npm --prefix assets/web run test:e2e:end-hook`
- `cargo test -q`
- `rm -rf /tmp/tmp_project /tmp/pw-*`
- `eza -la /tmp | rg 'tmp_project|pw-'` 결과 0건
- `rg -n 'tmp_project|pw-' configs/project.yaml` 결과 0건
