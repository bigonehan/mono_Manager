#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="/home/tree/project/rust-orc/.tmp_tmux_test2"
WORKSPACE_ID="workspace_bootstrap_school_landing_v1"
SESSION_NAME="tmux_${WORKSPACE_ID}"
LOG_DIR="$ROOT_DIR/logs/workspace_bootstrap"
RUNTIME_DIR="$LOG_DIR/runtime"
FORCE_FAIL_STEP="${FORCE_FAIL_STEP:-}"
TRANSITION_LOG="$LOG_DIR/state.log"
EXECUTION_LOG="$LOG_DIR/execution.log"
FAILURE_LOG="$LOG_DIR/failure.log"
STATUS_FILE="$LOG_DIR/final.status"
ASSET_HASH_FILE="$LOG_DIR/assets.sha256"
FORCE_FAIL_STEP="${WORKSPACE_BOOTSTRAP_FORCE_FAIL_STEP:-${FORCE_FAIL_STEP:-}}"

record_state() {
  printf '%s\n' "$1" >> "$TRANSITION_LOG"
}

record_exec() {
  printf '%s\n' "$1" >> "$EXECUTION_LOG"
}

cleanup_session() {
  if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
    tmux kill-session -t "$SESSION_NAME"
  fi
}

fail_now() {
  local reason="$1"
  record_state "실패"
  printf '%s\n' "$reason" > "$FAILURE_LOG"
  printf '%s\n' "실패" > "$STATUS_FILE"
  record_exec "result=fail reason=$reason"
  cleanup_session
  exit 1
}

if [[ "$(pwd)" != "$ROOT_DIR" ]]; then
  printf 'run this command from %s\n' "$ROOT_DIR" >&2
  exit 1
fi

command -v tmux >/dev/null 2>&1 || { printf 'tmux is required\n' >&2; exit 1; }
command -v bash >/dev/null 2>&1 || { printf 'bash is required\n' >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { printf 'sha256sum is required\n' >&2; exit 1; }

mkdir -p "$LOG_DIR"
rm -rf "$RUNTIME_DIR"
mkdir -p "$RUNTIME_DIR"
: > "$TRANSITION_LOG"
: > "$EXECUTION_LOG"
rm -f "$FAILURE_LOG" "$STATUS_FILE" "$ASSET_HASH_FILE"

record_state "초기"
record_exec "workspace_id=$WORKSPACE_ID"
record_exec "session_name=$SESSION_NAME"
record_exec "root_dir=$ROOT_DIR"
record_exec "force_fail_step=${FORCE_FAIL_STEP:-none}"

cat > "$ROOT_DIR/index.html" <<'HTML'
<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>School Landing Page</title>
  <link rel="stylesheet" href="./styles.css" />
</head>
<body>
  <header class="hero">
    <p class="eyebrow">Brighton Public School</p>
    <h1 id="school-title">Welcome to Our School Community</h1>
    <p id="status-text">Status: Preparing programs and admissions guidance</p>
    <a class="cta" href="#programs">Explore Programs</a>
  </header>
  <section id="programs" class="programs">
    <article><h2>Science Lab</h2><p>Hands-on experiments for curious minds.</p></article>
    <article><h2>Arts Studio</h2><p>Music, drawing, and performance every week.</p></article>
    <article><h2>Sports Club</h2><p>Teamwork and healthy routines after class.</p></article>
  </section>
  <script src="./script.js"></script>
</body>
</html>
HTML

cat > "$ROOT_DIR/styles.css" <<'CSS'
:root {
  --bg: #f3f7ff;
  --primary: #0b3d91;
  --accent: #f4b400;
  --text: #172033;
  font-family: "Trebuchet MS", "Segoe UI", sans-serif;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: linear-gradient(180deg, #ffffff 0%, var(--bg) 100%);
  color: var(--text);
}

.hero {
  text-align: center;
  padding: 4rem 1.5rem 2rem;
}

.eyebrow {
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--primary);
  font-weight: 700;
}

.subtitle {
  margin: 0.75rem 0 1.5rem;
}

.cta {
  display: inline-block;
  padding: 0.7rem 1.1rem;
  border-radius: 0.5rem;
  background: var(--primary);
  color: #ffffff;
  text-decoration: none;
}

.programs {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1rem;
  max-width: 900px;
  margin: 0 auto 3rem;
  padding: 0 1.5rem;
}

.programs article {
  background: #ffffff;
  border: 1px solid #d7e2ff;
  border-radius: 0.8rem;
  padding: 1rem;
  box-shadow: 0 6px 14px rgba(9, 39, 92, 0.08);
}
CSS

cat > "$ROOT_DIR/script.js" <<'JS'
(function () {
  var statusText = document.getElementById("status-text");
  if (!statusText) {
    return;
  }
  statusText.textContent = "Status: Ready for 2026 admissions | Programs: 3 pillars";
})();
JS

cat > "$RUNTIME_DIR/pane_prepare.sh" <<EOF_PREP
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$ROOT_DIR"

test -d "\$ROOT_DIR"
test -f "\$ROOT_DIR/index.html"
test -f "\$ROOT_DIR/styles.css"
test -f "\$ROOT_DIR/script.js"
printf 'prepare=ok\n'
EOF_PREP

cat > "$RUNTIME_DIR/pane_run.sh" <<EOF_RUN
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$ROOT_DIR"
ASSET_HASH_FILE="$ASSET_HASH_FILE"

rg -q '<title>School Landing Page</title>' "\$ROOT_DIR/index.html"
rg -q 'id="programs" class="programs"' "\$ROOT_DIR/index.html"
rg -q -- '--primary: #0b3d91;' "\$ROOT_DIR/styles.css"
rg -q 'Ready for 2026 admissions' "\$ROOT_DIR/script.js"
sha256sum "\$ROOT_DIR/index.html" "\$ROOT_DIR/styles.css" "\$ROOT_DIR/script.js" > "\$ASSET_HASH_FILE"
printf 'run=ok\n'
EOF_RUN

cat > "$RUNTIME_DIR/pane_finalize.sh" <<EOF_FIN
#!/usr/bin/env bash
set -euo pipefail
ASSET_HASH_FILE="$ASSET_HASH_FILE"

test -f "\$ASSET_HASH_FILE"
rg -q 'index.html' "\$ASSET_HASH_FILE"
rg -q 'styles.css' "\$ASSET_HASH_FILE"
rg -q 'script.js' "\$ASSET_HASH_FILE"
printf 'finalize=ok\n'
EOF_FIN

chmod +x "$RUNTIME_DIR/pane_prepare.sh" "$RUNTIME_DIR/pane_run.sh" "$RUNTIME_DIR/pane_finalize.sh"

cleanup_session

tmux new-session -d -s "$SESSION_NAME" -c "$ROOT_DIR" "bash"
PANE_PREP="$(tmux list-panes -t "$SESSION_NAME" -F '#{pane_id}' | head -n1)"
PANE_RUN="$(tmux split-window -h -P -F '#{pane_id}' -t "$PANE_PREP" -c "$ROOT_DIR" "bash")"
PANE_FIN="$(tmux split-window -v -P -F '#{pane_id}' -t "$PANE_RUN" -c "$ROOT_DIR" "bash")"
tmux select-layout -t "$SESSION_NAME" tiled

record_state "준비"
record_exec "pane_prepare=$PANE_PREP"
record_exec "pane_run=$PANE_RUN"
record_exec "pane_finalize=$PANE_FIN"

run_step() {
  local step_name="$1"
  local pane_id="$2"
  local script_path="$3"
  local pane_log="$LOG_DIR/${step_name}.pane.log"
  local pane_status="$LOG_DIR/${step_name}.status"
  local done_channel="${SESSION_NAME}_${step_name}_done"

  rm -f "$pane_status" "$pane_log"
  record_state "실행:${step_name}"
  record_exec "step=${step_name} pane=${pane_id} event=start"

  if [[ -n "$FORCE_FAIL_STEP" && "$FORCE_FAIL_STEP" == "$step_name" ]]; then
    fail_now "forced_failure:${step_name}"
  fi

  tmux send-keys -t "$pane_id" "bash -lc 'bash \"$script_path\" > \"$pane_log\" 2>&1; code=\$?; printf \"%s\" \"\$code\" > \"$pane_status\"; tmux wait-for -S \"$done_channel\"'" C-m
  tmux wait-for "$done_channel"

  if [[ ! -f "$pane_status" ]]; then
    fail_now "missing_status:${step_name}"
  fi

  local code
  code="$(cat "$pane_status")"
  record_exec "step=${step_name} pane=${pane_id} event=exit code=${code}"
  if [[ "$code" != "0" ]]; then
    fail_now "step_failed:${step_name}"
  fi
}

run_step "01_prepare" "$PANE_PREP" "$RUNTIME_DIR/pane_prepare.sh"
run_step "02_run" "$PANE_RUN" "$RUNTIME_DIR/pane_run.sh"
run_step "03_finalize" "$PANE_FIN" "$RUNTIME_DIR/pane_finalize.sh"

record_state "완료"
printf '%s\n' "완료" > "$STATUS_FILE"
record_exec "result=ok"
cleanup_session
printf 'workspace bootstrap verification complete\n'
