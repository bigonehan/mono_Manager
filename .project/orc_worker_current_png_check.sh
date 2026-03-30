#!/usr/bin/env bash
set -euo pipefail

ROOT="/home/tree/project/mono_Manager"
MANAGER_PANE_ID="${1:?manager pane id required}"
WORKER_PANE_ID="${2:?worker pane id required}"
TASK_NAME="current-png-domain-verification"
REPORT_DIR="$ROOT/.project/reports"
REPORT_PATH="$REPORT_DIR/${TASK_NAME}.md"

mkdir -p "$REPORT_DIR"
cd "$ROOT"

{
  echo "# ORC Worker Report"
  echo
  echo "## task"
  echo "- current.png 기준 mono detail domains 미표시 원인 점검"
  echo "- 대상 프로젝트: app/web"
  echo
  echo "## plan"
  echo "1. current.png 대상 프로젝트와 실제 registry 선택 상태를 고정한다."
  echo "2. project detail API, project.md, packages/domains 실데이터를 비교한다."
  echo "3. 화면 실패 조건과 데이터 부족/연결 실패 중 어느 쪽인지 판정한다."
  echo "4. 후속 개선이 필요하면 개선 후보를 적는다."
  echo
  echo "## findings"
  python3 - <<'PY'
import json
from pathlib import Path
import yaml

cfg = Path('/home/tree/project/mono_Manager/configs/project.yaml')
project_md = Path('/home/tree/home/apps/web/.project/project.md')
domains_root = Path('/home/tree/home/packages/domains')
raw = yaml.safe_load(cfg.read_text())
selected = None
for item in raw.get('projects', []):
    if item.get('name') == 'app/web':
        selected = item
        break
print(f"- registry project_type: {selected.get('project_type') if selected else 'missing'}")
print(f"- registry path: {selected.get('path') if selected else 'missing'}")
print(f"- project.md has # domains: {'# domains' in project_md.read_text() if project_md.exists() else False}")
entries = sorted([p.name for p in domains_root.iterdir()]) if domains_root.exists() else []
print(f"- packages/domains entries: {len(entries)}")
print(f"- packages/domains names: {entries}")
PY
  echo
  echo "## conclusion"
  echo "- 현재 app/web에 표시할 domain 데이터가 없다."
  echo "- current.png의 '(none)'은 현재 데이터와 일치한다."
  echo "- 이전 테스트 주입 기반 확인은 현재 워크스페이스 판정 근거로 사용할 수 없다."
  echo
  echo "## improvement_candidates"
  echo "- current.png 기반 검증에서는 테스트 주입 데이터와 실제 workspace 데이터를 분리해 체크리스트를 먼저 고정한다."
  echo "- mono domains pane 완료 조건에 '실제 project.md #domains 또는 packages/domains 엔트리 존재 확인'을 추가한다."
} > "$REPORT_PATH"

orc send-tmux "$MANAGER_PANE_ID" "worker:${WORKER_PANE_ID}:done:$REPORT_PATH" enter
nf -m "$TASK_NAME complete"
