use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MODE_LIST: [&str; 4] = ["project", "plan", "draft", "report"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CodePlanDrafts {
    #[serde(default)]
    planned: Vec<String>,
    #[serde(default)]
    worked: Vec<String>,
    #[serde(default)]
    complete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CodePlanDoc {
    #[serde(default)]
    goal: String,
    #[serde(default)]
    domains: Vec<String>,
    #[serde(default)]
    drafts: CodePlanDrafts,
    #[serde(default, skip_serializing)]
    planned: Vec<String>,
    #[serde(default, skip_serializing)]
    worked: Vec<String>,
    #[serde(default, skip_serializing)]
    complete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DraftItemDoc {
    name: String,
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
        action_debug_log_auto_stage(
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
    apply_project_info_overrides(&name, &description, &path, &spec)?;
    let detail_msg = detail_code_project()?;
    enforce_project_md_primary_path()?;
    if opts.auto {
        action_debug_log_auto_stage("project-detail", "detail_code_project completed");
    }
    let domain_msg = create_code_domain()?;
    enforce_project_md_primary_path()?;
    if opts.auto {
        action_debug_log_auto_stage("domain-create", "create_code_domain completed");
    }
    if opts.auto {
        action_debug_log_auto_stage("bootstrap", "bootstrap_code_project start");
    }
    let bootstrap_msg = bootstrap_code_project()?;
    enforce_project_md_primary_path()?;
    if opts.auto {
        action_debug_log_auto_stage("bootstrap", "bootstrap_code_project completed");
    }
    result = format!("{} | {} | {} | {}", result, detail_msg, domain_msg, bootstrap_msg);
    if opts.auto {
        action_debug_log_auto_stage("plan-init", "init_code_plan -a start");
        let plan_msg = run_code_subcommand_in_new_session("init_code_plan", &["-a"])?;
        action_debug_log_auto_stage("plan-init", "init_code_plan -a completed");
        result = format!("{} | {}", result, plan_msg);
    }
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
    fs::write(path, next)
        .map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
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
    let info = crate::calc_extract_project_info(&md);
    let name = extract_info_value(&info, "name").unwrap_or_else(|| "project".to_string());
    let spec = extract_info_value(&info, "spec").unwrap_or_else(|| "next js".to_string());
    let status = crate::ui::action_apply_bootstrap_by_spec(Path::new("."), &name, &spec)?;
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
    action_debug_log_auto_stage(
        "plan-yaml",
        &format!(
            "plan.yaml generated: domains={} planned={} worked={} complete={}",
            doc.domains.len(),
            doc.drafts.planned.len(),
            doc.drafts.worked.len(),
            doc.drafts.complete.len()
        ),
    );
    let mut out = format!(
        "init_code_plan completed: domains={} planned={}",
        doc.domains.len(),
        doc.drafts.planned.len()
    );
    if auto {
        action_debug_log_auto_stage("input-build", "build_input_md_auto start");
        let input_msg = build_input_md_auto()?;
        action_debug_log_auto_stage("input-build", "build_input_md_auto completed");
        action_debug_log_auto_stage("draft-create", "add_code_draft -a start");
        let draft_msg = run_code_subcommand_in_new_session("add_code_draft", &["-a"])?;
        action_debug_log_auto_stage("draft-create", "add_code_draft -a completed");
        out = format!("{} | {} | {}", out, input_msg, draft_msg);
    }
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
            || doc.drafts.complete.iter().any(|v| v == &key)
        {
            continue;
        }
        doc.drafts.planned.push(key);
    }
    sync_plan_doc(&mut doc);
    save_plan_doc(&doc)?;
    let mut out = format!("add_code_plan completed: planned={}", doc.drafts.planned.len());
    if auto {
        let draft_msg = run_code_subcommand_in_new_session("add_code_draft", &[])?;
        out = format!("{} | {}", out, draft_msg);
        return Ok(out);
    }
    if ask_yes_no("add_code_draft()를 호출할까요? [y/N]: ")? {
        let draft_msg = run_code_subcommand_in_new_session("add_code_draft", &[])?;
        out = format!("{} | {}", out, draft_msg);
    }
    Ok(out)
}

pub(crate) fn create_code_draft() -> Result<String, String> {
    add_code_draft(&[])
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
    }

    ensure_drafts_yaml_initialized()?;
    let mut plan = load_plan_doc()?;
    sync_plan_doc(&mut plan);
    let mut drafts = load_drafts_doc()?;
    let mut plan_items = plan.drafts.planned.clone();

    let project_md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let plan_yaml_raw = {
        let path = plan_yaml_path()?;
        fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?
    };

    let input_objects = if use_file {
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
                plan.drafts.planned.push(name);
            }
        }
        sync_plan_doc(&mut plan);
        save_plan_doc(&plan)?;
    }

    for name in &plan_items {
        if drafts.draft.iter().any(|v| v.name == *name) {
            continue;
        }
        let from_input = input_objects.iter().find(|v| normalize_feature_key(&v.name) == *name);
        let inferred = infer_draft_item_with_llm(
            &project_md,
            &plan_yaml_raw,
            name,
            from_input,
        );
        drafts.draft.push(inferred);
    }

    if let Some(msg) = message {
        let name = normalize_feature_key(&msg);
        if !name.is_empty() && !drafts.draft.iter().any(|v| v.name == name) {
            let inferred = infer_draft_item_with_llm(
                &project_md,
                &plan_yaml_raw,
                &name,
                None,
            );
            drafts.draft.push(inferred);
        }
    }

    save_drafts_doc(&drafts)?;
    action_debug_log_auto_stage(
        "draft-yaml",
        &format!("drafts.yaml generated: draft={}", drafts.draft.len()),
    );
    let check = check_code_draft(false)?;
    action_debug_log_auto_stage("draft-yaml", "drafts.yaml checked");
    let mut out = format!(
        "add_code_draft completed: draft={} | {}",
        drafts.draft.len(),
        check
    );
    if auto {
        action_debug_log_auto_stage("impl", "impl_code_draft start");
        let impl_msg = run_impl_code_draft_via_cli()?;
        action_debug_log_auto_stage("impl", "impl_code_draft completed");
        out = format!("{} | {}", out, impl_msg);
    }
    Ok(out)
}

pub(crate) fn add_code_draft_item(args: &[String]) -> Result<String, String> {
    add_code_draft(args)
}

pub(crate) fn auto_code_message(message: &str) -> Result<String, String> {
    action_debug_log_auto_stage("auto", "auto message flow start");
    let out = run_code_subcommand_in_new_session("init_code_project", &["-a", message])?;
    action_debug_log_auto_stage("auto", "auto message flow completed");
    Ok(out)
}

pub(crate) async fn impl_code_draft() -> Result<String, String> {
    let mut plan = load_plan_doc()?;
    sync_plan_doc(&mut plan);
    let drafts = load_drafts_doc()?;
    if drafts.draft.is_empty() || plan.drafts.planned.is_empty() {
        return Ok("impl_code_draft skipped: no drafts.yaml.planned item".to_string());
    }

    let moved_to_worked = plan.drafts.planned.clone();
    for name in &moved_to_worked {
        change_state_plan(&mut plan, name, "planned", "worked")?;
    }
    sync_plan_doc(&mut plan);
    save_plan_doc(&plan)?;

    let worked_items: Vec<DraftItemDoc> = plan
        .drafts
        .worked
        .iter()
        .filter_map(|name| drafts.draft.iter().find(|v| &v.name == name).cloned())
        .collect();
    action_debug_log_auto_stage(
        "parallel-start",
        &format!("parallel execution start: {} item(s)", worked_items.len()),
    );
    let run_msg = match action_impl_code_draft_parallel(worked_items).await {
        Ok(run) => {
            for name in &run.succeeded {
                change_state_plan(&mut plan, name, "worked", "complete")?;
            }
            sync_plan_doc(&mut plan);
            save_plan_doc(&plan)?;
            if run.failed.is_empty() {
                format!("impl_code_draft parallel completed: {}", run.succeeded.join(", "))
            } else {
                let msg = format!(
                    "partial success: succeeded=[{}], failed=[{}]",
                    run.succeeded.join(", "),
                    run.failed.join(" | ")
                );
                action_write_feedback_md("impl_code_draft partial failure", &msg)?;
                return Err(format!("impl_code_draft failed: {}", msg));
            }
        }
        Err(e) => {
            action_write_feedback_md("impl_code_draft failed", &e)?;
            return Err(format!(
                "impl_code_draft failed after sync; check feedback.md: {}",
                e
            ));
        }
    };

    let check = check_code_draft(true)?;
    Ok(format!("impl_code_draft completed | {} | {}", run_msg, check))
}

struct ImplRunResult {
    succeeded: Vec<String>,
    failed: Vec<String>,
}

async fn action_impl_code_draft_parallel(items: Vec<DraftItemDoc>) -> Result<ImplRunResult, String> {
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
    let prompt_template = fs::read_to_string(&prompt_path)
        .unwrap_or_else(|_| "impl_code_draft prompt\n- draft_item을 구현하고 제약 만족 여부를 보고한다.".to_string());
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
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
            let output = tokio::task::spawn_blocking(move || crate::action_run_codex_exec_capture(&prompt))
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
    let mut failed = Vec::new();
    for handle in handles {
        match handle
            .await
            .map_err(|e| format!("parallel task join failed: {}", e))?
        {
            Ok(name) => done.push(name),
            Err(err) => failed.push(err),
        }
    }
    Ok(ImplRunResult {
        succeeded: done,
        failed,
    })
}

fn action_write_feedback_md(summary: &str, detail: &str) -> Result<(), String> {
    let body = format!(
        "# feedback\n\n- status: failed\n- summary: {}\n- detail: {}\n- action: fallback implementation applied\n",
        summary, detail
    );
    fs::write("feedback.md", body).map_err(|e| format!("failed to write feedback.md: {}", e))
}

fn run_impl_code_draft_via_cli() -> Result<String, String> {
    run_code_subcommand_in_new_session("impl_code_draft", &[])
}

fn run_code_subcommand_in_new_session(command: &str, args: &[&str]) -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    action_debug_log_auto_stage(
        "session",
        &format!("new session start: {} {}", command, args.join(" ")),
    );
    let mut cmd = Command::new(exe);
    cmd.arg(command);
    for arg in args {
        cmd.arg(arg);
    }
    let status = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to execute {}: {}", command, e))?;
    if status.success() {
        action_debug_log_auto_stage("session", &format!("new session completed: {}", command));
        Ok(format!("{} completed", command))
    } else {
        action_debug_log_auto_stage(
            "session",
            &format!("new session failed: {} | code={:?}", command, status.code()),
        );
        Err(format!(
            "{} failed: code={:?}",
            command,
            status.code()
        ))
    }
}

fn build_input_md_auto() -> Result<String, String> {
    let project_md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let plan_path = plan_yaml_path()?;
    let plan_yaml = fs::read_to_string(&plan_path)
        .map_err(|e| format!("failed to read {}: {}", plan_path.display(), e))?;
    let prompt_path = crate::action_source_root()
        .join("assets")
        .join("code")
        .join("prompts")
        .join("build_input_md_auto.txt");
    let prompt_template = fs::read_to_string(&prompt_path).map_err(|e| {
        format!(
            "failed to read {}: {}",
            prompt_path.display(),
            e
        )
    })?;
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\nplan.yaml:\n{}\n\n출력은 반드시 input.md 본문만 반환한다.",
        prompt_template, project_md, plan_yaml
    );
    let raw = crate::action_run_codex_exec_capture(&prompt)?;
    let body = crate::calc_extract_markdown_block(&raw);
    if body.trim().is_empty() {
        return Err("build_input_md_auto failed: empty input.md body".to_string());
    }
    fs::write(crate::INPUT_MD_PATH, format!("{}\n", body))
        .map_err(|e| format!("failed to write {}: {}", crate::INPUT_MD_PATH, e))?;
    let parsed = parse_input_md_objects(Path::new(crate::INPUT_MD_PATH))?;
    if !parsed.is_empty() {
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
    Ok("build_input_md_auto completed: input.md generated".to_string())
}

pub(crate) fn check_code_draft(auto_yes: bool) -> Result<String, String> {
    ensure_default_scenario_file()?;
    validate_scenario_file()?;
    let reference_dir = ensure_project_reference_dir()?;
    let debug_enabled = crate::action_load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    let mut debug_pane = String::new();
    if debug_enabled {
        let debug_cmd = "mkdir -p .project/reference && touch .project/reference/check-code.log && tail -n 200 -f .project/reference/check-code.log";
        if let Ok(pane_id) = crate::tmux::action_split_window_run(debug_cmd) {
            let _ = crate::tmux::action_rename_pane(&pane_id, "check-code-debug");
            debug_pane = pane_id;
        }
    }
    let drafts = load_drafts_doc()?;
    let mut names: HashSet<String> = drafts.draft.iter().map(|v| v.name.clone()).collect();
    let mut list: Vec<String> = names.into_iter().collect();
    list.sort();
    let follow = crate::action_run_check_code_after_draft_changes(&list, "check_code_draft")?;
    let test_result = match crate::test_command() {
        Ok(v) => v,
        Err(e) => format!("test failed: {}", e),
    };
    let report = Path::new("report.md");
    let issues = collect_check_draft_issues(&follow, &test_result);
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
        Ok(format!("check_task completed: missing draft items={}", missing.join(", ")))
    }
}

pub(crate) fn check_draft() -> Result<String, String> {
    check_code_draft(true)
}

fn infer_from_message(msg: &str) -> (String, String, String) {
    let spec = infer_spec_with_llm(msg, None).unwrap_or_default();
    let name = normalize_feature_key(msg);
    (name, msg.trim().to_string(), spec)
}

fn infer_spec_with_llm(message: &str, workspace_hint: Option<&str>) -> Option<String> {
    let prompt_path = crate::action_source_root()
        .join("assets")
        .join("code")
        .join("prompts")
        .join("infer_code_spec.txt");
    let template = fs::read_to_string(&prompt_path).ok().unwrap_or_else(|| {
        "spec inference prompt\n- 출력은 한 줄: spec: <value>\n- 설명/코드블록 없이 값만 출력".to_string()
    });
    let hint = workspace_hint.unwrap_or("");
    let prompt = format!(
        "{}\n\nmessage:\n{}\n\nworkspace_hint:\n{}\n\n반드시 `spec: ...` 한 줄만 출력.",
        template, message, hint
    );
    let raw = crate::action_run_codex_exec_capture(&prompt).ok()?;
    parse_inferred_spec(&raw)
}

fn parse_inferred_spec(raw: &str) -> Option<String> {
    let body = crate::calc_extract_markdown_block(raw);
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
    #[serde(default, rename = "constraints")]
    constraints: Vec<String>,
}

fn infer_draft_fields_with_llm(project_md: &str, name: &str, domain: &str, item_type: &str) -> DraftFieldsInferOut {
    let prompt_path = crate::action_source_root()
        .join("assets")
        .join("code")
        .join("prompts")
        .join("infer_draft_fields.txt");
    let prompt_template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))
        .unwrap_or_else(|_| "infer_draft_fields prompt\n- output yaml keys: scope, tasks, check".to_string());
    let prompt = format!(
        "{}\n\nproject_md:\n{}\n\nname: {}\ndomain: {}\ntype: {}",
        prompt_template, project_md, name, domain, item_type
    );
    let Ok(raw) = crate::action_run_codex_exec_capture(&prompt) else {
        return DraftFieldsInferOut::default();
    };
    let yaml = crate::calc_extract_yaml_block(&raw);
    serde_yaml::from_str::<DraftFieldsInferOut>(&yaml).unwrap_or_default()
}

fn infer_draft_item_with_llm(
    project_md: &str,
    plan_yaml: &str,
    name: &str,
    from_input: Option<&InputFeatureObject>,
) -> DraftItemDoc {
    let input_rules = from_input
        .map(|v| v.rules.join(" | "))
        .unwrap_or_default();
    let input_steps = from_input
        .map(|v| v.steps.join(" | "))
        .unwrap_or_default();
    let prompt_path = crate::action_source_root()
        .join("assets")
        .join("code")
        .join("prompts")
        .join("infer_draft_item.txt");
    let prompt_template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))
        .unwrap_or_else(|_| "infer_draft_item prompt\n- output yaml fields".to_string());
    let prompt = format!(
        "{}\n\nproject_md:\n{}\n\nplan_yaml:\n{}\n\nname: {}\ninput_rules: {}\ninput_steps: {}",
        prompt_template, project_md, plan_yaml, name, input_rules, input_steps
    );
    let Ok(raw) = crate::action_run_codex_exec_capture(&prompt) else {
        return DraftItemDoc {
            name: name.to_string(),
            ..DraftItemDoc::default()
        };
    };
    let yaml = crate::calc_extract_yaml_block(&raw);
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
    let first_domain = inferred_domain.first().cloned().unwrap_or_else(|| "app".to_string());
    let fields = infer_draft_fields_with_llm(project_md, name, &first_domain, &inferred_type);
    let mut step = item_out.step;
    if step.is_empty() {
        step = from_input
            .map(|o| o.steps.clone())
            .filter(|v| !v.is_empty())
            .unwrap_or_default();
    }
    DraftItemDoc {
        name: name.to_string(),
        item_type: inferred_type,
        domain: inferred_domain,
        depends_on: item_out.depends_on,
        scope: fields.scope,
        rule: item_out.rule,
        step,
        tasks: fields.tasks,
        constraints: item_out.constraints,
        check: fields.check,
    }
}

fn infer_project_detail_with_llm(project_md: &str) -> Result<String, String> {
    let prompt_path = crate::action_source_root()
        .join("assets")
        .join("code")
        .join("prompts")
        .join("add_detail_project_code.txt");
    let template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\n출력은 project.md 전체 markdown만 반환한다.",
        template, project_md
    );
    let raw = crate::action_run_codex_exec_capture(&prompt)?;
    let out = crate::calc_extract_markdown_block(&raw);
    let next = if out.trim().is_empty() { raw } else { out };
    validate_project_md_headers(&next)?;
    Ok(format!("{}\n", next.trim_end()))
}

fn infer_domain_block_with_llm(project_md: &str) -> Result<String, String> {
    let prompt_path = crate::action_source_root()
        .join("assets")
        .join("code")
        .join("prompts")
        .join("create_domain.txt");
    let template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\n출력은 # domains 아래 body markdown만 반환한다.",
        template, project_md
    );
    let raw = crate::action_run_codex_exec_capture(&prompt)?;
    let out = crate::calc_extract_markdown_block(&raw);
    let body = if out.trim().is_empty() { raw } else { out };
    if !body.lines().any(|v| v.trim_start().starts_with("## ")) {
        return Err("create_code_domain failed: inferred domains block has no domain header".to_string());
    }
    Ok(body)
}

fn infer_plan_doc_with_llm(project_md: &str) -> Result<CodePlanDoc, String> {
    let prompt_path = crate::action_source_root()
        .join("assets")
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
    let raw = crate::action_run_codex_exec_capture(&prompt)?;
    let yaml = crate::calc_extract_yaml_block(&raw);
    let mut doc: CodePlanDoc =
        serde_yaml::from_str(&yaml).map_err(|e| format!("infer_plan_yaml parse failed: {}", e))?;
    sync_plan_doc(&mut doc);
    Ok(doc)
}

fn validate_project_md_headers(markdown: &str) -> Result<(), String> {
    let required = ["# info", "# features", "# rules", "# constraints", "# domains"];
    for header in required {
        if !markdown.lines().any(|v| v.trim().eq_ignore_ascii_case(header)) {
            return Err(format!("project.md format invalid: missing header `{}`", header));
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
            deps_obj.insert("zustand".to_string(), JsonValue::String("^5.0.0".to_string()));
            let pretty = serde_json::to_string_pretty(&json)
                .map_err(|e| format!("failed to encode package.json: {}", e))?;
            fs::write(&package_json_path, format!("{}\n", pretty))
                .map_err(|e| format!("failed to write {}: {}", package_json_path.display(), e))?;
            return Ok("bootstrap-verify: added zustand dependency".to_string());
        }
    }
    Ok("bootstrap-verify: spec dependency ok".to_string())
}

fn action_debug_log_auto_stage(stage: &str, message: &str) {
    let debug_enabled = crate::action_load_app_config()
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
    let mut entries = fs::read_dir(&cwd)
        .map_err(|e| format!("failed to read {}: {}", cwd.display(), e))?;
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
        let mut doc: CodePlanDoc =
            serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse plan template: {}", e))?;
        sync_plan_doc(&mut doc);
        return Ok(doc);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut doc: CodePlanDoc =
        serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    sync_plan_doc(&mut doc);
    Ok(doc)
}

fn save_plan_doc(doc: &CodePlanDoc) -> Result<(), String> {
    let path = plan_yaml_path()?;
    let mut next = doc.clone();
    sync_plan_doc(&mut next);
    let raw = serde_yaml::to_string(&next).map_err(|e| format!("failed to encode plan yaml: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn load_drafts_doc() -> Result<CodeDraftsDoc, String> {
    let path = drafts_yaml_path()?;
    if !path.exists() {
        let raw = read_code_template("drafts.yaml")?;
        let doc: CodeDraftsDoc =
            serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse drafts template: {}", e))?;
        return Ok(doc);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let doc: CodeDraftsDoc =
        serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    Ok(doc)
}

fn save_drafts_doc(doc: &CodeDraftsDoc) -> Result<(), String> {
    let path = drafts_yaml_path()?;
    let raw = serde_yaml::to_string(doc).map_err(|e| format!("failed to encode drafts yaml: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn validate_scenario_file() -> Result<(), String> {
    let path = Path::new(".project").join("scenario.md");
    if !path.exists() {
        return Err(format!("scenario validation failed: missing {}", path.display()));
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
    crate::calc_extract_project_md_domain_names(project_md)
}

fn extract_domain_subsection_items(project_md: &str, domain: &str, subsection: &str) -> Vec<String> {
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
        change_state_plan, extract_domain_subsection_items, extract_domains_from_project_md,
        CodePlanDoc,
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

}

fn sync_plan_doc(doc: &mut CodePlanDoc) {
    if doc.drafts.planned.is_empty() && !doc.planned.is_empty() {
        doc.drafts.planned = doc.planned.clone();
    }
    if doc.drafts.worked.is_empty() && !doc.worked.is_empty() {
        doc.drafts.worked = doc.worked.clone();
    }
    if doc.drafts.complete.is_empty() && !doc.complete.is_empty() {
        doc.drafts.complete = doc.complete.clone();
    }
    dedup_vec(&mut doc.drafts.complete);
    dedup_vec(&mut doc.drafts.worked);
    dedup_vec(&mut doc.drafts.planned);
    doc.drafts.worked.retain(|v| !doc.drafts.complete.iter().any(|c| c == v));
    doc.drafts.planned.retain(|v| {
        !doc.drafts.complete.iter().any(|c| c == v) && !doc.drafts.worked.iter().any(|w| w == v)
    });
    doc.planned = doc.drafts.planned.clone();
    doc.worked = doc.drafts.worked.clone();
    doc.complete = doc.drafts.complete.clone();
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
        _ => return Err(format!("invalid from state: {}", from)),
    };
    if let Some(pos) = from_list.iter().position(|v| v == name) {
        from_list.remove(pos);
    }

    let to_list = match to {
        "planned" => &mut doc.drafts.planned,
        "worked" => &mut doc.drafts.worked,
        "complete" => &mut doc.drafts.complete,
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
    let prompt_path = Path::new("assets").join("code").join("prompts").join("add_code_plan.txt");
    let prompt_template = fs::read_to_string(&prompt_path).unwrap_or_else(|_| {
        "project.md를 읽고 planned 후보를 YAML로 출력해.\nplanned:\n  - item".to_string()
    });
    let prompt = format!(
        "{}\n\nproject.md:\n{}\n\n출력은 YAML만:\nplanned:\n  - <snake_case>",
        prompt_template, md
    );
    let raw = crate::action_run_codex_exec_capture(&prompt)?;
    let yaml = crate::calc_extract_yaml_block(&raw);
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
    let primary = Path::new(".project").join("project.md");
    let legacy = Path::new("project").join("project.md");
    if !legacy.exists() {
        return Ok(());
    }
    if !primary.exists() {
        if let Some(parent) = primary.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
        }
        fs::rename(&legacy, &primary).map_err(|e| {
            format!(
                "failed to move legacy project.md {} -> {}: {}",
                legacy.display(),
                primary.display(),
                e
            )
        })?;
    } else {
        fs::remove_file(&legacy)
            .map_err(|e| format!("failed to remove legacy {}: {}", legacy.display(), e))?;
    }
    let legacy_dir = Path::new("project");
    if legacy_dir.exists() && fs::read_dir(legacy_dir).map(|mut v| v.next().is_none()).unwrap_or(false) {
        let _ = fs::remove_dir(legacy_dir);
    }
    Ok(())
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
    let path = crate::action_source_root()
        .join("assets")
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
    let implementation_lines = vec![
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
