use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};

const AUTO_PLAN_RETRY_COUNT: usize = 2;
const AUTO_FULL_CYCLE_MAX: usize = 3;

#[derive(Clone, Copy)]
enum FallbackStage {
    Plan,
    Draft,
    Verify,
}

impl FallbackStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Draft => "draft",
            Self::Verify => "verify",
        }
    }
}

fn calc_timeout_sec_from_config() -> u64 {
    crate::action_load_app_config()
        .as_ref()
        .map_or(300, crate::config::AppConfig::default_timeout_sec)
        .max(30)
}

fn action_runtime_dir(project_root: &Path) -> Result<PathBuf, String> {
    let runtime = project_root.join(".project").join("runtime");
    fs::create_dir_all(&runtime)
        .map_err(|e| format!("failed to create runtime dir {}: {}", runtime.display(), e))?;
    Ok(runtime)
}

fn action_write_runtime_report(project_root: &Path, name: &str, body: &str) -> Result<PathBuf, String> {
    let runtime = action_runtime_dir(project_root)?;
    let path = runtime.join(name);
    fs::write(&path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok(path)
}

fn action_append_auto_bootstrap_log(project_root: &Path, stage: &str, detail: &str) {
    let Ok(runtime) = action_runtime_dir(project_root) else {
        return;
    };
    let path = runtime.join("auto-bootstrap.log");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(file, "[{}] {} | {}", now, stage, detail);
    }
}

fn action_fallback_stage_output_status(project_root: &Path, stage: FallbackStage) -> String {
    let mut lines = Vec::new();
    match stage {
        FallbackStage::Plan => {
            let project_md = project_root.join(".project").join("project.md");
            let drafts_list = project_root.join(".project").join("drafts_list.yaml");
            let project_md_exists = project_md.exists();
            let drafts_exists = drafts_list.exists();
            lines.push(format!(
                "- `.project/project.md`: {}",
                if project_md_exists { "exists" } else { "missing" }
            ));
            lines.push(format!(
                "- `.project/drafts_list.yaml`: {}",
                if drafts_exists { "exists" } else { "missing" }
            ));
            let likely_ui_only = project_md_exists && drafts_exists;
            if likely_ui_only {
                lines.push("- 판단: 산출물은 생성됨(중단이 아니라 UI/상태 갱신 가능성)".to_string());
            } else {
                lines.push("- 판단: 산출물 미완성 상태에서 중단됨".to_string());
            }
        }
        FallbackStage::Draft => {
            let feature_root = project_root.join(".project").join("feature");
            let clear_root = project_root.join(".project").join("clear");
            let mut generated = 0usize;
            for root in [&feature_root, &clear_root] {
                if let Ok(entries) = fs::read_dir(root) {
                    for entry in entries.flatten() {
                        let feature_dir = entry.path();
                        if feature_dir.join("draft.yaml").exists()
                            || feature_dir.join("drafts.yaml").exists()
                        {
                            generated += 1;
                        }
                    }
                }
            }
            lines.push(format!(
                "- `.project/feature`: {}",
                if feature_root.exists() { "exists" } else { "missing" }
            ));
            lines.push(format!(
                "- `.project/clear`: {}",
                if clear_root.exists() { "exists" } else { "missing" }
            ));
            lines.push(format!("- 생성된 draft 파일 수: {}", generated));
            if generated > 0 {
                lines.push("- 판단: draft 산출물 일부 생성됨(중단이 아니라 상태 갱신 지연 가능성)".to_string());
            } else {
                lines.push("- 판단: draft 산출물 미완성 상태에서 중단됨".to_string());
            }
        }
        FallbackStage::Verify => {
            let (ok, detail) = action_verify_auto_bootstrap_outputs(project_root);
            lines.push(detail);
            lines.push(format!(
                "- 판단: {}",
                if ok {
                    "검증 기준 충족"
                } else {
                    "검증 기준 미충족"
                }
            ));
        }
    }
    lines.join("\n")
}

fn action_append_plan_md_fallback_record(
    project_root: &Path,
    stage: FallbackStage,
    attempt: usize,
    total_attempts: usize,
    error: &str,
    output_status: &str,
) -> Result<PathBuf, String> {
    let path = project_root.join("plan.md");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0);
    let mut body = if path.exists() {
        fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };
    if !body.ends_with('\n') && !body.is_empty() {
        body.push('\n');
    }
    body.push_str(&format!("## fallback {} @{}\n\n", stage.as_str(), now));
    body.push_str("### 문제\n");
    body.push_str(&format!(
        "- stage: {}\n- attempt: {}/{}\n- error: {}\n",
        stage.as_str(),
        attempt,
        total_attempts,
        error
    ));
    body.push_str("### 해결책\n");
    body.push_str(
        "- 현재 단계 산출물 존재 여부를 먼저 확인한다.\n- 산출물 상태를 바탕으로 동일 LLM 단계를 재시도한다(수동 bootstrap/하드코딩 fallback 금지).\n",
    );
    body.push_str("### 검증\n");
    body.push_str(output_status);
    body.push('\n');
    fs::write(&path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok(path)
}

fn action_write_auto_plan_md(project_root: &Path, description: &str, spec: &str) -> Result<PathBuf, String> {
    let path = project_root.join("plan.md");
    if path.exists() {
        return Ok(path);
    }
    let body = format!(
        "# auto execution plan\n\n## 문제\n- 목표: `orc auto`로 앱이 실제 생성되는지 검증\n- 설명: {}\n- 스펙: {}\n\n## 해결책\n- 1) project.md/drafts_list 생성\n- 2) create-draft 실행\n- 3) draft report 생성\n\n## 검증\n- `.project/project.md` 존재\n- `.project/drafts_list.yaml` 존재\n- `.project/feature/*/(draft.yaml|drafts.yaml)` 최소 1개 존재\n\n## 피드백\n- 실행 결과/실패 원인/다음 개선점을 기록\n",
        description.trim(),
        spec.trim()
    );
    fs::write(&path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok(path)
}

fn action_append_plan_retry_note(project_root: &Path, cause: &str, action: &str) {
    let path = project_root.join("plan.md");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0);
    let mut body = fs::read_to_string(&path).unwrap_or_default();
    if !body.ends_with('\n') && !body.is_empty() {
        body.push('\n');
    }
    body.push_str(&format!(
        "\n## retry-note @{}\n- cause: {}\n- applied_action: {}\n",
        now, cause, action
    ));
    let _ = fs::write(path, body);
}

fn action_verify_auto_bootstrap_outputs(project_root: &Path) -> (bool, String) {
    let project_md = project_root.join(".project").join("project.md");
    let drafts_list = project_root.join(".project").join("drafts_list.yaml");
    let feature_root = project_root.join(".project").join("feature");
    let clear_root = project_root.join(".project").join("clear");
    let mut draft_count = 0usize;
    for root in [&feature_root, &clear_root] {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let dir = entry.path();
                if dir.join("draft.yaml").exists() || dir.join("drafts.yaml").exists() {
                    draft_count += 1;
                }
            }
        }
    }
    let app_artifacts = ["package.json", "Cargo.toml", "pyproject.toml", "go.mod"];
    let has_app_artifact = app_artifacts
        .iter()
        .any(|name| project_root.join(name).exists());
    let ok = project_md.exists() && drafts_list.exists() && draft_count > 0 && has_app_artifact;
    let detail = format!(
        "- `.project/project.md`: {}\n- `.project/drafts_list.yaml`: {}\n- draft file count: {}\n- app artifact(package.json/Cargo.toml/pyproject.toml/go.mod): {}",
        if project_md.exists() { "exists" } else { "missing" },
        if drafts_list.exists() { "exists" } else { "missing" },
        draft_count,
        if has_app_artifact { "exists" } else { "missing" }
    );
    (ok, detail)
}

fn action_write_feedback_md(
    project_root: &Path,
    status: &str,
    summary: &str,
    verification: &str,
) -> Result<PathBuf, String> {
    let path = project_root.join("feedback.md");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0);
    let mut body = String::new();
    body.push_str(&format!("# feedback @{}\n\n", now));
    body.push_str(&format!("- status: {}\n- summary: {}\n\n", status, summary));
    body.push_str("## verification\n");
    body.push_str(verification);
    body.push('\n');
    fs::write(&path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok(path)
}

fn calc_auto_cycle_state_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".project")
        .join("runtime")
        .join("auto-cycle.state")
}

fn action_read_auto_cycle(project_root: &Path) -> usize {
    let path = calc_auto_cycle_state_path(project_root);
    fs::read_to_string(path)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

fn action_write_auto_cycle(project_root: &Path, value: usize) {
    let path = calc_auto_cycle_state_path(project_root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, value.to_string());
}

fn action_reset_auto_cycle(project_root: &Path) {
    let path = calc_auto_cycle_state_path(project_root);
    let _ = fs::remove_file(path);
    let snapshot = project_root
        .join(".project")
        .join("runtime")
        .join("last-plan.snapshot");
    let _ = fs::remove_file(snapshot);
}

fn action_preflight_plan_feedback_rules(project_root: &Path, cycle: usize) -> Result<(), String> {
    let plan_path = project_root.join("plan.md");
    if !plan_path.exists() {
        return Err("policy violation: plan.md missing (plan-first rule)".to_string());
    }
    let plan_raw = fs::read_to_string(&plan_path)
        .map_err(|e| format!("policy violation: failed to read {}: {}", plan_path.display(), e))?;
    for section in ["## 문제", "## 해결책", "## 검증"] {
        if !plan_raw.contains(section) {
            return Err(format!(
                "policy violation: plan.md missing required section `{}`",
                section
            ));
        }
    }

    let feedback_path = project_root.join("feedback.md");
    if feedback_path.exists() {
        let feedback_meta = fs::metadata(&feedback_path)
            .and_then(|m| m.modified())
            .map_err(|e| format!("policy violation: failed to stat feedback.md: {}", e))?;
        let plan_meta = fs::metadata(&plan_path)
            .and_then(|m| m.modified())
            .map_err(|e| format!("policy violation: failed to stat plan.md: {}", e))?;
        if plan_meta <= feedback_meta {
            return Err(
                "policy violation: plan.md must be updated after feedback.md (merge rule)".to_string(),
            );
        }
        if !plan_raw.contains("미해결점")
            && !plan_raw.contains("## 실패")
            && !plan_raw.contains("retry-note")
            && !plan_raw.contains("applied_change")
        {
            return Err(
                "policy violation: merged plan must include feedback deltas (미해결점/실패/retry-note/applied_change)"
                    .to_string(),
            );
        }
    }

    let runtime = action_runtime_dir(project_root)?;
    let snapshot_path = runtime.join("last-plan.snapshot");
    if cycle > 1 && snapshot_path.exists() {
        let prev = fs::read_to_string(&snapshot_path).unwrap_or_default();
        if prev == plan_raw {
            return Err(
                "policy violation: retry has no new plan change (forced-resolution rule)".to_string(),
            );
        }
    }
    fs::write(&snapshot_path, plan_raw).map_err(|e| {
        format!(
            "policy violation: failed to write plan snapshot {}: {}",
            snapshot_path.display(),
            e
        )
    })?;
    Ok(())
}

pub(crate) fn auto_mode(project_name: Option<&str>) -> Result<String, String> {
    let registry = crate::action_load_registry(&crate::action_registry_path())?;
    let target = if let Some(name) = project_name {
        registry.projects.iter().find(|p| p.name == name)
    } else {
        registry.projects.iter().find(|p| p.selected)
    }
    .ok_or_else(|| "auto mode requires a selected project".to_string())?;

    let pane_id = crate::tmux::action_current_pane_id().map_err(|_| {
        "auto mode warning: tmux pane is not active. open tmux and retry.".to_string()
    })?;
    crate::tmux::action_rename_pane(&pane_id, "plan")?;

    let project_root = PathBuf::from(&target.path);
    let project_md_path = project_root.join(".project").join("project.md");
    let project_info = fs::read_to_string(&project_md_path).unwrap_or_else(|_| {
        format!(
            "# info\n- name: {}\n- description: {}\n- path: {}",
            target.name, target.description, target.path
        )
    });
    let features = crate::action_collect_project_features(&project_root)?;
    let features_text = if features.is_empty() {
        "- (none)".to_string()
    } else {
        features
            .iter()
            .map(|f| format!("- {}", f))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let auto_prompt = format!(
        "You are in auto mode for project `{}`.\n\
1) Use web search to find similar apps/services for this project context.\n\
2) Propose missing features and pick high-impact items.\n\
3) Create/update drafts under `.project/feature/*/draft.yaml`.\n\
4) Implement all selected features in this repository with minimal safe changes.\n\
5) Run project tests/lint required by project rules.\n\
If a YAML/Markdown file is referenced, read it first, identify headers/properties to fill, then append in the required format.\n\
Output a short action log at the end.\n\n\
Current project info:\n{}\n\nCurrent feature list:\n{}",
        target.name, project_info, features_text
    );
    let _ = crate::action_run_codex_exec_capture_in_dir(&project_root, &auto_prompt)?;

    let _ = crate::action_run_command_in_dir(&project_root, "cargo", &["test"], "cargo test")?;
    let _ = crate::action_run_command_in_dir(
        &project_root,
        "jj",
        &["commit", "-m", "auto mode: feature completion after passing tests"],
        "jj commit",
    )?;
    Ok(format!(
        "auto mode completed: project={} pane={} tests=passed committed=yes",
        target.name, pane_id
    ))
}

pub(crate) fn auto_bootstrap(description: &str, spec: &str) -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let cycle = action_read_auto_cycle(&cwd) + 1;
    action_write_auto_cycle(&cwd, cycle);
    action_preflight_plan_feedback_rules(&cwd, cycle)?;
    action_append_auto_bootstrap_log(&cwd, "start", "auto bootstrap started");
    action_append_auto_bootstrap_log(
        &cwd,
        "cycle",
        &format!("cycle {}/{}", cycle, AUTO_FULL_CYCLE_MAX),
    );
    if let Ok(plan_path) = action_write_auto_plan_md(&cwd, description, spec) {
        action_append_auto_bootstrap_log(&cwd, "plan-doc", &format!("initialized: {}", plan_path.display()));
    }
    let project_name = cwd
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("project")
        .to_string();
    let path_owned = cwd.to_string_lossy().to_string();
    let create_msg =
        create_project_with_defer_option(&project_name, Some(path_owned.as_str()), description, true)?;
    action_append_auto_bootstrap_log(&cwd, "create-project", &create_msg);
    let mut plan_msg = None;
    let mut plan_errors: Vec<String> = Vec::new();
    let total_attempts = AUTO_PLAN_RETRY_COUNT + 1;
    for attempt in 1..=total_attempts {
        action_append_auto_bootstrap_log(
            &cwd,
            "plan-attempt",
            &format!("attempt {}/{} started", attempt, total_attempts),
        );
        match crate::action_generate_project_plan(
            &cwd,
            &project_name,
            description,
            spec,
            description,
            &[],
            "",
            None,
            true,
        ) {
            Ok(msg) => {
                action_append_auto_bootstrap_log(
                    &cwd,
                    "plan-success",
                    &format!("attempt {}/{} success", attempt, total_attempts),
                );
                plan_msg = Some(msg);
                break;
            }
            Err(err) => {
                let output_status = action_fallback_stage_output_status(&cwd, FallbackStage::Plan);
                let plan_path = action_append_plan_md_fallback_record(
                    &cwd,
                    FallbackStage::Plan,
                    attempt,
                    total_attempts,
                    &err,
                    &output_status,
                )
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("plan.md update failed: {}", e));
                action_append_auto_bootstrap_log(
                    &cwd,
                    "plan-failed",
                    &format!("attempt {}/{} failed: {}", attempt, total_attempts, err),
                );
                action_append_auto_bootstrap_log(&cwd, "fallback-output-check", &output_status);
                action_append_auto_bootstrap_log(&cwd, "fallback-plan-update", &plan_path);
                plan_errors.push(format!("attempt {}/{}: {}", attempt, total_attempts, err));
            }
        }
    }
    let plan_msg = if let Some(msg) = plan_msg {
        msg
    } else {
        let mut report = String::new();
        report.push_str("# auto-bootstrap fallback\n\n");
        report.push_str(&format!("- retries: {}\n", total_attempts));
        report.push_str("- plan errors:\n");
        for item in &plan_errors {
            report.push_str(&format!("  - {}\n", item));
        }
        report.push_str("- fallback policy: LLM retry only\n");
        let report_path = action_write_runtime_report(&cwd, "auto-bootstrap-fallback.md", &report)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|e| format!("failed to write fallback report: {}", e));
        action_append_auto_bootstrap_log(&cwd, "fallback-report", &report_path);
        let verification = action_fallback_stage_output_status(&cwd, FallbackStage::Plan);
        let _ = action_write_feedback_md(
            &cwd,
            "fail",
            &format!("plan stage failed after {} attempts", total_attempts),
            &verification,
        );
        if cycle < AUTO_FULL_CYCLE_MAX {
            let _ = action_write_feedback_md(
                &cwd,
                "fail",
                "plan stage failed; restarting full cycle",
                &verification,
            );
            let _ = auto_improve(
                "plan 단계 실패 원인을 해결하고 다음 auto cycle에서 통과될 수 있도록 문서/출력을 보정해.",
            );
            action_append_auto_bootstrap_log(&cwd, "cycle-restart", "reason=plan-failed");
            return auto_bootstrap(description, spec);
        }
        action_reset_auto_cycle(&cwd);
        return Err(format!(
            "auto bootstrap failed after retries: {} | report={}",
            plan_errors.join(" | "),
            report_path
        ));
    };
    let old_cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    env::set_current_dir(&cwd).map_err(|e| format!("failed to enter {}: {}", cwd.display(), e))?;
    let draft_total_attempts = crate::action_load_app_config()
        .as_ref()
        .map_or(2usize, |v| v.llm_retry_count().max(1) as usize);
    let mut draft_msg = None;
    let mut draft_errors = Vec::new();
    for attempt in 1..=draft_total_attempts {
        action_append_auto_bootstrap_log(
            &cwd,
            "draft-attempt",
            &format!("attempt {}/{} started", attempt, draft_total_attempts),
        );
        match crate::draft::draft_create() {
            Ok(msg) => {
                draft_msg = Some(msg);
                action_append_auto_bootstrap_log(
                    &cwd,
                    "draft-success",
                    &format!("attempt {}/{} success", attempt, draft_total_attempts),
                );
                break;
            }
            Err(err) => {
                let output_status = action_fallback_stage_output_status(&cwd, FallbackStage::Draft);
                let plan_path = action_append_plan_md_fallback_record(
                    &cwd,
                    FallbackStage::Draft,
                    attempt,
                    draft_total_attempts,
                    &err,
                    &output_status,
                )
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("plan.md update failed: {}", e));
                action_append_auto_bootstrap_log(
                    &cwd,
                    "draft-failed",
                    &format!("attempt {}/{} failed: {}", attempt, draft_total_attempts, err),
                );
                action_append_auto_bootstrap_log(&cwd, "fallback-output-check", &output_status);
                action_append_auto_bootstrap_log(&cwd, "fallback-plan-update", &plan_path);
                draft_errors.push(format!("attempt {}/{}: {}", attempt, draft_total_attempts, err));
            }
        }
    }
    let draft_msg = if let Some(msg) = draft_msg {
        msg
    } else {
        let report = format!(
            "# draft fallback\n\n- retries: {}\n- errors:\n{}\n",
            draft_total_attempts,
            draft_errors
                .iter()
                .map(|v| format!("  - {}", v))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let report_path = action_write_runtime_report(&cwd, "draft-fallback.md", &report)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|e| format!("failed to write draft fallback report: {}", e));
        let verification = action_fallback_stage_output_status(&cwd, FallbackStage::Draft);
        let _ = action_write_feedback_md(
            &cwd,
            "fail",
            &format!("draft stage failed after {} attempts", draft_total_attempts),
            &verification,
        );
        if cycle < AUTO_FULL_CYCLE_MAX {
            let _ = auto_improve(
                "draft 단계 실패 원인을 해결하고 다음 auto cycle에서 draft 검증을 통과시켜.",
            );
            action_append_auto_bootstrap_log(&cwd, "cycle-restart", "reason=draft-failed");
            return auto_bootstrap(description, spec);
        }
        action_reset_auto_cycle(&cwd);
        return Err(format!(
            "draft-create failed after retries: {} | report={}",
            draft_errors.join(" | "),
            report_path
        ));
    };
    action_append_auto_bootstrap_log(&cwd, "draft-create", &draft_msg);
    let report_msg = draft_report().unwrap_or_else(|e| format!("draft-report failed: {}", e));
    let report_ok = !report_msg.starts_with("draft-report failed:");
    action_append_auto_bootstrap_log(&cwd, "draft-report", &report_msg);
    let (build_ok, build_msg) = match crate::action_run_command_in_dir(
        &cwd,
        "orc",
        &["build-parallel-code"],
        "orc build-parallel-code",
    ) {
        Ok(msg) => (true, msg),
        Err(err) => (false, err),
    };
    action_append_auto_bootstrap_log(&cwd, "build-parallel-code", &build_msg);
    let (base_verify_ok, base_verify_detail) = action_verify_auto_bootstrap_outputs(&cwd);
    let verify_ok = base_verify_ok && report_ok && build_ok;
    let verify_detail = format!(
        "{}\n- draft report: {}\n- build-parallel-code: {}",
        base_verify_detail,
        if report_ok { "pass" } else { "fail" },
        if build_ok { "pass" } else { "fail" }
    );
    action_append_auto_bootstrap_log(
        &cwd,
        "verification",
        &format!("{} | {}", if verify_ok { "pass" } else { "fail" }, verify_detail.replace('\n', " | ")),
    );
    let feedback_path = action_write_feedback_md(
        &cwd,
        if verify_ok { "pass" } else { "fail" },
        "auto bootstrap execution result",
        &verify_detail,
    )
    .map(|p| p.display().to_string())
    .unwrap_or_else(|e| format!("feedback write failed: {}", e));
    action_append_auto_bootstrap_log(&cwd, "feedback", &feedback_path);
    let _ = env::set_current_dir(old_cwd);
    if !verify_ok {
        action_append_plan_retry_note(
            &cwd,
            "verification failed",
            &format!(
                "run build-parallel-code before verify (build_ok={})",
                build_ok
            ),
        );
        let plan_update = action_append_plan_md_fallback_record(
            &cwd,
            FallbackStage::Verify,
            1,
            2,
            "verification failed",
            &verify_detail,
        )
        .map(|p| p.display().to_string())
        .unwrap_or_else(|e| format!("plan.md update failed: {}", e));
        action_append_auto_bootstrap_log(&cwd, "verify-failed", &plan_update);
        let improve_request = format!(
            "검증 실패를 해결해. 실제 앱 산출물 파일(package.json/Cargo.toml/pyproject.toml/go.mod)과 실행 가능한 기본 코드를 생성하고, 검증을 통과시켜."
        );
        let improve_msg =
            auto_improve(&improve_request).unwrap_or_else(|e| format!("auto-improve failed: {}", e));
        action_append_auto_bootstrap_log(&cwd, "auto-improve", &improve_msg);
        let (retry_build_ok, retry_build_msg) = match crate::action_run_command_in_dir(
            &cwd,
            "orc",
            &["build-parallel-code"],
            "orc build-parallel-code(retry)",
        ) {
            Ok(msg) => (true, msg),
            Err(err) => (false, err),
        };
        action_append_auto_bootstrap_log(&cwd, "build-parallel-code-retry", &retry_build_msg);
        let retry_report_msg = draft_report().unwrap_or_else(|e| format!("draft-report failed: {}", e));
        action_append_auto_bootstrap_log(&cwd, "draft-report-retry", &retry_report_msg);
        let retry_report_ok = !retry_report_msg.starts_with("draft-report failed:");
        let (retry_base_ok, retry_base_detail) = action_verify_auto_bootstrap_outputs(&cwd);
        let retry_ok = retry_base_ok && retry_report_ok && retry_build_ok;
        let retry_detail = format!(
            "{}\n- draft report: {}\n- build-parallel-code: {}",
            retry_base_detail,
            if retry_report_ok { "pass" } else { "fail" },
            if retry_build_ok { "pass" } else { "fail" }
        );
        action_append_auto_bootstrap_log(
            &cwd,
            "verification",
            &format!(
                "{} | {}",
                if retry_ok { "pass" } else { "fail" },
                retry_detail.replace('\n', " | ")
            ),
        );
        let retry_feedback_path = action_write_feedback_md(
            &cwd,
            if retry_ok { "pass" } else { "fail" },
            "auto bootstrap retry result",
            &retry_detail,
        )
        .map(|p| p.display().to_string())
        .unwrap_or_else(|e| format!("feedback write failed: {}", e));
        action_append_auto_bootstrap_log(&cwd, "feedback-retry", &retry_feedback_path);
        if !retry_ok {
            if cycle < AUTO_FULL_CYCLE_MAX {
                action_append_auto_bootstrap_log(&cwd, "cycle-restart", "reason=verify-failed");
                return auto_bootstrap(description, spec);
            }
            action_reset_auto_cycle(&cwd);
            return Err(format!(
                "auto bootstrap verification failed(after retry): {} | feedback={}",
                retry_detail.replace('\n', " | "),
                retry_feedback_path
            ));
        }
        action_reset_auto_cycle(&cwd);
        return Ok(format!(
            "auto bootstrap completed(after retry): {} | {} | {} | {} | improve={} | verify=pass | feedback={}",
            create_msg, plan_msg, draft_msg, retry_report_msg, improve_msg, retry_feedback_path
        ));
    }
    action_reset_auto_cycle(&cwd);
    Ok(format!(
        "auto bootstrap completed: {} | {} | {} | {} | verify=pass | feedback={}",
        create_msg, plan_msg, draft_msg, report_msg, feedback_path
    ))
}

pub(crate) fn draft_report() -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let feature_root = cwd.join(".project").join("feature");
    if !feature_root.exists() {
        return Err("draft-report failed: .project/feature not found".to_string());
    }
    let mut feature_names = HashSet::new();
    let mut dep_issues = Vec::new();
    let mut touch_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut task_name_map: HashMap<String, Vec<String>> = HashMap::new();
    let entries = fs::read_dir(&feature_root)
        .map_err(|e| format!("failed to read {}: {}", feature_root.display(), e))?;
    let mut docs: Vec<(String, crate::DraftDoc)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to read file type: {}", e))?;
        if !file_type.is_dir() {
            continue;
        }
        let feature = entry.file_name().to_string_lossy().to_string();
        feature_names.insert(feature.clone());
        let feature_dir = entry.path();
        let draft_path = if feature_dir.join("draft.yaml").exists() {
            feature_dir.join("draft.yaml")
        } else {
            feature_dir.join("draft.yaml")
        };
        if !draft_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&draft_path)
            .map_err(|e| format!("failed to read {}: {}", draft_path.display(), e))?;
        let doc: crate::DraftDoc = serde_yaml::from_str(&raw)
            .map_err(|e| format!("failed to parse {}: {}", draft_path.display(), e))?;
        docs.push((feature, doc));
    }
    for (feature, doc) in &docs {
        for dep in &doc.depends_on {
            if !feature_names.contains(dep) {
                dep_issues.push(format!("- {} depends_on unknown feature `{}`", feature, dep));
            }
        }
        for task in &doc.task {
            task_name_map
                .entry(task.name.clone())
                .or_default()
                .push(feature.clone());
            for touch in &task.touches {
                touch_map
                    .entry(touch.clone())
                    .or_default()
                    .push(feature.clone());
            }
        }
    }
    let duplicate_tasks: Vec<String> = task_name_map
        .iter()
        .filter(|(_, owners)| owners.len() > 1)
        .map(|(name, owners)| format!("- task `{}` duplicated in {:?}", name, owners))
        .collect();
    let touch_conflicts: Vec<String> = touch_map
        .iter()
        .filter(|(_, owners)| owners.len() > 1)
        .map(|(path, owners)| format!("- touch conflict `{}` by {:?}", path, owners))
        .collect();
    let mut report = String::new();
    report.push_str("# draft report\n\n");
    report.push_str(&format!("- features: {}\n", docs.len()));
    report.push_str(&format!("- dependency issues: {}\n", dep_issues.len()));
    report.push_str(&format!("- duplicate tasks: {}\n", duplicate_tasks.len()));
    report.push_str(&format!("- touch conflicts: {}\n\n", touch_conflicts.len()));
    report.push_str("## dependency issues\n");
    if dep_issues.is_empty() {
        report.push_str("- none\n");
    } else {
        report.push_str(&format!("{}\n", dep_issues.join("\n")));
    }
    report.push_str("\n## duplicate tasks\n");
    if duplicate_tasks.is_empty() {
        report.push_str("- none\n");
    } else {
        report.push_str(&format!("{}\n", duplicate_tasks.join("\n")));
    }
    report.push_str("\n## touch conflicts\n");
    if touch_conflicts.is_empty() {
        report.push_str("- none\n");
    } else {
        report.push_str(&format!("{}\n", touch_conflicts.join("\n")));
    }
    let path = action_write_runtime_report(&cwd, "draft-report.md", &report)?;
    Ok(format!("draft-report completed: {}", path.display()))
}

pub(crate) fn auto_check() -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let result = crate::action_run_command_in_dir(
        &cwd,
        "npx",
        &["playwright", "test"],
        "npx playwright test",
    );
    match result {
        Ok(stdout) => {
            let report = format!("# auto-check\n\n- status: pass\n\n```\n{}\n```\n", stdout);
            let path = action_write_runtime_report(&cwd, "auto-check.md", &report)?;
            Ok(format!("auto-check passed: {}", path.display()))
        }
        Err(err) => {
            let report = format!("# auto-check\n\n- status: fail\n\n```\n{}\n```\n", err);
            let path = action_write_runtime_report(&cwd, "auto-check.md", &report)?;
            Err(format!("auto-check failed: {} ({})", err, path.display()))
        }
    }
}

pub(crate) fn auto_improve(request: &str) -> Result<String, String> {
    if request.trim().is_empty() {
        return Err("auto-improve requires non-empty request".to_string());
    }
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let runtime_report = cwd.join(".project").join("runtime").join("auto-check.md");
    let check_report = fs::read_to_string(&runtime_report).unwrap_or_else(|_| "- no check report".to_string());
    let prompt = format!(
        "너는 rust-orc 프로젝트 자동 보완기다.\n\
현재 작업 디렉터리의 소스코드를 직접 수정해 요청 기능이 실제 동작하게 만들어.\n\
지시:\n\
- 최소 변경으로 구현하고, dead path를 만들지 마.\n\
- 수정 후 테스트 기준(Playwright) 실패 원인을 제거해야 한다.\n\
- 파일 수정/추가 후 변경 요약을 간단히 출력해.\n\
- auto 모드이므로 질문/선택지/확인 요청을 절대 출력하지 마.\n\
- 사용자 지시를 기다리는 문장을 출력하지 마.\n\
- 응답 마지막 줄은 반드시 `RESULT: APPLIED` 또는 `RESULT: NO_CHANGE`로 끝내라.\n\n\
사용자 요청:\n{}\n\n\
최근 점검 리포트:\n{}\n",
        request.trim(),
        check_report
    );
    let timeout_sec = calc_timeout_sec_from_config();
    let summary =
        crate::action_run_codex_exec_capture_in_dir_with_timeout(&cwd, &prompt, timeout_sec)?;
    if summary.contains("진행 방법을 선택")
        || summary.contains("진행 방식만 지정")
        || summary.contains("지시해 주세요")
        || summary.contains("선택해 주세요")
        || summary.contains("확인해 주세요")
    {
        return Err(
            "auto-improve produced interactive response in auto mode; retry with non-interactive output"
                .to_string(),
        );
    }
    let normalized = summary.trim_end();
    if !normalized.ends_with("RESULT: APPLIED") && !normalized.ends_with("RESULT: NO_CHANGE") {
        return Err(
            "auto-improve response contract violated: missing terminal RESULT line".to_string(),
        );
    }
    let report = format!("# auto-improve\n\n- request: {}\n\n```\n{}\n```\n", request.trim(), summary);
    let report_path = action_write_runtime_report(&cwd, "auto-improve.md", &report)?;
    Ok(format!("auto-improve completed: {}", report_path.display()))
}

fn create_project_with_defer_option(
    name: &str,
    path: Option<&str>,
    description: &str,
    defer_project_plan: bool,
) -> Result<String, String> {
    let target = path
        .map(PathBuf::from)
        .unwrap_or_else(crate::calc_default_project_path);

    crate::action_ensure_project_dir(&target)?;

    let existing = crate::calc_is_existing_project(&target);
    if !existing {
        fs::create_dir_all(target.join(".project"))
            .map_err(|e| format!("failed to create .project: {}", e))?;
    }
    let registry_path = crate::action_registry_path();
    let registry = crate::action_load_registry(&registry_path)?;
    let upserted = crate::action_upsert_project(&registry, name, &target, description);
    let selected = crate::calc_select_only(&upserted, name);
    crate::action_save_registry(&registry_path, &selected)?;

    let project_md_path = target.join(crate::PROJECT_MD_PATH);
    let mut create_project_plan_msg = String::new();
    if !project_md_path.exists() && !defer_project_plan {
        create_project_plan_msg = crate::action_generate_project_plan(
            &target,
            name,
            description,
            "",
            "",
            &[],
            "",
            None,
            false,
        )?;
    }

    if existing {
        if create_project_plan_msg.is_empty() {
            Ok(format!("loaded existing project: {} ({})", name, target.display()))
        } else {
            Ok(format!(
                "loaded existing project: {} ({}) | {}",
                name,
                target.display(),
                create_project_plan_msg
            ))
        }
    } else if create_project_plan_msg.is_empty() {
        Ok(format!("created project: {} ({})", name, target.display()))
    } else {
        Ok(format!(
            "created project: {} ({}) | {}",
            name,
            target.display(),
            create_project_plan_msg
        ))
    }
}

pub(crate) fn create_project(
    name: &str,
    path: Option<&str>,
    description: &str,
) -> Result<String, String> {
    let defer_project_plan = env::var("ORC_DEFER_PROJECT_PLAN")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false);
    create_project_with_defer_option(name, path, description, defer_project_plan)
}

pub(crate) fn select_project(name: &str) -> Result<String, String> {
    let registry_path = crate::action_registry_path();
    let registry = crate::action_load_registry(&registry_path)?;
    let exists = registry.projects.iter().any(|p| p.name == name);
    if !exists {
        return Err(format!("project not found: {}", name));
    }
    let updated = crate::calc_select_only(&registry, name);
    crate::action_save_registry(&registry_path, &updated)?;
    Ok(format!("selected project: {}", name))
}

pub(crate) fn delete_project(name: &str) -> Result<String, String> {
    let registry_path = crate::action_registry_path();
    let registry = crate::action_load_registry(&registry_path)?;
    let updated = crate::action_delete_project(&registry, name);
    crate::action_save_registry(&registry_path, &updated)?;
    Ok(format!("deleted project: {}", name))
}
