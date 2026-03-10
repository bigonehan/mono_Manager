use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MODE_LIST: [&str; 4] = ["project", "plan", "draft", "report"];
const CODE_SUBCOMMAND_TIMEOUT_SEC: u64 = 600;
const DRAFT_ITEM_LLM_TIMEOUT_SEC: u64 = 10;
const IMPL_DRAFT_LLM_TIMEOUT_SEC: u64 = 240;
const AUTO_RETRY_MAX: usize = 0;
const AUTO_RETRY_SLEEP_SEC: u64 = 2;
const LONG_WAIT_REPORT_SEC: u64 = 60;

fn manual_web_check_mode_enabled() -> bool {
    match env::var("ORC_WEB_MANUAL_CHECK") {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CodePlanDrafts {
    #[serde(default)]
    planned: Vec<String>,
    #[serde(default)]
    worked: Vec<String>,
    #[serde(default)]
    complete: Vec<String>,
    #[serde(default)]
    error: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CodePlanDoc {
    #[serde(default)]
    goal: String,
    #[serde(default)]
    domains: Vec<String>,
    #[serde(default)]
    drafts: CodePlanDrafts,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DraftItemDoc {
    name: String,
    #[serde(default = "default_draft_item_state")]
    state: String,
    #[serde(default, rename = "type")]
    item_type: String,
    #[serde(default)]
    domain: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    step: Vec<String>,
    #[serde(default)]
    tasks: Vec<String>,
    #[serde(default, rename = "constraints")]
    constraints: Vec<String>,
    #[serde(default)]
    check: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CodeDraftsDoc {
    #[serde(default)]
    draft: Vec<DraftItemDoc>,
}

#[derive(Debug, Clone, Default)]
struct InputFeatureObject {
    name: String,
    rules: Vec<String>,
    steps: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WorkflowEvidenceSnapshot {
    plan_exists: bool,
    plan_planned_len: usize,
    plan_worked_len: usize,
    plan_complete_len: usize,
    plan_error_len: usize,
    drafts_exists: bool,
    draft_items_len: usize,
    drafts_planned_len: usize,
    drafts_worked_len: usize,
    drafts_complete_len: usize,
    drafts_error_len: usize,
}

fn default_draft_item_state() -> String {
    "planned".to_string()
}

fn normalize_draft_item_state(state: &str) -> String {
    match state.trim().to_ascii_lowercase().as_str() {
        "planned" => "planned".to_string(),
        "worked" => "worked".to_string(),
        "complete" => "complete".to_string(),
        "error" => "error".to_string(),
        _ => "planned".to_string(),
    }
}

pub(crate) fn init_code_project(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args);
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let default_name = cwd
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("project")
        .to_string();
    let default_path = cwd
        .canonicalize()
        .unwrap_or(cwd.clone())
        .display()
        .to_string();
    let mut name = opts.name.unwrap_or(default_name);
    let mut description = opts
        .description
        .unwrap_or_else(|| "hello world 출력".to_string());
    let mut spec = opts.spec.unwrap_or_else(|| "next js".to_string());
    let path = opts.path.unwrap_or(default_path);
    if let Some(msg) = opts.message.clone() {
        let inferred = infer_from_message(&msg);
        if !inferred.0.is_empty() {
            name = inferred.0;
        }
        if !inferred.1.is_empty() {
            description = inferred.1;
        }
        if !inferred.2.is_empty() {
            spec = inferred.2;
        }
    }
    if opts.auto && opts.message.is_none() {
        return Err("init_code_project -a requires message (`-a <msg>`)".to_string());
    }
    let current_empty = is_current_dir_empty()?;
    if opts.auto {
        debug_log_auto_stage(
            "project-create",
            if current_empty {
                "empty workspace: create project.md"
            } else {
                "non-empty workspace: load project.md"
            },
        );
    }
    let mut result = if current_empty {
        create_project_md_from_template(&name, &description, &path, &spec)?
    } else {
        load_code_project()?
    };
    ensure_project_md_initialized()?;
    enforce_project_md_primary_path()?;
    ensure_project_memo_initialized()?;
    apply_project_info_overrides(&name, &description, &path, &spec)?;
    let detail_msg = detail_code_project()?;
    enforce_project_md_primary_path()?;
    if opts.auto {
        debug_log_auto_stage("project-detail", "detail_code_project completed");
    }
    let domain_msg = create_code_domain()?;
    enforce_project_md_primary_path()?;
    if opts.auto {
        debug_log_auto_stage("domain-create", "create_code_domain completed");
    }
    if opts.auto {
        debug_log_auto_stage("bootstrap", "bootstrap_code_project start");
    }
    let bootstrap_msg = bootstrap_code_project()?;
    enforce_project_md_primary_path()?;
    if opts.auto {
        debug_log_auto_stage("bootstrap", "bootstrap_code_project completed");
    }
    result = format!(
        "{} | {} | {} | {}",
        result, detail_msg, domain_msg, bootstrap_msg
    );
    Ok(format!("mode={:?} | {}", MODE_LIST, result))
}

pub(crate) fn load_code_project() -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let name = cwd
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("project")
        .to_string();
    let path = cwd
        .canonicalize()
        .unwrap_or(cwd.clone())
        .display()
        .to_string();
    let spec = infer_workspace_spec(&cwd)?;
    let description = "현재 폴더 파일을 기준으로 생성된 프로젝트입니다.".to_string();
    let mut project_md = read_code_template("project.md")?;
    project_md = replace_info_field_value(&project_md, "name", &name);
    project_md = replace_info_field_value(&project_md, "description", &description);
    project_md = replace_info_field_value(&project_md, "path", &path);
    project_md = replace_info_field_value(&project_md, "spec", &spec);
    let feature_items = infer_workspace_features(&cwd)?;
    project_md = replace_markdown_list_section(&project_md, "# features", &feature_items);
    write_project_md(&project_md)?;
    Ok("load_code_project completed".to_string())
}

pub(crate) fn detail_code_project() -> Result<String, String> {
    let path = Path::new(crate::PROJECT_MD_PATH);
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let next = infer_project_detail_with_llm(&raw)?;
    fs::write(path, next).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok("detail_code_project completed".to_string())
}

pub(crate) fn create_code_domain() -> Result<String, String> {
    let path = Path::new(crate::PROJECT_MD_PATH);
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let current_domains: Vec<String> = extract_domains_from_project_md(&raw)
        .into_iter()
        .filter(|d| normalize_feature_key(d) != "name")
        .collect();
    if !current_domains.is_empty() {
        return Ok("create_code_domain skipped: domains already exists".to_string());
    }
    let domain_block = infer_domain_block_with_llm(&raw)?;
    let next = replace_domains_section(&raw, &domain_block);
    fs::write(path, next).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok("create_code_domain completed".to_string())
}

pub(crate) fn bootstrap_code_project() -> Result<String, String> {
    let md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let info = crate::extract_project_info(&md);
    let name = extract_info_value(&info, "name").unwrap_or_else(|| "project".to_string());
    let spec = extract_project_spec_from_md(&md).unwrap_or_else(|| "next js".to_string());
    let status = crate::ui::apply_bootstrap_by_spec(Path::new("."), &name)?;
    let verify = ensure_bootstrap_spec_artifacts(Path::new("."), &spec)?;
    Ok(format!(
        "bootstrap_code_project completed: {} | {} | spec={}",
        status, verify, spec
    ))
}

pub(crate) fn init_code_plan(args: &[String]) -> Result<String, String> {
    let auto = args.iter().any(|v| v == "-a");
    let path = plan_yaml_path()?;
    if path.exists() {
        return Err(format!(
            "init_code_plan can run only once: {} already exists. use add_code_plan for updates",
            path.display()
        ));
    }
    ensure_plan_yaml_initialized()?;
    let project_md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let mut doc = infer_plan_doc_with_llm(&project_md)?;
    sync_plan_doc(&mut doc);
    save_plan_doc(&doc)?;
    debug_log_auto_stage(
        "plan-yaml",
        &format!(
            "plan.yaml generated: domains={} planned={} worked={} complete={} error={}",
            doc.domains.len(),
            doc.drafts.planned.len(),
            doc.drafts.worked.len(),
            doc.drafts.complete.len(),
            doc.drafts.error.len()
        ),
    );
    let out = format!(
        "init_code_plan completed: domains={} planned={}",
        doc.domains.len(),
        doc.drafts.planned.len()
    );
    let _ = auto;
    Ok(out)
}

pub(crate) fn add_code_plan(args: &[String]) -> Result<String, String> {
    let mut use_file = false;
    let mut auto = false;
    let mut message: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-f" => use_file = true,
            "-a" => auto = true,
            "-m" => {
                i += 1;
                message = args.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }
    let mut doc = load_plan_doc()?;
    sync_plan_doc(&mut doc);
    let mut items = Vec::new();
    if auto {
        items.extend(infer_plan_items_with_llm()?);
    }
    if use_file {
        let objs = parse_input_md_objects(Path::new(crate::INPUT_MD_PATH))?;
        items.extend(objs.into_iter().map(|o| o.name));
    }
    if let Some(msg) = message {
        items.push(msg);
    }
    if items.is_empty() {
        return Err("add_code_plan requires -f or -m or -a".to_string());
    }
    for item in items {
        let key = normalize_feature_key(&item);
        if key.is_empty()
            || doc.drafts.planned.iter().any(|v| v == &key)
            || doc.drafts.worked.iter().any(|v| v == &key)
            || doc.drafts.complete.iter().any(|v| v == &key)
            || doc.drafts.error.iter().any(|v| v == &key)
        {
            continue;
        }
        doc.drafts.planned.push(key);
    }
    sync_plan_doc(&mut doc);
    save_plan_doc(&doc)?;
    let mut out = format!(
        "add_code_plan completed: planned={}",
        doc.drafts.planned.len()
    );
    if auto {
        return Ok(out);
    }
    if !use_file && ask_yes_no("add_code_draft()를 호출할까요? [y/N]: ")? {
        let draft_msg = run_code_subcommand_in_new_session("add_code_draft", &[])?;
        out = format!("{} | {}", out, draft_msg);
    }
    Ok(out)
}

pub(crate) fn create_code_draft() -> Result<String, String> {
    add_code_draft(&[])
}

pub(crate) fn create_input_md() -> Result<String, String> {
    append_check_process_status("create_input_md", "start");
    build_input_md_auto()
}

pub(crate) fn add_code_draft(args: &[String]) -> Result<String, String> {
    let mut use_file = false;
    let mut auto = false;
    let mut message: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-f" => use_file = true,
            "-a" => auto = true,
            "-m" => {
                i += 1;
                message = args.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }
    if auto {
        use_file = true;
        let build_msg = build_input_md_auto()?;
        debug_log_auto_stage("input-md", &build_msg);
        append_check_process_status(
            "add_code_draft",
            &format!("auto input generated | {}", build_msg),
        );
    }

    append_check_process_status(
        "add_code_draft",
        &format!(
            "start | mode={}{}",
            if use_file { "file" } else { "llm" },
            if message.is_some() { " | message" } else { "" }
        ),
    );
    ensure_drafts_yaml_initialized()?;
    append_check_process_status("add_code_draft", "drafts.yaml initialized");
    let mut plan = load_plan_doc()?;
    sync_plan_doc(&mut plan);
    let mut drafts = load_drafts_doc()?;
    let mut plan_items = plan.drafts.planned.clone();
    append_check_process_status(
        "add_code_draft",
        &format!(
            "plan/drafts loaded | planned={} draft_items={}",
            plan_items.len(),
            drafts.draft.len()
        ),
    );

    append_check_process_status("add_code_draft", "read project.md for draft inference");
    let project_md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    append_check_process_status("add_code_draft", "read plan.yaml for draft inference");
    let plan_yaml_raw = {
        let path = plan_yaml_path()?;
        fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?
    };

    let input_objects = if use_file {
        append_check_process_status("add_code_draft", "parse input.md objects");
        parse_input_md_objects(Path::new(crate::INPUT_MD_PATH))?
    } else {
        Vec::new()
    };
    if use_file && plan_items.is_empty() {
        for obj in &input_objects {
            let name = normalize_feature_key(&obj.name);
            if name.is_empty() || plan_items.iter().any(|v| v == &name) {
                continue;
            }
            plan_items.push(name.clone());
            if !plan.drafts.planned.iter().any(|v| v == &name) {
                plan.drafts.planned.push(name.clone());
            }
        }
        sync_plan_doc(&mut plan);
        save_plan_doc(&plan)?;
        append_check_process_status(
            "add_code_draft",
            &format!(
                "plan synced from input.md | planned={}",
                plan.drafts.planned.len()
            ),
        );
    }

    for name in &plan_items {
        if drafts.draft.iter().any(|v| v.name == *name) {
            continue;
        }
        append_check_process_status("add_code_draft", &format!("infer draft item | {}", name));
        let from_input = input_objects
            .iter()
            .find(|v| normalize_feature_key(&v.name) == *name);
        let inferred = infer_draft_item_with_llm(&project_md, &plan_yaml_raw, name, from_input);
        drafts.draft.push(DraftItemDoc {
            state: default_draft_item_state(),
            ..inferred
        });
        plan_draft_state_change(&mut plan, name, "planned")?;
    }

    if let Some(msg) = message {
        let name = normalize_feature_key(&msg);
        if !name.is_empty() && !drafts.draft.iter().any(|v| v.name == name) {
            append_check_process_status(
                "add_code_draft",
                &format!("infer message draft item | {}", name),
            );
            let inferred = infer_draft_item_with_llm(&project_md, &plan_yaml_raw, &name, None);
            drafts.draft.push(DraftItemDoc {
                state: default_draft_item_state(),
                ..inferred
            });
            plan_draft_state_change(&mut plan, &name, "planned")?;
        }
    }

    sync_drafts_doc(&mut drafts);
    sync_plan_with_draft_items(&mut plan, &drafts);
    save_plan_doc(&plan)?;
    save_drafts_doc(&drafts)?;
    append_check_process_status(
        "add_code_draft",
        &format!("artifacts saved | draft_items={}", drafts.draft.len()),
    );
    debug_log_auto_stage(
        "draft-yaml",
        &format!("drafts.yaml generated: draft={}", drafts.draft.len()),
    );
    append_check_process_status("add_code_draft", "run check_code_draft");
    let check = check_code_draft(false)?;
    debug_log_auto_stage("draft-yaml", "drafts.yaml checked");
    append_check_process_status("add_code_draft", "completed");
    let out = format!(
        "add_code_draft completed: draft={} | {}",
        drafts.draft.len(),
        check
    );
    let _ = auto;
    Ok(out)
}

pub(crate) fn add_code_draft_item(args: &[String]) -> Result<String, String> {
    add_code_draft(args)
}

pub(crate) fn auto_code_message(message: &str) -> Result<String, String> {
    debug_log_auto_stage("auto", "auto message flow start");
    let trimmed = message.trim();
    match run_code_subcommand_in_new_session("init_code_project", &["-a", trimmed]) {
        Ok(init_out) => {
            let loop_out = run_auto_retry_loop("auto", Some(trimmed), false)?;
            debug_log_auto_stage("auto", "auto message flow completed");
            Ok(format!("{} | {}", init_out, loop_out))
        }
        Err(err) => {
            write_feedback_md("auto_code_message failed", &err)?;
            Err(format!(
                "auto flow failed; check .project/check-process.md: {}",
                err
            ))
        }
    }
}

pub(crate) fn auto_add_function_message(message: &str) -> Result<String, String> {
    debug_log_auto_stage("auto-add", "auto_add_function flow start");
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err("auto_add_function requires non-empty message".to_string());
    }
    let init_out = run_code_subcommand_in_new_session("init_code_project", &["-a", trimmed])?;
    let retry_out = run_auto_retry_loop("auto_add_function", Some(trimmed), false)?;
    let plan_md_out = write_plan_md_snapshot(trimmed)?;
    let improve_prompt = format!(
        "프로젝트 루트에서 다음 작업을 수행해.\n\
요청: {}\n\
현재 구현 결과를 점검하고 개선이 필요한 항목이 있으면 직접 수정해.\n\
수정 후 `check_code_draft -a`가 통과하도록 정리해.\n\
출력 형식:\n\
- improve_summary: <요약>\n\
- actions: <수행한 작업>\n",
        trimmed
    );
    let improve_out = crate::run_codex_exec_capture_with_timeout(&improve_prompt, 600)
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|e| format!("improve stage skipped: {}", e));
    let post_retry_out =
        run_auto_retry_loop("auto_add_function-post-improve", Some(trimmed), false)?;
    let final_check = check_code_draft(true)?;
    debug_log_auto_stage("auto-add", "auto_add_function flow completed");
    Ok(format!(
        "auto_add_function completed: {} | {} | {} | improve={} | {} | {}",
        init_out, retry_out, plan_md_out, improve_out, post_retry_out, final_check
    ))
}

pub(crate) fn auto_code_from_input_file() -> Result<String, String> {
    debug_log_auto_stage("auto-file", "auto -f flow start");
    let init_msg = run_code_subcommand_in_new_session("init_code_project", &[])?;
    let plan_msg = if plan_yaml_path()?.exists() {
        "init_code_plan skipped: .project/plan.yaml already exists".to_string()
    } else {
        run_code_subcommand_in_new_session("init_code_plan", &["-a"])?
    };
    let input_msg = run_code_subcommand_in_new_session("create_input_md", &[])?;
    let retry_msg = run_auto_retry_loop("auto -f", None, true)?;
    debug_log_auto_stage("auto-file", "auto -f flow completed");

    Ok(format!(
        "auto -f completed: {} | {} | {} | {}",
        init_msg, plan_msg, input_msg, retry_msg
    ))
}

fn write_plan_md_snapshot(message: &str) -> Result<String, String> {
    let plan = load_plan_doc()?;
    let planned = if plan.drafts.planned.is_empty() {
        "- (none)".to_string()
    } else {
        plan.drafts
            .planned
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let worked = if plan.drafts.worked.is_empty() {
        "- (none)".to_string()
    } else {
        plan.drafts
            .worked
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let complete = if plan.drafts.complete.is_empty() {
        "- (none)".to_string()
    } else {
        plan.drafts
            .complete
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let body = format!(
        "# plan.md\n\n## request\n- {}\n\n## planned\n{}\n\n## worked\n{}\n\n## complete\n{}\n\n## error\n{}\n",
        message.trim(),
        planned,
        worked,
        complete,
        if plan.drafts.error.is_empty() {
            "- (none)".to_string()
        } else {
            plan.drafts
                .error
                .iter()
                .map(|v| format!("- {}", v))
                .collect::<Vec<_>>()
                .join("\n")
        }
    );
    fs::write("plan.md", body).map_err(|e| format!("failed to write plan.md: {}", e))?;
    Ok("plan.md updated".to_string())
}

fn auto_retry_max() -> usize {
    env::var("ORC_AUTO_RETRY_MAX")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(AUTO_RETRY_MAX)
}

fn auto_retry_sleep_sec() -> u64 {
    env::var("ORC_AUTO_RETRY_SLEEP_SEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(AUTO_RETRY_SLEEP_SEC)
}

fn drafts_state_counts() -> Result<(usize, usize, usize, usize), String> {
    let mut drafts = load_drafts_doc()?;
    sync_drafts_doc(&mut drafts);
    Ok((
        drafts.draft.iter().filter(|v| v.state == "planned").count(),
        drafts.draft.iter().filter(|v| v.state == "worked").count(),
        drafts
            .draft
            .iter()
            .filter(|v| v.state == "complete")
            .count(),
        drafts.draft.iter().filter(|v| v.state == "error").count(),
    ))
}

fn drafts_state_summary() -> String {
    match drafts_state_counts() {
        Ok((planned, worked, complete, error)) => {
            format!(
                "planned={} worked={} complete={} error={}",
                planned, worked, complete, error
            )
        }
        Err(e) => format!("draft-state-unavailable: {}", e),
    }
}

fn drafts_resolved() -> bool {
    match drafts_state_counts() {
        Ok((planned, worked, complete, error)) => {
            planned == 0 && worked == 0 && error == 0 && complete > 0
        }
        Err(_) => false,
    }
}

fn run_auto_retry_loop(
    mode: &str,
    message: Option<&str>,
    from_file: bool,
) -> Result<String, String> {
    let max_retry = auto_retry_max();
    let sleep_sec = auto_retry_sleep_sec();
    let mut attempt: usize = 0;

    loop {
        attempt += 1;
        let retry_input = if from_file {
            crate::INPUT_MD_PATH.to_string()
        } else {
            message.unwrap_or("(empty)").to_string()
        };
        append_check_process_retry(
            mode,
            &retry_input,
            &format!("attempt={} from_file={}", attempt, from_file),
        );
        if max_retry > 0 && attempt > max_retry {
            let detail = format!(
                "{} unresolved after {} attempt(s) | {}",
                mode,
                max_retry,
                drafts_state_summary()
            );
            write_feedback_md("auto retry unresolved", &detail)?;
            return Err(detail);
        }

        let mut stage_log: Vec<String> = Vec::new();
        stage_log.push(format!("attempt={}", attempt));

        if from_file {
            match run_code_subcommand_in_new_session("create_input_md", &[]) {
                Ok(msg) => stage_log.push(msg),
                Err(e) => stage_log.push(format!("create_input_md failed: {}", e)),
            }
            match run_code_subcommand_in_new_session("add_code_plan", &["-f"]) {
                Ok(msg) => stage_log.push(msg),
                Err(e) => stage_log.push(format!("add_code_plan failed: {}", e)),
            }
            match run_code_subcommand_in_new_session("add_code_draft", &["-f"]) {
                Ok(msg) => stage_log.push(msg),
                Err(e) => stage_log.push(format!("add_code_draft failed: {}", e)),
            }
        } else if let Some(msg) = message {
            match run_code_subcommand_in_new_session("create_input_md", &[]) {
                Ok(out) => stage_log.push(out),
                Err(e) => stage_log.push(format!("create_input_md failed: {}", e)),
            }
            match run_code_subcommand_in_new_session("add_code_plan", &["-m", msg]) {
                Ok(out) => stage_log.push(out),
                Err(e) => stage_log.push(format!("add_code_plan failed: {}", e)),
            }
            match run_code_subcommand_in_new_session("add_code_draft", &["-m", msg]) {
                Ok(out) => stage_log.push(out),
                Err(e) => stage_log.push(format!("add_code_draft failed: {}", e)),
            }
        }

        let impl_status = match run_code_subcommand_in_new_session("impl_code_draft", &[]) {
            Ok(msg) => {
                stage_log.push(msg.clone());
                true
            }
            Err(e) => {
                stage_log.push(format!("impl_code_draft failed: {}", e));
                false
            }
        };
        let check_status = match run_code_subcommand_in_new_session("check_code_draft", &["-a"]) {
            Ok(msg) => {
                stage_log.push(msg.clone());
                true
            }
            Err(e) => {
                stage_log.push(format!("check_code_draft failed: {}", e));
                false
            }
        };
        let state = drafts_state_summary();

        if impl_status && check_status && drafts_resolved() {
            return Ok(format!(
                "{} retry completed in {} attempt(s) | {} | {}",
                mode,
                attempt,
                stage_log.join(" | "),
                state
            ));
        }

        let detail = format!(
            "{} retry unresolved (attempt={}) | {} | {}",
            mode,
            attempt,
            stage_log.join(" | "),
            state
        );
        append_check_process_status(mode, &detail);
        let _ = write_feedback_md("auto retry unresolved", &detail);
        if sleep_sec > 0 {
            thread::sleep(Duration::from_secs(sleep_sec));
        }
    }
}

pub(crate) async fn impl_code_draft() -> Result<String, String> {
    let mut plan = load_plan_doc()?;
    sync_plan_doc(&mut plan);
    let mut drafts = load_drafts_doc()?;
    sync_drafts_doc(&mut drafts);
    sync_plan_with_draft_items(&mut plan, &drafts);
    save_plan_doc(&plan)?;
    save_drafts_doc(&drafts)?;

    let planned_names: Vec<String> = drafts
        .draft
        .iter()
        .filter(|item| item.state == "planned")
        .map(|item| item.name.clone())
        .collect();
    if drafts.draft.is_empty() || planned_names.is_empty() {
        return Ok("impl_code_draft skipped: no drafts.yaml.planned item".to_string());
    }

    for name in &planned_names {
        draft_item_state_change(&mut drafts, name, "worked")?;
        plan_draft_state_change(&mut plan, name, "worked")?;
    }
    sync_plan_doc(&mut plan);
    sync_drafts_doc(&mut drafts);
    save_plan_doc(&plan)?;
    save_drafts_doc(&drafts)?;

    if let Some(fast_path) =
        try_complete_impl_from_existing_project(&mut plan, &mut drafts, &planned_names)?
    {
        if manual_web_check_mode_enabled() {
            append_check_process_status(
                "impl_code_draft",
                "manual web check pending after fast-path implementation",
            );
            return Ok(format!(
                "impl_code_draft completed | {} | manual rc check pending",
                fast_path
            ));
        }
        let check = check_code_draft(true)?;
        return Ok(format!(
            "impl_code_draft completed | {} | {}",
            fast_path, check
        ));
    }

    let worked_items: Vec<DraftItemDoc> = plan
        .drafts
        .worked
        .iter()
        .filter_map(|name| drafts.draft.iter().find(|v| &v.name == name).cloned())
        .collect();
    debug_log_auto_stage(
        "parallel-start",
        &format!("parallel execution start: {} item(s)", worked_items.len()),
    );
    let run_msg = match impl_code_draft_parallel(worked_items).await {
        Ok(run) => {
            for name in &run.succeeded {
                draft_item_state_change(&mut drafts, name, "complete")?;
                plan_draft_state_change(&mut plan, name, "complete")?;
            }
            for (name, _) in &run.failed {
                draft_item_state_change(&mut drafts, name, "error")?;
                plan_draft_state_change(&mut plan, name, "error")?;
            }
            sync_plan_doc(&mut plan);
            sync_drafts_doc(&mut drafts);
            save_plan_doc(&plan)?;
            save_drafts_doc(&drafts)?;
            if run.failed.is_empty() {
                format!(
                    "impl_code_draft parallel completed: {}",
                    run.succeeded.join(", ")
                )
            } else {
                let msg = format!(
                    "partial success: succeeded=[{}], failed=[{}]",
                    run.succeeded.join(", "),
                    run.failed
                        .iter()
                        .map(|(name, detail)| format!("{}: {}", name, detail))
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                write_feedback_md("impl_code_draft partial failure", &msg)?;
                return Err(format!("impl_code_draft failed: {}", msg));
            }
        }
        Err(e) => {
            let current_worked: Vec<String> = drafts
                .draft
                .iter()
                .filter(|item| item.state == "worked")
                .map(|item| item.name.clone())
                .collect();
            for name in current_worked {
                let _ = draft_item_state_change(&mut drafts, &name, "error");
                let _ = plan_draft_state_change(&mut plan, &name, "error");
            }
            sync_plan_doc(&mut plan);
            sync_drafts_doc(&mut drafts);
            let _ = save_plan_doc(&plan);
            let _ = save_drafts_doc(&drafts);
            write_feedback_md("impl_code_draft failed", &e)?;
            return Err(format!(
                "impl_code_draft failed after sync; check .project/check-process.md: {}",
                e
            ));
        }
    };

    if manual_web_check_mode_enabled() {
        append_check_process_status(
            "impl_code_draft",
            "manual web check pending after parallel implementation",
        );
        return Ok(format!(
            "impl_code_draft completed | {} | manual rc check pending",
            run_msg
        ));
    }
    let check = check_code_draft(true)?;
    append_check_process_status(
        "impl_code_draft",
        &format!("artifact verification completed | {}", check),
    );
    Ok(format!(
        "impl_code_draft completed | {} | {}",
        run_msg, check
    ))
}

fn try_complete_impl_from_existing_project(
    plan: &mut CodePlanDoc,
    drafts: &mut CodeDraftsDoc,
    worked_names: &[String],
) -> Result<Option<String>, String> {
    if worked_names.is_empty() {
        return Ok(None);
    }
    let test_out = crate::test_command()?;
    let lowered = test_out.to_ascii_lowercase();
    if lowered.contains("fail") || lowered.contains("error") || lowered.contains("skipped") {
        return Ok(None);
    }
    for name in worked_names {
        draft_item_state_change(drafts, name, "complete")?;
        plan_draft_state_change(plan, name, "complete")?;
    }
    sync_plan_doc(plan);
    sync_drafts_doc(drafts);
    save_plan_doc(plan)?;
    save_drafts_doc(drafts)?;
    Ok(Some(format!(
        "impl_code_draft fast-path completed from existing project validation: {}",
        test_out
    )))
}

struct ImplRunResult {
    succeeded: Vec<String>,
    failed: Vec<(String, String)>,
}

async fn impl_code_draft_parallel(items: Vec<DraftItemDoc>) -> Result<ImplRunResult, String> {
    if items.is_empty() {
        return Ok(ImplRunResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        });
    }
    let prompt_path = Path::new("assets")
        .join("code")
        .join("prompts")
        .join("impl_code_draft.txt");
    let prompt_template = fs::read_to_string(&prompt_path).unwrap_or_else(|_| {
        "impl_code_draft prompt\n- draft_item을 구현하고 제약 만족 여부를 보고한다.".to_string()
    });
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
    let llm_timeout_sec = impl_draft_llm_timeout_sec();
    let mut handles = Vec::new();
    for item in items {
        let permit_pool = semaphore.clone();
        let prompt_template = prompt_template.clone();
        handles.push(tokio::spawn(async move {
            let _permit = permit_pool
                .acquire_owned()
                .await
                .map_err(|e| format!("semaphore acquire failed: {}", e))?;
            let raw = serde_yaml::to_string(&item)
                .map_err(|e| format!("failed to encode draft item {}: {}", item.name, e))?;
            let prompt = format!(
                "{}\n\n```yaml\n{}\n```\n\n위 draft_item을 구현하고 constraints 만족 여부를 마지막 줄에 `constraints: ok|fail`로 출력한다.",
                prompt_template, raw
            );
            let name = item.name.clone();
            let output = tokio::task::spawn_blocking(move || {
                crate::run_codex_exec_capture_with_timeout(&prompt, llm_timeout_sec)
            })
                .await
                .map_err(|e| format!("spawn blocking join failed for {}: {}", name, e))??;
            let tail = output.lines().last().unwrap_or("").to_ascii_lowercase();
            if tail.contains("constraints: fail") {
                return Err(format!("{}: constraints reported fail", item.name));
            }
            Ok::<String, String>(item.name)
        }));
    }
    let mut done = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    for handle in handles {
        match handle
            .await
            .map_err(|e| format!("parallel task join failed: {}", e))?
        {
            Ok(name) => done.push(name),
            Err(err) => {
                let name = err
                    .split_once(':')
                    .map(|(left, _)| left.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| "unknown".to_string());
                failed.push((name, err));
            }
        }
    }
    Ok(ImplRunResult {
        succeeded: done,
        failed,
    })
}

fn write_feedback_md(summary: &str, detail: &str) -> Result<(), String> {
    crate::append_check_process_status(summary, detail)
}

fn run_impl_code_draft_via_cli() -> Result<String, String> {
    run_code_subcommand_in_new_session("impl_code_draft", &[])
}

fn run_code_subcommand_in_new_session(command: &str, args: &[&str]) -> Result<String, String> {
    if should_use_tmux_worker_pane() {
        return run_code_subcommand_via_tmux_pane(command, args);
    }
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    emit_command_start_status(None, "session", command, args);
    let before = capture_workflow_evidence().ok();
    debug_log_auto_stage(
        "session",
        &format!("new session start: {} {}", command, args.join(" ")),
    );
    let mut cmd = Command::new(exe);
    cmd.arg(command);
    for arg in args {
        cmd.arg(arg);
    }
    if let Ok(task_session) = env::var("ORC_TASK_SESSION_KEY") {
        cmd.env("ORC_TASK_SESSION_KEY", task_session);
    }
    let mut child = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to execute {}: {}", command, e))?;
    let timeout_sec = code_subcommand_timeout_sec();
    let started = Instant::now();
    let mut next_report_sec = LONG_WAIT_REPORT_SEC;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                let elapsed_sec = started.elapsed().as_secs();
                if elapsed_sec >= next_report_sec {
                    report_long_wait_status(
                        None,
                        "session",
                        command,
                        elapsed_sec,
                        "child process still running; waiting for command exit",
                    );
                    next_report_sec += LONG_WAIT_REPORT_SEC;
                }
                if started.elapsed() >= Duration::from_secs(timeout_sec) {
                    let _ = child.kill();
                    let _ = child.wait();
                    debug_log_auto_stage(
                        "session",
                        &format!(
                            "new session timeout: {} | timeout={}s",
                            command, timeout_sec
                        ),
                    );
                    return Err(format!(
                        "{} failed: timeout after {}s",
                        command, timeout_sec
                    ));
                }
                thread::sleep(Duration::from_millis(250));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed while waiting {}: {}", command, e));
            }
        }
    };
    if status.success() {
        verify_command_artifacts(command, before.as_ref())?;
        debug_log_auto_stage("session", &format!("new session completed: {}", command));
        Ok(format!("{} completed", command))
    } else {
        debug_log_auto_stage(
            "session",
            &format!("new session failed: {} | code={:?}", command, status.code()),
        );
        Err(format!("{} failed: code={:?}", command, status.code()))
    }
}

fn should_use_tmux_worker_pane() -> bool {
    if !env_flag_true("ORC_USE_TMUX_PANES") {
        return false;
    }
    env::var("TMUX")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

fn env_flag_true(name: &str) -> bool {
    match env::var(name) {
        Ok(v) => {
            let lowered = v.trim().to_ascii_lowercase();
            lowered == "1" || lowered == "true" || lowered == "yes" || lowered == "on"
        }
        Err(_) => false,
    }
}

fn trim_wait_detail(detail: &str) -> String {
    let normalized = detail.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.len() <= 160 {
        return normalized;
    }
    format!("{}...", &normalized[..157])
}

fn read_last_non_empty_line(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    raw.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(trim_wait_detail)
}

fn report_long_wait_status(
    parent_pane: Option<&str>,
    scope: &str,
    command: &str,
    elapsed_sec: u64,
    detail: &str,
) {
    let status = format!(
        "[orc-status] scope={} command={} elapsed={}s detail={}",
        scope,
        command,
        elapsed_sec,
        trim_wait_detail(detail)
    );
    eprintln!("{}", status);
    debug_log_auto_stage("wait", &status);
    if let Some(parent) = parent_pane {
        let _ = crate::tmux::display_message(parent, &status);
    }
}

fn append_check_process_status(stage: &str, detail: &str) {
    let _ = crate::append_check_process_status(stage, detail);
}

fn append_check_process_retry(mode: &str, input: &str, detail: &str) {
    let _ = crate::append_check_process_retry(mode, input, detail);
}

fn current_exe_process_detail() -> String {
    crate::binary_context_summary()
}

fn emit_command_start_status(parent_pane: Option<&str>, scope: &str, command: &str, args: &[&str]) {
    let status = format!(
        "[orc-start] scope={} command={} args={} {}",
        scope,
        command,
        if args.is_empty() {
            "(none)".to_string()
        } else {
            args.join(" ")
        },
        current_exe_process_detail()
    );
    eprintln!("{}", status);
    append_check_process_status(scope, &status);
    if let Some(parent) = parent_pane {
        let _ = crate::tmux::display_message(parent, &status);
    }
}

fn quote_sh(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn run_code_subcommand_via_tmux_pane(command: &str, args: &[&str]) -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let parent_pane = crate::tmux::current_pane_id().ok();
    let task_session = env::var("ORC_TASK_SESSION_KEY").ok();
    emit_command_start_status(parent_pane.as_deref(), "tmux-worker", command, args);
    let before = capture_workflow_evidence().ok();
    let runtime = Path::new(".project").join("runtime");
    fs::create_dir_all(&runtime)
        .map_err(|e| format!("failed to create {}: {}", runtime.display(), e))?;
    let token = format!(
        "{}-{}-{}",
        now_unix_ts(),
        std::process::id(),
        normalize_feature_key(command)
    );
    let script_path = runtime.join(format!("tmux-subcmd-{}.sh", token));
    let stdout_path = runtime.join(format!("tmux-subcmd-{}.stdout.log", token));
    let stderr_path = runtime.join(format!("tmux-subcmd-{}.stderr.log", token));
    let code_path = runtime.join(format!("tmux-subcmd-{}.code", token));

    let mut quoted_args = Vec::new();
    for arg in args {
        quoted_args.push(quote_sh(arg));
    }
    let args_joined = if quoted_args.is_empty() {
        String::new()
    } else {
        format!(" {}", quoted_args.join(" "))
    };
    let script = format!(
        "#!/usr/bin/env bash\n\
set +e\n\
cd {cwd}\n\
echo \"[orc-worker] start: {command}{args}\"\n\
echo \"[orc-worker] cwd: {cwd_display}\"\n\
{task_session_export}\
{exe} {command}{args} > >(tee {stdout}) 2> >(tee {stderr} >&2)\n\
status=$?\n\
printf \"%s\" \"$status\" > {code}\n",
        cwd = quote_sh(&cwd.display().to_string()),
        cwd_display = cwd.display(),
        task_session_export = task_session
            .as_deref()
            .map(|value| format!("export ORC_TASK_SESSION_KEY={}\n", quote_sh(value)))
            .unwrap_or_default(),
        exe = quote_sh(&exe.display().to_string()),
        command = quote_sh(command),
        args = args_joined,
        stdout = quote_sh(&stdout_path.display().to_string()),
        stderr = quote_sh(&stderr_path.display().to_string()),
        code = quote_sh(&code_path.display().to_string()),
    );
    fs::write(&script_path, script)
        .map_err(|e| format!("failed to write {}: {}", script_path.display(), e))?;
    debug_log_auto_stage(
        "session",
        &format!("worker pane start: {} {}", command, args.join(" ")),
    );
    let pane_id = crate::tmux::split_window_run(&format!(
        "bash {}",
        quote_sh(&script_path.display().to_string())
    ))
    .map_err(|e| format!("failed to spawn tmux worker pane: {}", e))?;
    let worker = crate::tmux::register_worker_pane(&pane_id);
    let _ = crate::tmux::rename_pane(
        &worker.pane_id,
        &format!("orc-{}-{}", command, worker.short_id()),
    );
    let timeout_sec = code_subcommand_timeout_sec();
    let started = Instant::now();
    let mut next_report_sec = LONG_WAIT_REPORT_SEC;
    while !code_path.exists() {
        let elapsed_sec = started.elapsed().as_secs();
        if elapsed_sec >= next_report_sec {
            let latest = read_last_non_empty_line(&stderr_path)
                .or_else(|| read_last_non_empty_line(&stdout_path))
                .unwrap_or_else(|| "worker pane has no output yet".to_string());
            report_long_wait_status(
                parent_pane.as_deref(),
                "tmux-worker",
                command,
                elapsed_sec,
                &latest,
            );
            next_report_sec += LONG_WAIT_REPORT_SEC;
        }
        if started.elapsed() >= Duration::from_secs(timeout_sec) {
            let _ = crate::tmux::display_message(
                parent_pane.as_deref().unwrap_or(""),
                &format!("orc worker timeout: {}", command),
            );
            let _ = crate::tmux::kill_worker_pane(&worker);
            return Err(format!(
                "{} failed: timeout after {}s",
                command, timeout_sec
            ));
        }
        thread::sleep(Duration::from_millis(200));
    }
    let code_raw = fs::read_to_string(&code_path)
        .map_err(|e| format!("failed to read {}: {}", code_path.display(), e))?;
    let code = code_raw.trim().parse::<i32>().unwrap_or(1);
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    let _ = crate::tmux::kill_worker_pane(&worker);
    if let Some(parent) = parent_pane.as_deref() {
        let status_msg = if code == 0 {
            format!("orc worker done: {}", command)
        } else {
            format!("orc worker failed: {} (code={})", command, code)
        };
        let _ = crate::tmux::display_message(parent, &status_msg);
    }
    if code == 0 {
        verify_command_artifacts(command, before.as_ref())?;
        debug_log_auto_stage("session", &format!("worker pane done: {}", command));
        Ok(format!("{} completed", command))
    } else {
        let detail = stderr.lines().next().unwrap_or("").trim().to_string();
        Err(format!(
            "{} failed: code={}{}",
            command,
            code,
            if detail.is_empty() {
                "".to_string()
            } else {
                format!(" | {}", detail)
            }
        ))
    }
}

fn code_subcommand_timeout_sec() -> u64 {
    env::var("ORC_CODE_SUBCOMMAND_TIMEOUT_SEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(CODE_SUBCOMMAND_TIMEOUT_SEC)
}

fn capture_workflow_evidence() -> Result<WorkflowEvidenceSnapshot, String> {
    let plan_path = plan_yaml_path()?;
    let drafts_path = drafts_yaml_path()?;
    let plan_exists = plan_path.exists();
    let drafts_exists = drafts_path.exists();
    let plan = if plan_exists {
        Some(load_plan_doc()?)
    } else {
        None
    };
    let drafts = if drafts_exists {
        Some(load_drafts_doc()?)
    } else {
        None
    };
    Ok(WorkflowEvidenceSnapshot {
        plan_exists,
        plan_planned_len: plan.as_ref().map_or(0, |doc| doc.drafts.planned.len()),
        plan_worked_len: plan.as_ref().map_or(0, |doc| doc.drafts.worked.len()),
        plan_complete_len: plan.as_ref().map_or(0, |doc| doc.drafts.complete.len()),
        plan_error_len: plan.as_ref().map_or(0, |doc| doc.drafts.error.len()),
        drafts_exists,
        draft_items_len: drafts.as_ref().map_or(0, |doc| doc.draft.len()),
        drafts_planned_len: drafts.as_ref().map_or(0, |doc| {
            doc.draft
                .iter()
                .filter(|item| item.state == "planned")
                .count()
        }),
        drafts_worked_len: drafts.as_ref().map_or(0, |doc| {
            doc.draft
                .iter()
                .filter(|item| item.state == "worked")
                .count()
        }),
        drafts_complete_len: drafts.as_ref().map_or(0, |doc| {
            doc.draft
                .iter()
                .filter(|item| item.state == "complete")
                .count()
        }),
        drafts_error_len: drafts.as_ref().map_or(0, |doc| {
            doc.draft
                .iter()
                .filter(|item| item.state == "error")
                .count()
        }),
    })
}

fn verify_command_artifacts(
    command: &str,
    before: Option<&WorkflowEvidenceSnapshot>,
) -> Result<(), String> {
    let after = capture_workflow_evidence()?;
    match command {
        "init_code_plan" => verify_plan_artifacts(before, &after),
        "add_code_draft" => verify_draft_artifacts(before, &after),
        "impl_code_draft" => verify_impl_artifacts(before, &after),
        "check_code_draft" => verify_check_artifacts(&after),
        _ => Ok(()),
    }
}

fn verify_plan_artifacts(
    before: Option<&WorkflowEvidenceSnapshot>,
    after: &WorkflowEvidenceSnapshot,
) -> Result<(), String> {
    if !after.plan_exists {
        return Err("init_code_plan completed without .project/plan.yaml".to_string());
    }
    if after.plan_planned_len
        + after.plan_worked_len
        + after.plan_complete_len
        + after.plan_error_len
        == 0
    {
        return Err(
            "init_code_plan completed but .project/plan.yaml has no draft states".to_string(),
        );
    }
    if let Some(before) = before {
        if before == after {
            return Err(
                "init_code_plan completed but .project/plan.yaml state did not change".to_string(),
            );
        }
    }
    Ok(())
}

fn verify_draft_artifacts(
    before: Option<&WorkflowEvidenceSnapshot>,
    after: &WorkflowEvidenceSnapshot,
) -> Result<(), String> {
    if !after.drafts_exists {
        return Err("add_code_draft completed without .project/drafts.yaml".to_string());
    }
    if after.draft_items_len == 0 {
        return Err(
            "add_code_draft completed but .project/drafts.yaml has no draft items".to_string(),
        );
    }
    if after.drafts_planned_len + after.drafts_worked_len + after.drafts_complete_len == 0 {
        return Err("add_code_draft completed but .project/drafts.yaml has no planned/worked/complete state".to_string());
    }
    if let Some(before) = before {
        if before.draft_items_len == after.draft_items_len
            && before.drafts_planned_len == after.drafts_planned_len
            && before.drafts_worked_len == after.drafts_worked_len
            && before.drafts_complete_len == after.drafts_complete_len
            && before.drafts_error_len == after.drafts_error_len
        {
            return Err(
                "add_code_draft completed but .project/drafts.yaml state did not change"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn verify_impl_artifacts(
    before: Option<&WorkflowEvidenceSnapshot>,
    after: &WorkflowEvidenceSnapshot,
) -> Result<(), String> {
    if !after.plan_exists || !after.drafts_exists {
        return Err("impl_code_draft completed without plan/drafts artifacts".to_string());
    }
    if let Some(before) = before {
        let after_resolved =
            after.plan_complete_len + after.drafts_complete_len + after.drafts_error_len;
        let before_resolved =
            before.plan_complete_len + before.drafts_complete_len + before.drafts_error_len;
        if after_resolved < before_resolved {
            return Err("impl_code_draft completed but resolved draft state regressed".to_string());
        }
    }
    Ok(())
}

fn verify_check_artifacts(after: &WorkflowEvidenceSnapshot) -> Result<(), String> {
    if !after.plan_exists || !after.drafts_exists {
        return Err("check_code_draft completed without plan/drafts artifacts".to_string());
    }
    Ok(())
}

fn impl_draft_llm_timeout_sec() -> u64 {
    env::var("ORC_IMPL_DRAFT_LLM_TIMEOUT_SEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(IMPL_DRAFT_LLM_TIMEOUT_SEC)
}

fn build_input_md_auto() -> Result<String, String> {
    append_check_process_status("create_input_md", "read project.md");
    let project_md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let plan_path = plan_yaml_path()?;
    append_check_process_status(
        "create_input_md",
        &format!("read plan.yaml | {}", plan_path.display()),
    );
    let plan_yaml = fs::read_to_string(&plan_path)
        .map_err(|e| format!("failed to read {}: {}", plan_path.display(), e))?;
    let prompt_path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("build_input_md_auto.txt");
    append_check_process_status(
        "create_input_md",
        &format!("load prompt | {}", prompt_path.display()),
    );
    let prompt_template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\nplan.yaml:\n{}\n\n출력은 반드시 input.md 본문만 반환한다.",
        prompt_template, project_md, plan_yaml
    );
    append_check_process_status("create_input_md", "prompt prepared");
    append_check_process_status("create_input_md", "request llm");
    let raw = crate::run_codex_exec_capture(&prompt)?;
    let body = crate::extract_markdown_block(&raw);
    if body.trim().is_empty() {
        return Err("build_input_md_auto failed: empty input.md body".to_string());
    }
    append_check_process_status("create_input_md", "write input.md raw body");
    fs::write(crate::INPUT_MD_PATH, format!("{}\n", body))
        .map_err(|e| format!("failed to write {}: {}", crate::INPUT_MD_PATH, e))?;
    append_check_process_status("create_input_md", "validate input.md objects");
    let parsed = parse_input_md_objects(Path::new(crate::INPUT_MD_PATH))?;
    if !parsed.is_empty() {
        append_check_process_status("create_input_md", "rewrite normalized input.md");
        let mut rebuilt = String::new();
        for obj in &parsed {
            rebuilt.push_str(&format!("# {}\n", obj.name));
            for rule in &obj.rules {
                rebuilt.push_str(&format!("- {}\n", rule));
            }
            for step in &obj.steps {
                rebuilt.push_str(&format!("> {}\n", step));
            }
            rebuilt.push('\n');
        }
        fs::write(crate::INPUT_MD_PATH, rebuilt)
            .map_err(|e| format!("failed to write {}: {}", crate::INPUT_MD_PATH, e))?;
    }
    if parsed.is_empty() {
        return Err("build_input_md_auto failed: input.md has no valid feature object".to_string());
    }
    append_check_process_status("create_input_md", "artifact verification completed");
    append_check_process_status(
        "create_input_md",
        &format!("completed | feature_objects={}", parsed.len()),
    );
    Ok("build_input_md_auto completed: input.md generated".to_string())
}

pub(crate) fn check_code_draft(_auto_yes: bool) -> Result<String, String> {
    ensure_default_scenario_file()?;
    validate_scenario_file()?;
    let reference_dir = ensure_project_reference_dir()?;
    let debug_enabled = crate::load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    let mut debug_pane = String::new();
    if debug_enabled && env_flag_true("ORC_ENABLE_CHECK_DEBUG_PANE") {
        let debug_cmd = "mkdir -p .project/reference && touch .project/reference/check-code.log && tail -n 200 -f .project/reference/check-code.log";
        if let Ok(pane_id) = crate::tmux::split_window_run(debug_cmd) {
            let _ = crate::tmux::rename_pane(&pane_id, "check-code-debug");
            debug_pane = pane_id;
        }
    }
    let drafts = load_drafts_doc()?;
    let names: HashSet<String> = drafts.draft.iter().map(|v| v.name.clone()).collect();
    let mut list: Vec<String> = names.into_iter().collect();
    list.sort();
    let test_result = match crate::test_command() {
        Ok(v) => v,
        Err(e) => format!("test failed: {}", e),
    };
    let fast_path_check = drafts
        .draft
        .iter()
        .all(|item| matches!(item.state.as_str(), "complete" | "error"));
    let follow = if fast_path_check
        && !test_result.to_ascii_lowercase().contains("fail")
        && !test_result.to_ascii_lowercase().contains("error")
    {
        "NO_CHANGE (fast-path)".to_string()
    } else {
        crate::run_check_code_after_draft_changes(&list, "check_code_draft")?
    };
    let report = Path::new("report.md");
    let issues = collect_check_draft_issues(&follow, &test_result);
    crate::append_spec_checkpoint_issues("check_code_draft", &issues)?;
    let body = render_check_report_from_template(
        &list,
        &follow,
        &test_result,
        if debug_pane.is_empty() {
            "(not opened)"
        } else {
            &debug_pane
        },
        &issues,
    )?;
    fs::write(report, body).map_err(|e| format!("failed to write {}: {}", report.display(), e))?;
    write_feedback_sections(&test_result, &issues)?;
    Ok(format!(
        "check_code_draft completed: report.md generated | reference={}",
        reference_dir.display()
    ))
}

pub(crate) fn check_task() -> Result<String, String> {
    let mut plan = load_plan_doc()?;
    sync_plan_doc(&mut plan);
    let drafts = load_drafts_doc()?;
    let draft_names: HashSet<String> = drafts.draft.iter().map(|v| v.name.clone()).collect();
    let missing: Vec<String> = plan
        .drafts
        .planned
        .iter()
        .filter(|v| !draft_names.contains(*v))
        .cloned()
        .collect();
    if missing.is_empty() {
        Ok("check_task completed: plan/draft linkage ok".to_string())
    } else {
        Ok(format!(
            "check_task completed: missing draft items={}",
            missing.join(", ")
        ))
    }
}

fn infer_from_message(msg: &str) -> (String, String, String) {
    let spec = infer_spec_with_llm(msg, None).unwrap_or_default();
    let name = normalize_feature_key(msg);
    (name, msg.trim().to_string(), spec)
}

fn infer_spec_with_llm(message: &str, workspace_hint: Option<&str>) -> Option<String> {
    let prompt_path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("infer_code_spec.txt");
    let template = fs::read_to_string(&prompt_path).ok().unwrap_or_else(|| {
        "spec inference prompt\n- 출력은 한 줄: spec: <value>\n- 설명/코드블록 없이 값만 출력"
            .to_string()
    });
    let hint = workspace_hint.unwrap_or("");
    let prompt = format!(
        "{}\n\nmessage:\n{}\n\nworkspace_hint:\n{}\n\n반드시 `spec: ...` 한 줄만 출력.",
        template, message, hint
    );
    let raw = crate::run_codex_exec_capture(&prompt).ok()?;
    parse_inferred_spec(&raw)
}

fn parse_inferred_spec(raw: &str) -> Option<String> {
    let body = crate::extract_markdown_block(raw);
    let source = if body.trim().is_empty() { raw } else { &body };
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some((lhs, rhs)) = trimmed.split_once(':') {
            if lhs.trim().eq_ignore_ascii_case("spec") {
                let value = normalize_inferred_spec(rhs);
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    for line in source.lines() {
        let value = normalize_inferred_spec(line);
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}

fn normalize_inferred_spec(value: &str) -> String {
    value
        .trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_start_matches("- ")
        .to_string()
}

#[derive(Deserialize, Default)]
struct DraftFieldsInferOut {
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    tasks: Vec<String>,
    #[serde(default)]
    check: Vec<String>,
}

#[derive(Deserialize, Default)]
struct DraftItemInferOut {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    item_type: String,
    #[serde(default)]
    domain: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    step: Vec<String>,
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    tasks: Vec<String>,
    #[serde(default, rename = "constraints")]
    constraints: Vec<String>,
    #[serde(default)]
    check: Vec<String>,
}

fn infer_draft_fields_with_llm(
    project_md: &str,
    name: &str,
    domain: &str,
    item_type: &str,
) -> DraftFieldsInferOut {
    let prompt_path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("infer_draft_fields.txt");
    let prompt_template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))
        .unwrap_or_else(|_| {
            "infer_draft_fields prompt\n- output yaml keys: scope, tasks, check".to_string()
        });
    let prompt = format!(
        "{}\n\nproject_md:\n{}\n\nname: {}\ndomain: {}\ntype: {}",
        prompt_template, project_md, name, domain, item_type
    );
    let Ok(raw) = crate::run_codex_exec_capture(&prompt) else {
        return DraftFieldsInferOut::default();
    };
    let yaml = crate::extract_yaml_block(&raw);
    serde_yaml::from_str::<DraftFieldsInferOut>(&yaml).unwrap_or_default()
}

fn infer_draft_item_with_llm(
    project_md: &str,
    plan_yaml: &str,
    name: &str,
    from_input: Option<&InputFeatureObject>,
) -> DraftItemDoc {
    let fallback_item = fallback_draft_item(name, from_input);
    if from_input.is_some() {
        append_check_process_status(
            "add_code_draft",
            &format!("reuse input object without llm | {}", name),
        );
        return fallback_item;
    }
    let input_rules = from_input.map(|v| v.rules.join(" | ")).unwrap_or_default();
    let input_steps = from_input.map(|v| v.steps.join(" | ")).unwrap_or_default();
    let prompt_path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("infer_draft_item.txt");
    let prompt_template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))
        .unwrap_or_else(|_| "infer_draft_item prompt\n- output yaml fields".to_string());
    let draft_item_template = read_code_template("draft_item.yaml").unwrap_or_else(|_| {
        "- name: \"\"\n  type: \"action\"\n  domain: []\n  depends_on: []\n  scope: []\n  rule: []\n  step: []\n  tasks: []\n  constraints: []\n  check: []\n".to_string()
    });
    let prompt = format!(
        "{}\n\ndraft_item_template:\n{}\n\nproject_md:\n{}\n\nplan_yaml:\n{}\n\nname: {}\ninput_rules: {}\ninput_steps: {}",
        prompt_template, draft_item_template, project_md, plan_yaml, name, input_rules, input_steps
    );
    append_check_process_status(
        "add_code_draft",
        &format!("draft item prompt prepared | {}", name),
    );
    let Ok(raw) = crate::run_codex_exec_capture_with_timeout(&prompt, DRAFT_ITEM_LLM_TIMEOUT_SEC)
    else {
        append_check_process_status(
            "add_code_draft",
            &format!("draft item llm fallback | {}", name),
        );
        return fallback_item;
    };
    append_check_process_status(
        "add_code_draft",
        &format!("draft item llm response received | {}", name),
    );
    let yaml = crate::extract_yaml_block(&raw);
    let item_out = serde_yaml::from_str::<DraftItemInferOut>(&yaml).unwrap_or_default();
    let inferred_type = if item_out.item_type.trim().is_empty() {
        infer_item_type(name)
    } else {
        item_out.item_type
    };
    let inferred_domain = if item_out.domain.is_empty() {
        vec!["app".to_string()]
    } else {
        item_out.domain
    };
    let first_domain = inferred_domain
        .first()
        .cloned()
        .unwrap_or_else(|| "app".to_string());
    let fallback_fields =
        infer_draft_fields_with_llm(project_md, name, &first_domain, &inferred_type);
    let mut step = item_out.step;
    if step.is_empty() {
        step = from_input
            .map(|o| o.steps.clone())
            .filter(|v| !v.is_empty())
            .unwrap_or_default();
    }
    let scope = if item_out.scope.is_empty() {
        fallback_fields.scope
    } else {
        item_out.scope
    };
    let tasks = if item_out.tasks.is_empty() {
        fallback_fields.tasks
    } else {
        item_out.tasks
    };
    let check = if item_out.check.is_empty() {
        fallback_fields.check
    } else {
        item_out.check
    };
    let inferred_name = if item_out.name.trim().is_empty() {
        name.to_string()
    } else {
        normalize_feature_key(&item_out.name)
    };
    let constraints = normalize_constraints_format(item_out.constraints);
    append_check_process_status(
        "add_code_draft",
        &format!("draft item normalized | {}", name),
    );
    DraftItemDoc {
        name: if inferred_name.is_empty() {
            fallback_item.name
        } else {
            inferred_name
        },
        state: default_draft_item_state(),
        item_type: inferred_type,
        domain: inferred_domain,
        depends_on: item_out.depends_on,
        scope,
        rule: item_out.rule,
        step,
        tasks,
        constraints,
        check,
    }
}

fn fallback_draft_item(name: &str, from_input: Option<&InputFeatureObject>) -> DraftItemDoc {
    let normalized_name = normalize_feature_key(name);
    let safe_name = if normalized_name.is_empty() {
        "draft_item".to_string()
    } else {
        normalized_name
    };
    let fallback_steps = from_input
        .map(|v| v.steps.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec![format!("implement {}", safe_name)]);
    let fallback_rules = from_input
        .map(|v| v.rules.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec![format!("{} exists", safe_name)]);
    DraftItemDoc {
        name: safe_name.clone(),
        state: default_draft_item_state(),
        item_type: infer_item_type(&safe_name),
        domain: vec!["app".to_string()],
        depends_on: Vec::new(),
        scope: vec![safe_name.clone()],
        rule: fallback_rules,
        step: fallback_steps.clone(),
        tasks: fallback_steps,
        constraints: Vec::new(),
        check: vec!["cargo test".to_string()],
    }
}

fn normalize_constraint_entry(raw: &str) -> String {
    let mut value = raw
        .trim()
        .trim_start_matches("- ")
        .trim()
        .replace("=>", "->");
    if value.is_empty() {
        return "입력 -> 출력 : 기능 설명".to_string();
    }
    if let Some(arrow_pos) = value.find("->") {
        if let Some(colon_pos) = value[arrow_pos + 2..].find(':') {
            let colon_abs = arrow_pos + 2 + colon_pos;
            let input = value[..arrow_pos].trim();
            let output = value[arrow_pos + 2..colon_abs].trim();
            let desc = value[colon_abs + 1..].trim();
            let fixed_input = if input.is_empty() { "입력" } else { input };
            let fixed_output = if output.is_empty() { "출력" } else { output };
            let fixed_desc = if desc.is_empty() {
                "기능 설명"
            } else {
                desc
            };
            return format!("{} -> {} : {}", fixed_input, fixed_output, fixed_desc);
        }
    }
    if let Some((left, right)) = value.split_once(':') {
        let input = left.trim();
        let desc = right.trim();
        let fixed_input = if input.is_empty() { "입력" } else { input };
        let fixed_desc = if desc.is_empty() {
            "기능 설명"
        } else {
            desc
        };
        return format!("{} -> 출력 : {}", fixed_input, fixed_desc);
    }
    value = value.replace(':', " ");
    format!("입력 -> 출력 : {}", value.trim())
}

fn normalize_constraints_format(raw_constraints: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in raw_constraints {
        let normalized = normalize_constraint_entry(&raw);
        if !out.iter().any(|v| v == &normalized) {
            out.push(normalized);
        }
    }
    out
}

fn infer_project_detail_with_llm(project_md: &str) -> Result<String, String> {
    let prompt_path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("add_detail_project_code.txt");
    let template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\n출력은 project.md 전체 markdown만 반환한다.",
        template, project_md
    );
    let raw = crate::run_codex_exec_capture(&prompt)?;
    let out = crate::extract_markdown_block(&raw);
    let next = if out.trim().is_empty() { raw } else { out };
    validate_project_md_headers(&next)?;
    Ok(format!("{}\n", next.trim_end()))
}

fn infer_domain_block_with_llm(project_md: &str) -> Result<String, String> {
    let prompt_path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("create_domain.txt");
    let template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\n출력은 # domains 아래 body markdown만 반환한다.",
        template, project_md
    );
    let raw = crate::run_codex_exec_capture(&prompt)?;
    let out = crate::extract_markdown_block(&raw);
    let body = if out.trim().is_empty() { raw } else { out };
    if !body.lines().any(|v| v.trim_start().starts_with("## ")) {
        return Err(
            "create_code_domain failed: inferred domains block has no domain header".to_string(),
        );
    }
    Ok(body)
}

fn infer_plan_doc_with_llm(project_md: &str) -> Result<CodePlanDoc, String> {
    let prompt_path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("infer_plan_yaml.txt");
    let template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;
    let plan_template = read_code_template("plan.yaml")?;
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\nplan template:\n{}\n\n출력은 plan.yaml YAML만 반환한다.",
        template, project_md, plan_template
    );
    let raw = crate::run_codex_exec_capture(&prompt)?;
    let yaml = crate::extract_yaml_block(&raw);
    let mut doc: CodePlanDoc =
        serde_yaml::from_str(&yaml).map_err(|e| format!("infer_plan_yaml parse failed: {}", e))?;
    sync_plan_doc(&mut doc);
    Ok(doc)
}

fn validate_project_md_headers(markdown: &str) -> Result<(), String> {
    let required = [
        "# info",
        "# features",
        "# rules",
        "# constraints",
        "# domains",
    ];
    for header in required {
        if !markdown
            .lines()
            .any(|v| v.trim().eq_ignore_ascii_case(header))
        {
            return Err(format!(
                "project.md format invalid: missing header `{}`",
                header
            ));
        }
    }
    Ok(())
}

fn apply_project_info_overrides(
    name: &str,
    description: &str,
    path: &str,
    spec: &str,
) -> Result<(), String> {
    let project_path = Path::new(crate::PROJECT_MD_PATH);
    let raw = fs::read_to_string(project_path)
        .map_err(|e| format!("failed to read {}: {}", project_path.display(), e))?;
    let mut next = raw;
    next = replace_info_field_value(&next, "name", name);
    next = replace_info_field_value(&next, "description", description);
    next = replace_info_field_value(&next, "path", path);
    next = replace_info_field_value(&next, "spec", spec);
    fs::write(project_path, next)
        .map_err(|e| format!("failed to write {}: {}", project_path.display(), e))
}

fn ensure_bootstrap_spec_artifacts(project_root: &Path, spec: &str) -> Result<String, String> {
    let spec_lc = spec.to_ascii_lowercase();
    if !(spec_lc.contains("react") || spec_lc.contains("next")) {
        return Ok("bootstrap-verify: skipped(non-react spec)".to_string());
    }
    let package_json_path = project_root.join("package.json");
    if !package_json_path.exists() {
        return Ok("bootstrap-verify: package.json missing".to_string());
    }
    let raw = fs::read_to_string(&package_json_path)
        .map_err(|e| format!("failed to read {}: {}", package_json_path.display(), e))?;
    let mut json: JsonValue =
        serde_json::from_str(&raw).map_err(|e| format!("invalid package.json: {}", e))?;
    if spec_lc.contains("zustand") {
        let has_zustand = json
            .get("dependencies")
            .and_then(|v| v.as_object())
            .is_some_and(|deps| deps.contains_key("zustand"))
            || json
                .get("devDependencies")
                .and_then(|v| v.as_object())
                .is_some_and(|deps| deps.contains_key("zustand"));
        if !has_zustand {
            let deps = json
                .as_object_mut()
                .ok_or_else(|| "package.json root is not object".to_string())?
                .entry("dependencies")
                .or_insert_with(|| serde_json::json!({}));
            let deps_obj = deps
                .as_object_mut()
                .ok_or_else(|| "package.json dependencies is not object".to_string())?;
            deps_obj.insert(
                "zustand".to_string(),
                JsonValue::String("^5.0.0".to_string()),
            );
            let pretty = serde_json::to_string_pretty(&json)
                .map_err(|e| format!("failed to encode package.json: {}", e))?;
            fs::write(&package_json_path, format!("{}\n", pretty))
                .map_err(|e| format!("failed to write {}: {}", package_json_path.display(), e))?;
            return Ok("bootstrap-verify: added zustand dependency".to_string());
        }
    }
    Ok("bootstrap-verify: spec dependency ok".to_string())
}

fn debug_log_auto_stage(stage: &str, message: &str) {
    let debug_enabled = crate::load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    if !debug_enabled {
        return;
    }
    println!("[auto:{}] {}", stage, message);
    let project_dir = Path::new(".project");
    if !project_dir.exists() {
        return;
    }
    let runtime_dir = project_dir.join("runtime");
    if fs::create_dir_all(&runtime_dir).is_err() {
        return;
    }
    let path = runtime_dir.join("auto-code.log");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {} | {}", now, stage, message);
    }
}

fn extract_info_value(info: &str, key: &str) -> Option<String> {
    for line in info.lines() {
        let trimmed = line.trim();
        let Some((k, v)) = trimmed.split_once(':') else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case(key) {
            return Some(v.trim().trim_matches('`').to_string());
        }
    }
    None
}

fn extract_project_spec_from_md(project_md: &str) -> Option<String> {
    let info = crate::extract_project_info(project_md);
    extract_info_value(&info, "spec")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[derive(Default)]
struct CommonOpts {
    name: Option<String>,
    path: Option<String>,
    spec: Option<String>,
    description: Option<String>,
    message: Option<String>,
    auto: bool,
}

fn parse_common_opts(args: &[String]) -> CommonOpts {
    let mut out = CommonOpts::default();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                out.name = args.get(i).cloned();
            }
            "-p" => {
                i += 1;
                out.path = args.get(i).cloned();
            }
            "-s" => {
                i += 1;
                out.spec = args.get(i).cloned();
            }
            "-d" => {
                i += 1;
                out.description = args.get(i).cloned();
            }
            "-a" if i + 1 < args.len() => {
                out.auto = true;
                if args[i + 1].starts_with('-') {
                    // keep as flag-only form
                } else {
                    out.message = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "-a" => out.auto = true,
            _ => {}
        }
        i += 1;
    }
    out
}

fn normalize_feature_key(raw: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for ch in raw.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_us = false;
        } else if !last_us {
            out.push('_');
            last_us = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn infer_item_type(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if lower.contains("calc") || lower.contains("compute") || lower.contains("sum") {
        "calc".to_string()
    } else if lower.contains("fix") || lower.contains("bug") || lower.contains("refactor") {
        "fix".to_string()
    } else {
        "action".to_string()
    }
}

fn pick_domain_for_feature(domains: &[String], name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for domain in domains {
        if lower.contains(domain) {
            return domain.clone();
        }
    }
    domains
        .first()
        .cloned()
        .unwrap_or_else(|| "app".to_string())
}

fn parse_input_md_feature_names(path: &Path) -> Result<Vec<String>, String> {
    Ok(parse_input_md_objects(path)?
        .into_iter()
        .map(|v| v.name)
        .collect::<Vec<_>>())
}

fn parse_input_md_objects(path: &Path) -> Result<Vec<InputFeatureObject>, String> {
    if !path.exists() {
        return Err(format!("input file not found: {}", path.display()));
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut out = Vec::new();
    let mut current: Option<InputFeatureObject> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix('#') {
            if let Some(prev) = current.take() {
                out.push(prev);
            }
            current = Some(InputFeatureObject {
                name: normalize_feature_key(name),
                rules: Vec::new(),
                steps: Vec::new(),
            });
            continue;
        }
        if let Some(rule) = trimmed.strip_prefix("- ") {
            if let Some(ref mut obj) = current {
                obj.rules.push(rule.trim().to_string());
            }
            continue;
        }
        if let Some(step) = trimmed.strip_prefix("> ") {
            if let Some(ref mut obj) = current {
                obj.steps.push(step.trim().to_string());
            }
        }
    }
    if let Some(last) = current.take() {
        out.push(last);
    }
    Ok(out
        .into_iter()
        .filter(|v| !v.name.trim().is_empty())
        .collect::<Vec<_>>())
}

fn extract_list_under_header(markdown: &str, header: &str) -> Vec<String> {
    let mut in_section = false;
    let mut out = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with('#') {
            break;
        }
        if !in_section {
            continue;
        }
        let item = if let Some(v) = trimmed.strip_prefix("- ") {
            v.trim()
        } else if let Some((_, right)) = trimmed.split_once(". ") {
            if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                right.trim()
            } else {
                continue;
            }
        } else {
            continue;
        };
        if item.is_empty() {
            continue;
        }
        let key = normalize_feature_key(item);
        if !key.is_empty() && !out.iter().any(|v| v == &key) {
            out.push(key);
        }
    }
    out
}

fn read_line(prompt: &str) -> Result<String, String> {
    print!("{}", prompt);
    io::stdout()
        .flush()
        .map_err(|e| format!("failed to flush stdout: {}", e))?;
    let mut buf = Vec::new();
    io::stdin()
        .lock()
        .read_until(b'\n', &mut buf)
        .map_err(|e| format!("failed to read stdin: {}", e))?;
    Ok(String::from_utf8_lossy(&buf).trim().to_string())
}

fn ask_yes_no(prompt: &str) -> Result<bool, String> {
    let ans = read_line(prompt)?;
    Ok(matches!(ans.to_ascii_lowercase().as_str(), "y" | "yes"))
}

fn is_current_dir_empty() -> Result<bool, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let mut entries =
        fs::read_dir(&cwd).map_err(|e| format!("failed to read {}: {}", cwd.display(), e))?;
    Ok(entries.next().is_none())
}

fn ensure_project_dir() -> Result<PathBuf, String> {
    let dir = Path::new(".project");
    fs::create_dir_all(dir).map_err(|e| format!("failed to create {}: {}", dir.display(), e))?;
    Ok(dir.to_path_buf())
}

fn plan_yaml_path() -> Result<PathBuf, String> {
    Ok(ensure_project_dir()?.join("plan.yaml"))
}

fn drafts_yaml_path() -> Result<PathBuf, String> {
    Ok(ensure_project_dir()?.join("drafts.yaml"))
}

fn load_plan_doc() -> Result<CodePlanDoc, String> {
    let path = plan_yaml_path()?;
    if !path.exists() {
        let raw = read_code_template("plan.yaml")?;
        let mut doc: CodePlanDoc = serde_yaml::from_str(&raw)
            .map_err(|e| format!("failed to parse plan template: {}", e))?;
        sync_plan_doc(&mut doc);
        return Ok(doc);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut doc: CodePlanDoc = serde_yaml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    sync_plan_doc(&mut doc);
    Ok(doc)
}

fn save_plan_doc(doc: &CodePlanDoc) -> Result<(), String> {
    let path = plan_yaml_path()?;
    let mut next = doc.clone();
    sync_plan_doc(&mut next);
    let raw =
        serde_yaml::to_string(&next).map_err(|e| format!("failed to encode plan yaml: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn load_drafts_doc() -> Result<CodeDraftsDoc, String> {
    let path = drafts_yaml_path()?;
    if !path.exists() {
        let raw = read_code_template("drafts.yaml")?;
        let mut doc: CodeDraftsDoc = serde_yaml::from_str(&raw)
            .map_err(|e| format!("failed to parse drafts template: {}", e))?;
        sync_drafts_doc(&mut doc);
        return Ok(doc);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut doc: CodeDraftsDoc = serde_yaml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    sync_drafts_doc(&mut doc);
    Ok(doc)
}

fn save_drafts_doc(doc: &CodeDraftsDoc) -> Result<(), String> {
    let path = drafts_yaml_path()?;
    let mut next = doc.clone();
    sync_drafts_doc(&mut next);
    let raw =
        serde_yaml::to_string(&next).map_err(|e| format!("failed to encode drafts yaml: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn sync_drafts_doc(doc: &mut CodeDraftsDoc) {
    let mut seen = HashSet::new();
    doc.draft.retain(|item| seen.insert(item.name.clone()));
    for item in &mut doc.draft {
        item.state = normalize_draft_item_state(&item.state);
    }
}

fn draft_item_state_change(doc: &mut CodeDraftsDoc, name: &str, to: &str) -> Result<(), String> {
    let next = normalize_draft_item_state(to);
    let item = doc
        .draft
        .iter_mut()
        .find(|item| item.name == name)
        .ok_or_else(|| format!("draft item not found: {}", name))?;
    item.state = next;
    sync_drafts_doc(doc);
    Ok(())
}

fn sync_plan_with_draft_items(plan: &mut CodePlanDoc, drafts: &CodeDraftsDoc) {
    for item in &drafts.draft {
        let _ = plan_draft_state_change(plan, &item.name, &item.state);
    }
    plan.drafts.planned.retain(|name| {
        !drafts
            .draft
            .iter()
            .any(|item| item.name == *name && item.state != "planned")
    });
    plan.drafts.worked.retain(|name| {
        !drafts
            .draft
            .iter()
            .any(|item| item.name == *name && item.state != "worked")
    });
    plan.drafts.complete.retain(|name| {
        !drafts
            .draft
            .iter()
            .any(|item| item.name == *name && item.state != "complete")
    });
    plan.drafts.error.retain(|name| {
        !drafts
            .draft
            .iter()
            .any(|item| item.name == *name && item.state != "error")
    });
    sync_plan_doc(plan);
}

fn plan_draft_state_change(doc: &mut CodePlanDoc, name: &str, to: &str) -> Result<(), String> {
    let to = normalize_draft_item_state(to);
    for list in [
        &mut doc.drafts.planned,
        &mut doc.drafts.worked,
        &mut doc.drafts.complete,
        &mut doc.drafts.error,
    ] {
        if let Some(pos) = list.iter().position(|v| v == name) {
            list.remove(pos);
        }
    }
    let target = match to.as_str() {
        "planned" => &mut doc.drafts.planned,
        "worked" => &mut doc.drafts.worked,
        "complete" => &mut doc.drafts.complete,
        "error" => &mut doc.drafts.error,
        _ => return Err(format!("invalid state: {}", to)),
    };
    if !target.iter().any(|v| v == name) {
        target.push(name.to_string());
    }
    sync_plan_doc(doc);
    Ok(())
}

fn now_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0)
}

fn validate_scenario_file() -> Result<(), String> {
    let path = Path::new(".project").join("scenario.md");
    if !path.exists() {
        return Err(format!(
            "scenario validation failed: missing {}",
            path.display()
        ));
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut checked = 0usize;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('|').map(|v| v.trim()).collect();
        if parts.len() != 3 || parts.iter().any(|v| v.is_empty()) {
            return Err(format!(
                "scenario validation failed: invalid line `{}` (expected: 명령 | 실행/변경 파일 | 파생 결과)",
                trimmed
            ));
        }
        checked += 1;
    }
    if checked == 0 {
        return Err("scenario validation failed: no executable scenario line".to_string());
    }
    Ok(())
}

fn ensure_default_scenario_file() -> Result<(), String> {
    let path = Path::new(".project").join("scenario.md");
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let default = "add_code_draft | .project/drafts.yaml | drafts planned updated\n";
    fs::write(&path, default).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn extract_domains_from_project_md(project_md: &str) -> Vec<String> {
    crate::extract_project_md_domain_names(project_md)
}

fn extract_domain_subsection_items(
    project_md: &str,
    domain: &str,
    subsection: &str,
) -> Vec<String> {
    let domain_key = normalize_feature_key(domain);
    let subsection_key = normalize_feature_key(subsection);
    let mut in_domains = false;
    let mut in_domain = false;
    let mut in_sub = false;
    let mut out = Vec::new();
    for line in project_md.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("# domains") {
            in_domains = true;
            in_domain = false;
            in_sub = false;
            continue;
        }
        if in_domains && trimmed.starts_with("# ") && !trimmed.eq_ignore_ascii_case("# domains") {
            break;
        }
        if !in_domains {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("## ") {
            let heading_key = normalize_feature_key(name.trim().trim_matches('`'));
            if heading_key == domain_key {
                in_domain = true;
                in_sub = false;
                continue;
            }
            if in_domain {
                break;
            }
        }
        if !in_domain {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("### ") {
            in_sub = normalize_feature_key(name.trim()) == subsection_key;
            continue;
        }
        if in_sub {
            if let Some(item) = trimmed.strip_prefix("- ") {
                let value = item.trim().to_string();
                if !value.is_empty() {
                    out.push(value);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        change_state_plan, draft_item_state_change, extract_domain_subsection_items,
        extract_domains_from_project_md, plan_draft_state_change, sync_plan_with_draft_items,
        verify_draft_artifacts, verify_plan_artifacts, CodeDraftsDoc, CodePlanDoc, DraftItemDoc,
        WorkflowEvidenceSnapshot,
    };

    #[test]
    fn extract_domains_from_project_md_reads_new_domain_headers() {
        let md = r#"# info
name : sample

# domains
## app
### states
- draft
### action
- run
### rules
- keep explicit boundaries

## auth
### states
- idle
### action
- login
### rules
- validate token
"#;
        let got = extract_domains_from_project_md(md);
        assert_eq!(got, vec!["app".to_string(), "auth".to_string()]);
    }

    #[test]
    fn extract_domain_subsection_items_reads_new_domain_subsections() {
        let md = r#"# domains
## app
### states
- draft
- complete
### action
- run
### rules
- keep explicit boundaries
"#;
        let got = extract_domain_subsection_items(md, "app", "states");
        assert_eq!(got, vec!["draft".to_string(), "complete".to_string()]);
    }

    #[test]
    fn change_state_plan_moves_item_without_duplication() {
        let mut doc = CodePlanDoc::default();
        doc.drafts.planned = vec!["ui".to_string()];
        change_state_plan(&mut doc, "ui", "planned", "worked").expect("move planned->worked");
        assert_eq!(doc.drafts.planned, Vec::<String>::new());
        assert_eq!(doc.drafts.worked, vec!["ui".to_string()]);

        change_state_plan(&mut doc, "ui", "worked", "complete").expect("move worked->complete");
        assert_eq!(doc.drafts.worked, Vec::<String>::new());
        assert_eq!(doc.drafts.complete, vec!["ui".to_string()]);
    }

    #[test]
    fn verify_plan_artifacts_requires_state_change() {
        let before = WorkflowEvidenceSnapshot {
            plan_exists: true,
            plan_planned_len: 1,
            ..WorkflowEvidenceSnapshot::default()
        };
        let after = before.clone();
        let err = verify_plan_artifacts(Some(&before), &after).expect_err("should fail");
        assert!(err.contains("did not change"));
    }

    #[test]
    fn verify_draft_artifacts_rejects_empty_drafts() {
        let after = WorkflowEvidenceSnapshot {
            drafts_exists: true,
            ..WorkflowEvidenceSnapshot::default()
        };
        let err = verify_draft_artifacts(None, &after).expect_err("should fail");
        assert!(err.contains("no draft items"));
    }

    #[test]
    fn verify_draft_artifacts_accepts_populated_state() {
        let before = WorkflowEvidenceSnapshot::default();
        let after = WorkflowEvidenceSnapshot {
            drafts_exists: true,
            draft_items_len: 2,
            drafts_planned_len: 2,
            ..WorkflowEvidenceSnapshot::default()
        };
        verify_draft_artifacts(Some(&before), &after).expect("populated drafts should pass");
    }

    #[test]
    fn draft_item_state_change_updates_plan_sync_target() {
        let mut drafts = CodeDraftsDoc {
            draft: vec![DraftItemDoc {
                name: "ui".to_string(),
                state: "planned".to_string(),
                ..DraftItemDoc::default()
            }],
        };
        let mut plan = CodePlanDoc::default();
        plan_draft_state_change(&mut plan, "ui", "planned").expect("plan init");
        draft_item_state_change(&mut drafts, "ui", "complete").expect("draft to complete");
        sync_plan_with_draft_items(&mut plan, &drafts);
        assert!(plan.drafts.complete.iter().any(|v| v == "ui"));
        assert!(!plan.drafts.planned.iter().any(|v| v == "ui"));
    }
}

fn sync_plan_doc(doc: &mut CodePlanDoc) {
    if let Ok(project_md) = fs::read_to_string(crate::PROJECT_MD_PATH) {
        for domain in extract_domains_from_project_md(&project_md) {
            if !doc.domains.iter().any(|v| v == &domain) {
                doc.domains.push(domain);
            }
        }
    }
    dedup_vec(&mut doc.domains);
    dedup_vec(&mut doc.drafts.error);
    dedup_vec(&mut doc.drafts.complete);
    dedup_vec(&mut doc.drafts.worked);
    dedup_vec(&mut doc.drafts.planned);
    doc.drafts
        .error
        .retain(|v| !doc.drafts.complete.iter().any(|c| c == v));
    doc.drafts
        .worked
        .retain(|v| !doc.drafts.error.iter().any(|e| e == v));
    doc.drafts
        .worked
        .retain(|v| !doc.drafts.complete.iter().any(|c| c == v));
    doc.drafts.planned.retain(|v| {
        !doc.drafts.complete.iter().any(|c| c == v)
            && !doc.drafts.worked.iter().any(|w| w == v)
            && !doc.drafts.error.iter().any(|e| e == v)
    });
}

fn dedup_vec(items: &mut Vec<String>) {
    let mut out = Vec::new();
    for item in items.iter() {
        if !out.iter().any(|v: &String| v == item) {
            out.push(item.clone());
        }
    }
    *items = out;
}

fn change_state_plan(
    doc: &mut CodePlanDoc,
    name: &str,
    from: &str,
    to: &str,
) -> Result<(), String> {
    let from_list = match from {
        "planned" => &mut doc.drafts.planned,
        "worked" => &mut doc.drafts.worked,
        "complete" => &mut doc.drafts.complete,
        "error" => &mut doc.drafts.error,
        _ => return Err(format!("invalid from state: {}", from)),
    };
    if let Some(pos) = from_list.iter().position(|v| v == name) {
        from_list.remove(pos);
    }

    let to_list = match to {
        "planned" => &mut doc.drafts.planned,
        "worked" => &mut doc.drafts.worked,
        "complete" => &mut doc.drafts.complete,
        "error" => &mut doc.drafts.error,
        _ => return Err(format!("invalid to state: {}", to)),
    };
    if !to_list.iter().any(|v| v == name) {
        to_list.push(name.to_string());
    }
    sync_plan_doc(doc);
    Ok(())
}

fn infer_plan_items_with_llm() -> Result<Vec<String>, String> {
    let md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let prompt_path = Path::new("assets")
        .join("presets")
        .join("code")
        .join("prompts")
        .join("add_code_plan.txt");
    let prompt_template = fs::read_to_string(&prompt_path).unwrap_or_else(|_| {
        "project.md를 읽고 planned 후보를 YAML로 출력해.\nplanned:\n  - item".to_string()
    });
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\n출력은 YAML만:\nplanned:\n  - <snake_case>",
        prompt_template, md
    );
    let raw = crate::run_codex_exec_capture(&prompt)?;
    let yaml = crate::extract_yaml_block(&raw);
    #[derive(Deserialize)]
    struct PlannedOut {
        #[serde(default)]
        planned: Vec<String>,
    }
    let parsed: PlannedOut = serde_yaml::from_str(&yaml)
        .map_err(|e| format!("add_code_plan auto parse failed: {}", e))?;
    let mut out = Vec::new();
    for item in parsed.planned {
        let key = normalize_feature_key(&item);
        if !key.is_empty() && !out.iter().any(|v| v == &key) {
            out.push(key);
        }
    }
    Ok(out)
}

fn create_project_md_from_template(
    name: &str,
    description: &str,
    path: &str,
    spec: &str,
) -> Result<String, String> {
    let mut body = read_code_template("project.md")?;
    body = replace_info_field_value(&body, "name", name);
    body = replace_info_field_value(&body, "description", description);
    body = replace_info_field_value(&body, "path", path);
    body = replace_info_field_value(&body, "spec", spec);

    write_project_md(&body)?;
    Ok("project.md created from template".to_string())
}

fn replace_domains_section(project_md: &str, domains_body: &str) -> String {
    let mut out = Vec::new();
    let mut in_domains = false;
    let mut inserted = false;
    for line in project_md.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("# domains") {
            in_domains = true;
            inserted = true;
            out.push("# domains".to_string());
            out.push(domains_body.trim_end().to_string());
            continue;
        }
        if in_domains {
            if trimmed.starts_with("# ") && !trimmed.eq_ignore_ascii_case("# domains") {
                in_domains = false;
                out.push(line.to_string());
            }
            continue;
        }
        out.push(line.to_string());
    }
    if !inserted {
        out.push(String::new());
        out.push("# domains".to_string());
        out.push(domains_body.trim_end().to_string());
    }
    format!("{}\n", out.join("\n"))
}

fn write_project_md(body: &str) -> Result<(), String> {
    let project_path = Path::new(crate::PROJECT_MD_PATH);
    if let Some(parent) = project_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    fs::write(project_path, format!("{}\n", body))
        .map_err(|e| format!("failed to write {}: {}", project_path.display(), e))
}

fn enforce_project_md_primary_path() -> Result<(), String> {
    let primary = Path::new(".project");
    if !primary.exists() {
        fs::create_dir_all(primary)
            .map_err(|e| format!("failed to create {}: {}", primary.display(), e))?;
    }
    Ok(())
}

fn ensure_project_memo_initialized() -> Result<(), String> {
    let memo_path = Path::new(".project").join("memo.md");
    if memo_path.exists() {
        return Ok(());
    }
    fs::write(&memo_path, "").map_err(|e| format!("failed to write {}: {}", memo_path.display(), e))
}

fn infer_workspace_features(cwd: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    if cwd.join("package.json").exists() {
        out.push("node_package_workspace".to_string());
    }
    if cwd.join("Cargo.toml").exists() {
        out.push("rust_cli_workspace".to_string());
    }
    if cwd.join("README.md").exists() {
        out.push("project_documentation".to_string());
    }
    let cli_path = cwd.join("src").join("cli.rs");
    if cli_path.exists() {
        let raw = fs::read_to_string(&cli_path)
            .map_err(|e| format!("failed to read {}: {}", cli_path.display(), e))?;
        for line in raw.lines() {
            let Some((_, right)) = line.split_once('"') else {
                continue;
            };
            let Some((cmd, _)) = right.split_once('"') else {
                continue;
            };
            if cmd.contains(' ')
                || cmd.is_empty()
                || cmd.starts_with('-')
                || !cmd
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c == '-')
            {
                continue;
            }
            let item = format!("cli_{}", cmd.replace('-', "_"));
            if !out.iter().any(|v| v == &item) {
                out.push(item);
            }
        }
    }
    if out.is_empty() {
        out.push("workspace_bootstrap".to_string());
    }
    Ok(out)
}

fn replace_markdown_list_section(raw: &str, header: &str, items: &[String]) -> String {
    let mut out = Vec::new();
    let mut in_section = false;
    let mut replaced = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) {
            in_section = true;
            replaced = true;
            out.push(line.to_string());
            for item in items {
                out.push(format!("- {}", item));
            }
            continue;
        }
        if in_section && trimmed.starts_with('#') {
            in_section = false;
        }
        if in_section {
            continue;
        }
        out.push(line.to_string());
    }
    if !replaced {
        out.push(header.to_string());
        for item in items {
            out.push(format!("- {}", item));
        }
    }
    out.join("\n")
}

fn upsert_list_items_under_header(raw: &str, header: &str, required_items: &[String]) -> String {
    let existing = extract_plain_list_under_header(raw, header);
    let mut merged = existing;
    for item in required_items {
        if !merged.iter().any(|v| v == item) {
            merged.push(item.clone());
        }
    }
    replace_markdown_list_section(raw, header, &merged)
}

fn remove_list_items_under_header(raw: &str, header: &str, items: &[&str]) -> String {
    let existing = extract_plain_list_under_header(raw, header);
    let filtered: Vec<String> = existing
        .into_iter()
        .filter(|value| !items.iter().any(|target| value.trim() == *target))
        .collect();
    replace_markdown_list_section(raw, header, &filtered)
}

fn extract_plain_list_under_header(markdown: &str, header: &str) -> Vec<String> {
    let mut in_section = false;
    let mut out = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with('#') {
            break;
        }
        if !in_section {
            continue;
        }
        let Some(item) = trimmed.strip_prefix("- ") else {
            continue;
        };
        let value = item.trim().to_string();
        if value.is_empty() || out.iter().any(|v| v == &value) {
            continue;
        }
        out.push(value);
    }
    out
}

fn read_code_template(file_name: &str) -> Result<String, String> {
    let path = crate::source_root()
        .join("assets")
        .join("presets")
        .join("code")
        .join("templates")
        .join(file_name);
    fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))
}

fn ensure_project_md_initialized() -> Result<(), String> {
    let path = Path::new(crate::PROJECT_MD_PATH);
    if path.exists() {
        return Ok(());
    }
    let body = read_code_template("project.md")?;
    write_project_md(&body)
}

fn ensure_plan_yaml_initialized() -> Result<(), String> {
    let path = plan_yaml_path()?;
    if path.exists() {
        return Ok(());
    }
    let body = read_code_template("plan.yaml")?;
    fs::write(&path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn ensure_drafts_yaml_initialized() -> Result<(), String> {
    let path = drafts_yaml_path()?;
    if path.exists() {
        return Ok(());
    }
    let body = read_code_template("drafts.yaml")?;
    fs::write(&path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn ensure_project_reference_dir() -> Result<PathBuf, String> {
    let dir = Path::new(".project").join("reference");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create {}: {}", dir.display(), e))?;
    Ok(dir)
}

fn collect_check_draft_issues(follow: &str, test_result: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let follow_l = follow.to_ascii_lowercase();
    if follow_l.contains("fail") || follow_l.contains("error") {
        issues.push(format!("check-code follow-up: {}", follow));
    }
    let test_l = test_result.to_ascii_lowercase();
    if test_l.contains("fail") || test_l.contains("error") {
        issues.push(format!("test: {}", test_result));
    }
    issues
}

fn render_check_report_from_template(
    targets: &[String],
    follow: &str,
    test_result: &str,
    debug_pane: &str,
    issues: &[String],
) -> Result<String, String> {
    let template = read_code_template("report.md")?;
    let implementation_lines = [
        format!("- targets: {}", targets.join(", ")),
        format!("- check_followup: {}", follow),
        format!("- test: {}", test_result),
        format!("- debug_pane: {}", debug_pane),
    ]
    .join("\n");
    let issues_block = if issues.is_empty() {
        "- 없음".to_string()
    } else {
        issues
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut body = template.replace("{{implementation_check}}", &implementation_lines);
    body = body.replace("{{issues}}", &issues_block);
    Ok(format!("{}\n", body.trim_end()))
}

fn write_feedback_sections(test_result: &str, issues: &[String]) -> Result<(), String> {
    let unresolved = if issues.is_empty() {
        "- 없음".to_string()
    } else {
        issues
            .iter()
            .map(|issue| format!("- {}", issue))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let improvement = if issues.is_empty() {
        "- 현재 검증 기준(cargo test, report 생성)이 통과 상태다.".to_string()
    } else {
        "- 발견된 문제를 기준으로 후속 수정 후 check 단계를 재실행한다.".to_string()
    };
    let body = format!(
        "# 결과\n- {}\n\n# 미해결\n{}\n\n# 개선점\n{}\n",
        test_result, unresolved, improvement
    );
    let path = crate::project_feedback_path();
    fs::write(&path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn replace_info_field_value(raw: &str, key: &str, value: &str) -> String {
    let mut out = Vec::new();
    let mut replaced = false;
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if let Some((lhs, _rhs)) = trimmed.split_once(':') {
            if lhs.trim() == key {
                out.push(format!("{} : {}", key, value));
                replaced = true;
                continue;
            }
        }
        out.push(line.to_string());
    }
    if !replaced {
        out.push(format!("- {}: {}", key, value));
    }
    out.join("\n")
}

fn infer_workspace_spec(cwd: &Path) -> Result<String, String> {
    let mut workspace_hints: Vec<String> = Vec::new();
    if cwd.join("package.json").exists() {
        workspace_hints.push("package.json".to_string());
        let package_json_path = cwd.join("package.json");
        if let Ok(raw) = fs::read_to_string(package_json_path) {
            if let Ok(json) = serde_json::from_str::<JsonValue>(&raw) {
                let mut deps = Vec::new();
                for key in ["next", "react", "zustand", "typescript", "vite"] {
                    let has_dep = json
                        .get("dependencies")
                        .and_then(|v| v.as_object())
                        .is_some_and(|obj| obj.contains_key(key))
                        || json
                            .get("devDependencies")
                            .and_then(|v| v.as_object())
                            .is_some_and(|obj| obj.contains_key(key));
                    if has_dep {
                        deps.push(key.to_string());
                    }
                }
                if !deps.is_empty() {
                    workspace_hints.push(format!("deps={}", deps.join(",")));
                }
            }
        }
    }
    if cwd.join("Cargo.toml").exists() {
        workspace_hints.push("Cargo.toml".to_string());
    }
    if cwd.join("pyproject.toml").exists() || cwd.join("requirements.txt").exists() {
        workspace_hints.push("python".to_string());
    }
    if let Some(spec) = infer_spec_with_llm(
        "current workspace spec inference",
        Some(&workspace_hints.join(" | ")),
    ) {
        if !spec.trim().is_empty() {
            return Ok(spec);
        }
    }

    let has = |name: &str| cwd.join(name).exists();
    let has_ext = |ext: &str| -> Result<bool, String> {
        let entries = fs::read_dir(cwd)
            .map_err(|e| format!("failed to read dir {}: {}", cwd.display(), e))?;
        for entry in entries {
            let path = entry
                .map_err(|e| format!("failed to read dir entry in {}: {}", cwd.display(), e))?
                .path();
            if path
                .extension()
                .and_then(|v| v.to_str())
                .map(|v| v.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
            {
                return Ok(true);
            }
        }
        Ok(false)
    };
    if has("package.json") {
        let mut spec_parts = vec!["next js".to_string()];
        let package_json_path = cwd.join("package.json");
        if let Ok(raw) = fs::read_to_string(package_json_path) {
            if let Ok(json) = serde_json::from_str::<JsonValue>(&raw) {
                let has_dep = |name: &str| {
                    json.get("dependencies")
                        .and_then(|v| v.as_object())
                        .is_some_and(|deps| deps.contains_key(name))
                        || json
                            .get("devDependencies")
                            .and_then(|v| v.as_object())
                            .is_some_and(|deps| deps.contains_key(name))
                };
                if has_dep("react") && !spec_parts.iter().any(|v| v == "react") {
                    spec_parts.push("react".to_string());
                }
                if has_dep("zustand") && !spec_parts.iter().any(|v| v == "zustand") {
                    spec_parts.push("zustand".to_string());
                }
            }
        }
        return Ok(spec_parts.join(", "));
    }
    if has("Cargo.toml") {
        return Ok("rust".to_string());
    }
    if has("pyproject.toml") || has("requirements.txt") {
        return Ok("python".to_string());
    }
    if has_ext("go")? {
        return Ok("go".to_string());
    }
    Ok("next js".to_string())
}
