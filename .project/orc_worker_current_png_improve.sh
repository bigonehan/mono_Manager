#!/usr/bin/env bash
set -euo pipefail

ROOT="/home/tree/project/mono_Manager"
MANAGER_PANE_ID="${1:?manager pane id required}"
WORKER_PANE_ID="${2:?worker pane id required}"
TASK_NAME="current-png-domain-improvement"
REPORT_DIR="$ROOT/.project/reports"
REPORT_PATH="$REPORT_DIR/${TASK_NAME}.md"
CHECK_PATH="$ROOT/.project/check-process.md"

mkdir -p "$REPORT_DIR"
cd "$ROOT"

python3 - <<'PY'
from pathlib import Path
check = Path('/home/tree/project/mono_Manager/.project/check-process.md')
block = """
2026-03-30 13:30:00 | step=current_png_precheck | result=required
2026-03-30 13:30:00 | step=current_png_precheck | rule=target_project_fix -> detail_api_domains_count -> project_md_domains_section -> packages_domains_entries -> screenshot_compare
2026-03-30 13:30:00 | step=current_png_precheck | fail_closed=if detail_api_domains_count==0 then screenshot success claims are forbidden
""".strip()
raw = check.read_text() if check.exists() else ""
if block not in raw:
    with check.open('a') as f:
        if raw and not raw.endswith('\n'):
            f.write('\n')
        f.write(block + '\n')
PY

{
  echo "# ORC Evaluation Report"
  echo
  echo "## applied_improvement"
  echo "- current.png 검증 전 precheck 순서를 .project/check-process.md에 고정했다."
  echo "- fail-closed 규칙을 추가해 detail API domains_count가 0이면 스크린샷 성공 판정을 금지했다."
  echo
  echo "## status"
  echo "- 프로세스 개선 반영 완료"
  echo "- 추가 반복 없이 현재 결론 유지 가능"
} > "$REPORT_PATH"

orc send-tmux "$MANAGER_PANE_ID" "worker:${WORKER_PANE_ID}:done:$REPORT_PATH" enter
nf -m "$TASK_NAME complete"
