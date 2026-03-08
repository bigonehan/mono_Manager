# 구현 확인
- `input.md` 기준으로 `./.project/check_list.md`를 생성/갱신했다.
- 고정 반환/스텁 후보를 `Ok(false)|Ok(true)|return false|return true|todo!/unimplemented!` 패턴으로 전수 점검했다.
- 실제 스텁 문제로 확인된 `sync_project_tasks_list_from_project_md`는 이미 구현되어 있었고, 추가로 정합성 로직을 강화했다.
- `plan.yaml` 도메인 동기화 보강: `code::sync_plan_doc`가 `project.md` 도메인을 병합하도록 수정했다.
- `drafts_list.yaml` 상태 스키마 호환 보강: `DraftsListDoc`에 `worked/complete/failed`를 추가했다.
- `planned`/`planned_items` 중복 완화: 기본 키복제(`value == name`)를 생성하지 않도록 수정했다.
- 검증 명령:
  - `cargo test --quiet` -> 통과 (23 passed)

# 발견된 문제
- `sync_project_tasks_list_from_project_md`가 과거 스텁 상태에서 넘어온 영향으로, 도메인/상태/planned_items 정합화가 깨질 수 있는 경로가 있었음.
- `DraftsListDoc`의 상태 필드 누락으로 `plan.yaml.drafts`와 `drafts_list.yaml` 상태 호환이 불완전했음.
- add-plan 경로에서 `planned`와 동일한 항목을 `planned_items`에 무조건 복제해 중복이 누적될 수 있었음.
- 위 문제는 이번 변경으로 모두 수정했고, 현재 테스트 기준에서 재현되지 않음.
