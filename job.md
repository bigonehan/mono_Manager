# plan
- manager session audits `scripts/` usages tied to rc-compatible behavior and replaces eligible shell/python script paths with rc functions plus rc-based call sites.
- manager session must only coordinate worker panes through `orc worker-create`, `orc worker-send`, `orc worker-wait`, and `orc worker-close`.
- impl worker must read `job.md`, identify script-backed flows that belong inside `rc`, implement rc functions/subcommands, and switch callers to rc entrypoints.
- qa/check workers must verify both the new rc command paths and the affected tests/documented workflows.
- improve worker must inspect for leftover script-backed rc-capable paths outside the planned set.

# requirement
## rc_internalize_scripts_usage
- move rc-capable logic out of `scripts/` files into `src/bin/rc.rs` and related rc modules where practical
- keep only minimal wrappers or remove direct script calls when rc command paths can replace them safely
- update call sites, docs, prompts, and tests so rc-capable flows invoke rc instead of raw script bodies
- keep changes minimal and limited to script-backed rc-compatible behavior

# task
## planned
## work
- rc_internalize_scripts_usage
## verify
## complete
- rc_internalize_scripts_usage
## fail

# problems
- none

# check
- impl worker -> `src/bin/rc.rs` and `src/bin/rc/browser.rs` now own `run-playwright-qa` and `check-front-ui-rules`, while `scripts/run_playwright_qa.py` and `scripts/check_front_ui_rules.sh` remain thin wrappers
- qa worker -> `cargo run --quiet --bin rc -- run-playwright-qa --web-root assets/web -- node /tmp/rc-qa-recheck.mjs` succeeded against the live UI and produced `/tmp/rc-qa-recheck.png`
- check worker -> `cargo test --bin rc`, `python3 -m unittest scripts.run_playwright_qa_test`, and `node --test scripts/playwright_safe_helpers.test.mjs` all passed
- check worker -> direct `cargo run --quiet --bin rc -- check-front-ui-rules` and wrapper `bash scripts/check_front_ui_rules.sh` both hit the same existing Playwright design-rules failure, so the wrapper parity is preserved for this scope
- improve worker -> re-read `job.md`, searched remaining direct references to `scripts/run_playwright_qa.py` and `scripts/check_front_ui_rules.sh`, and reported `NO_IMPROVEMENT`
