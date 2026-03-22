use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CODE_SUBCOMMAND_TIMEOUT_SEC: u64 = 600;
const IMPL_DRAFT_LLM_TIMEOUT_SEC: u64 = 240;
const LONG_WAIT_REPORT_SEC: u64 = 60;
const ADD_ORC_DRAFTS_SOFT_TIMEOUT_SEC: u64 = 150;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct JobTaskStatus {
    #[serde(default)]
    pub planned: Vec<String>,
    #[serde(default)]
    pub work: Vec<String>,
    #[serde(default)]
    pub check: Vec<String>,
    #[serde(default)]
    pub completed: Vec<String>,
    #[serde(default)]
    pub fail: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct JobRequirement {
    pub name: String,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct JobDoc {
    #[serde(default)]
    pub plan: Vec<String>,
    #[serde(default)]
    pub requirement: Vec<JobRequirement>,
    #[serde(default)]
    pub task: JobTaskStatus,
    #[serde(default)]
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct DraftItemDoc {
    pub name: String,
    pub state: String,
    #[serde(default, rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub domain: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub rule: Vec<String>,
    #[serde(default)]
    pub step: Vec<String>,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub check: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct CodeDraftsDoc {
    #[serde(default)]
    pub draft: Vec<DraftItemDoc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkflowEvidenceSnapshot {
    pub job_exists: bool,
    pub job_planned_len: usize,
    pub job_work_len: usize,
    pub job_check_len: usize,
    pub job_completed_len: usize,
    pub job_fail_len: usize,
    pub drafts_exists: bool,
    pub draft_items_len: usize,
}

// --- Path Utilities ---

pub(crate) fn job_md_path() -> PathBuf {
    PathBuf::from("job.md")
}

pub(crate) fn drafts_yaml_path() -> PathBuf {
    Path::new(".project").join("drafts.yaml")
}

pub(crate) fn ensure_project_dir() -> Result<PathBuf, String> {
    let dir = Path::new(".project");
    fs::create_dir_all(dir).map_err(|e| format!("failed to create .project: {}", e))?;
    Ok(dir.to_path_buf())
}

// --- IO Utilities ---

pub(crate) fn load_job_doc() -> Result<JobDoc, String> {
    let path = job_md_path();
    if !path.exists() {
        return Ok(JobDoc::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read job.md: {}", e))?;
    parse_job_md(&raw)
}

pub(crate) fn save_job_doc(doc: &JobDoc) -> Result<(), String> {
    let path = job_md_path();
    let body = render_job_md(doc);
    fs::write(&path, body).map_err(|e| format!("failed to write job.md: {}", e))
}

pub(crate) fn load_drafts_doc() -> Result<CodeDraftsDoc, String> {
    let path = drafts_yaml_path();
    if !path.exists() {
        return Ok(CodeDraftsDoc::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read drafts.yaml: {}", e))?;
    serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse drafts.yaml: {}", e))
}

pub(crate) fn save_drafts_doc(doc: &CodeDraftsDoc) -> Result<(), String> {
    let path = drafts_yaml_path();
    ensure_project_dir()?;
    let raw = serde_yaml::to_string(doc).map_err(|e| format!("failed to encode drafts.yaml: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write drafts.yaml: {}", e))
}

pub(crate) fn read_template(name: &str) -> Result<String, String> {
    let path = crate::source_root().join("assets").join("templates").join(name);
    fs::read_to_string(&path).map_err(|e| format!("failed to read template {}: {}", name, e))
}

pub(crate) fn read_prompt(name: &str) -> Result<String, String> {
    let path = crate::source_root().join("assets").join("prompts").join(name);
    fs::read_to_string(&path).map_err(|e| format!("failed to read prompt {}: {}", name, e))
}

// --- Core Workflow Functions ---

/// Step 1: Initialize project.md
pub(crate) fn init_orc_project(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args);
    ensure_project_dir()?;
    let template = read_template("project.md")?;
    let target = Path::new(".project").join("project.md");
    
    let mut body = template;
    if let Some(name) = opts.name {
        body = body.replace("{{name}}", &name);
    }
    if let Some(desc) = opts.description {
        body = body.replace("{{description}}", &desc);
    }
    if let Some(spec) = opts.spec {
        body = body.replace("{{spec}}", &spec);
    }
    
    fs::write(&target, body).map_err(|e| format!("failed to write project.md: {}", e))?;
    
    if opts.auto {
        // detail_code_project logic could go here
    }
    
    Ok("init_orc_project completed".to_string())
}

/// Step 2: Build Domains
pub(crate) fn build_orc_domains() -> Result<String, String> {
    let project_md_path = Path::new(".project").join("project.md");
    let project_md = fs::read_to_string(&project_md_path)
        .map_err(|e| format!("failed to read project.md: {}", e))?;
    
    let prompt_template = read_prompt("build_domains.md")?;
    let prompt = format!("{}\n\nproject.md:\n{}", prompt_template, project_md);
    
    let raw = crate::run_codex_exec_capture(&prompt)?;
    let domain_block = crate::extract_markdown_block(&raw);
    
    let next = replace_markdown_section(&project_md, "# domains", &domain_block);
    fs::write(&project_md_path, next).map_err(|e| format!("failed to update project.md: {}", e))?;
    
    Ok("build_orc_domains completed".to_string())
}

/// Step 3: Initialize Job
pub(crate) fn init_orc_job() -> Result<String, String> {
    let path = job_md_path();
    if path.exists() {
        return Ok("job.md already exists".to_string());
    }
    
    let template = read_template("job.md")?;
    let project_md = fs::read_to_string(Path::new(".project").join("project.md"))
        .map_err(|e| format!("failed to read project.md: {}", e))?;
    
    let features = extract_plain_list_under_header(&project_md, "# features");
    let mut doc = JobDoc::default();
    for f in features {
        doc.requirement.push(JobRequirement {
            name: f,
            ..Default::default()
        });
    }
    
    save_job_doc(&doc)?;
    Ok("init_orc_job completed".to_string())
}

/// Step 4: Add Drafts from Job
pub(crate) fn add_orc_drafts() -> Result<String, String> {
    let mut job = load_job_doc()?;
    let mut drafts = load_drafts_doc()?;
    
    let mut added = 0;
    let mut skipped_due_budget = 0;
    let started_at = Instant::now();
    for req in &job.requirement {
        if started_at.elapsed().as_secs() >= ADD_ORC_DRAFTS_SOFT_TIMEOUT_SEC {
            skipped_due_budget += 1;
            continue;
        }
        let key = normalize_feature_key(&req.name);
        if drafts.draft.iter().any(|d| d.name == key) {
            continue;
        }
        
        let mut item = build_draft_item_from_requirement(req);
        item.name = key.clone();
        item.state = "planned".to_string();
        drafts.draft.push(item);
        if !job.task.planned.contains(&key) {
            job.task.planned.push(key);
        }
        added += 1;
    }
    
    save_job_doc(&job)?;
    save_drafts_doc(&drafts)?;
    Ok(format!(
        "add_orc_drafts completed: added {} items, deferred {} items (budget)",
        added, skipped_due_budget
    ))
}

fn build_draft_item_from_requirement(req: &JobRequirement) -> DraftItemDoc {
    DraftItemDoc {
        name: normalize_feature_key(&req.name),
        state: "planned".to_string(),
        item_type: "action".to_string(),
        domain: vec!["core".to_string()],
        depends_on: vec![],
        scope: vec![format!("feature:{}", normalize_feature_key(&req.name))],
        rule: req.rules.clone(),
        step: if req.steps.is_empty() {
            vec!["trigger -> process -> result".to_string()]
        } else {
            req.steps.clone()
        },
        tasks: vec![format!("implement {}", normalize_feature_key(&req.name))],
        constraints: vec![
            format!(
                "{} -> {} : requirement 기반 draft item 생성",
                req.name,
                normalize_feature_key(&req.name)
            ),
        ],
        check: vec![format!("verify {}", normalize_feature_key(&req.name))],
    }
}

/// Step 5: Implement Code (Parallel)
pub(crate) async fn impl_orc_code() -> Result<String, String> {
    let mut drafts = load_drafts_doc()?;
    let mut job = load_job_doc()?;
    let targets: Vec<DraftItemDoc> = drafts.draft.iter()
        .filter(|d| d.state == "planned" || d.state == "work" || d.state == "worked")
        .cloned()
        .collect();
    
    if targets.is_empty() {
        return Ok("no drafts to implement".to_string());
    }

    for item in &targets {
        set_draft_item_state(&mut drafts, &item.name, "work")?;
        move_job_task_item(&mut job, &item.name, "work")?;
    }
    save_drafts_doc(&drafts)?;
    save_job_doc(&job)?;
    
    let result = impl_code_draft_parallel(targets).await?;
    for name in &result.succeeded {
        set_draft_item_state(&mut drafts, name, "complete")?;
        move_job_task_item(&mut job, name, "check")?;
    }
    for (name, reason) in &result.failed {
        set_draft_item_state(&mut drafts, name, "error")?;
        move_job_task_item(&mut job, name, "fail")?;
        if !reason.trim().is_empty() {
            job.problems.push(format!("- {} : {}", name, reason.trim()));
        }
    }
    save_drafts_doc(&drafts)?;
    save_job_doc(&job)?;
    Ok(format!("impl_orc_code completed: success={} fail={}", result.succeeded.len(), result.failed.len()))
}

/// Step 6: Check Code
pub(crate) fn check_orc_code() -> Result<String, String> {
    let prompt_template = read_prompt("check_code.md")?;
    // ... logic to run checks and update report.md/job.md problems ...
    Ok("check_orc_code completed".to_string())
}

// --- Internal Implementation Helpers ---

fn parse_job_md(raw: &str) -> Result<JobDoc, String> {
    let mut doc = JobDoc::default();
    let mut section = "";
    let mut req = JobRequirement::default();
    let mut task_list = "";

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# plan") {
            section = "plan";
            continue;
        } else if trimmed.starts_with("# requirement") {
            section = "req";
            continue;
        } else if trimmed.starts_with("# task") {
            if !req.name.is_empty() { doc.requirement.push(req.clone()); req = JobRequirement::default(); }
            section = "task";
            continue;
        } else if trimmed.starts_with("# problems") {
            if !req.name.is_empty() { doc.requirement.push(req.clone()); req = JobRequirement::default(); }
            section = "prob";
            continue;
        }

        match section {
            "plan" => {
                if trimmed.starts_with("- ") {
                    doc.plan.push(trimmed[2..].trim().to_string());
                }
            }
            "req" => {
                if trimmed.starts_with("## ") {
                    if !req.name.is_empty() { doc.requirement.push(req.clone()); }
                    req = JobRequirement { name: trimmed[3..].trim().to_string(), ..Default::default() };
                } else if trimmed.starts_with("- ") {
                    req.rules.push(trimmed[2..].trim().to_string());
                } else if trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) && trimmed.contains('.') {
                    req.steps.push(trimmed.split_once('.').unwrap().1.trim().to_string());
                }
            }
            "task" => {
                if trimmed.starts_with("## planned") { task_list = "planned"; }
                else if trimmed.starts_with("## work") { task_list = "work"; }
                else if trimmed.starts_with("## check") { task_list = "check"; }
                else if trimmed.starts_with("## completed") { task_list = "completed"; }
                else if trimmed.starts_with("## fail") { task_list = "fail"; }
                else if trimmed.starts_with("- ") {
                    let val = trimmed[2..].trim().to_string();
                    match task_list {
                        "planned" => doc.task.planned.push(val),
                        "work" => doc.task.work.push(val),
                        "check" => doc.task.check.push(val),
                        "completed" => doc.task.completed.push(val),
                        "fail" => doc.task.fail.push(val),
                        _ => {}
                    }
                }
            }
            "prob" => {
                if trimmed.starts_with("- ") { doc.problems.push(trimmed.to_string()); }
            }
            _ => {}
        }
    }
    if !req.name.is_empty() { doc.requirement.push(req); }
    Ok(doc)
}

fn render_job_md(doc: &JobDoc) -> String {
    let mut out = String::from("# plan\n");
    for p in &doc.plan {
        out.push_str(&format!("- {}\n", p));
    }
    out.push_str("\n# requirement\n");
    for r in &doc.requirement {
        out.push_str(&format!("## {}\n", r.name));
        for (i, s) in r.steps.iter().enumerate() { out.push_str(&format!("{}. {}\n", i+1, s)); }
        for ru in &r.rules { out.push_str(&format!("- {}\n", ru)); }
    }
    out.push_str("\n# task\n");
    out.push_str("## planned\n");
    for i in &doc.task.planned { out.push_str(&format!("- {}\n", i)); }
    out.push_str("## work\n");
    for i in &doc.task.work { out.push_str(&format!("- {}\n", i)); }
    out.push_str("## check\n");
    for i in &doc.task.check { out.push_str(&format!("- {}\n", i)); }
    out.push_str("## completed\n");
    for i in &doc.task.completed { out.push_str(&format!("- {}\n", i)); }
    out.push_str("## fail\n");
    for i in &doc.task.fail { out.push_str(&format!("- {}\n", i)); }
    out.push_str("\n# problems\n");
    for p in &doc.problems { out.push_str(&format!("{}\n", p)); }
    out
}

pub(crate) fn job_task_state_change(doc: &mut JobDoc, name: &str, to: &str) -> Result<(), String> {
    doc.task.planned.retain(|v| v != name);
    doc.task.work.retain(|v| v != name);
    doc.task.check.retain(|v| v != name);
    doc.task.completed.retain(|v| v != name);
    doc.task.fail.retain(|v| v != name);

    match to {
        "planned" => doc.task.planned.push(name.to_string()),
        "work" | "worked" => doc.task.work.push(name.to_string()),
        "check" => doc.task.check.push(name.to_string()),
        "completed" | "complete" => doc.task.completed.push(name.to_string()),
        "fail" | "error" => doc.task.fail.push(name.to_string()),
        _ => return Err(format!("invalid state: {}", to)),
    }
    Ok(())
}

fn normalize_draft_state(to: &str) -> Result<&'static str, String> {
    match to {
        "planned" => Ok("planned"),
        "work" | "worked" => Ok("work"),
        "complete" | "completed" => Ok("complete"),
        "error" | "fail" => Ok("error"),
        _ => Err(format!("invalid draft state: {}", to)),
    }
}

pub(crate) fn set_draft_item_state(
    doc: &mut CodeDraftsDoc,
    name: &str,
    to_state: &str,
) -> Result<(), String> {
    let next = normalize_draft_state(to_state)?;
    let target = doc
        .draft
        .iter_mut()
        .find(|item| item.name == name)
        .ok_or_else(|| format!("draft item not found: {}", name))?;
    target.state = next.to_string();
    Ok(())
}

pub(crate) fn move_job_task_item(doc: &mut JobDoc, name: &str, to_list: &str) -> Result<(), String> {
    job_task_state_change(doc, name, to_list)
}

fn replace_markdown_section(raw: &str, header: &str, body: &str) -> String {
    let mut out = Vec::new();
    let mut in_section = false;
    let mut found = false;
    for line in raw.lines() {
        if line.trim().eq_ignore_ascii_case(header) {
            in_section = true;
            found = true;
            out.push(line.to_string());
            out.push(body.to_string());
            continue;
        }
        if in_section && line.trim().starts_with('#') { in_section = false; }
        if !in_section { out.push(line.to_string()); }
    }
    if !found {
        out.push(header.to_string());
        out.push(body.to_string());
    }
    out.join("\n")
}

fn extract_plain_list_under_header(markdown: &str, header: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) { in_section = true; continue; }
        if in_section && trimmed.starts_with('#') { break; }
        if in_section && trimmed.starts_with("- ") {
            out.push(trimmed[2..].trim().to_string());
        }
    }
    out
}

fn normalize_feature_key(name: &str) -> String {
    name.trim().to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>()
        .split('_').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_")
}

#[derive(Default)]
struct CommonOpts {
    name: Option<String>,
    description: Option<String>,
    spec: Option<String>,
    auto: bool,
}

fn parse_common_opts(args: &[String]) -> CommonOpts {
    let mut opts = CommonOpts::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => { i += 1; opts.name = args.get(i).cloned(); }
            "-d" => { i += 1; opts.description = args.get(i).cloned(); }
            "-s" => { i += 1; opts.spec = args.get(i).cloned(); }
            "-a" => { opts.auto = true; }
            _ => {}
        }
        i += 1;
    }
    opts
}

// --- Parallel Implementation Logic (Simplified/Modernized) ---

async fn impl_code_draft_parallel(items: Vec<DraftItemDoc>) -> Result<ImplRunResult, String> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
    let mut handles = Vec::new();

    for item in items {
        let sem = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let name = item.name.clone();
            impl_single_draft_item(&item)
                .map(|_| name.clone())
                .map_err(|e| (name, e))
        }));
    }

    for h in handles {
        match h.await.unwrap() {
            Ok(name) => succeeded.push(name),
            Err((name, e)) => failed.push((name, e)),
        }
    }

    Ok(ImplRunResult { succeeded, failed })
}

fn impl_single_draft_item(item: &DraftItemDoc) -> Result<(), String> {
    let prompt_base = read_prompt("build_parallel.md")?;
    let task_yaml =
        serde_yaml::to_string(item).map_err(|e| format!("failed to encode draft item yaml: {}", e))?;
    let prompt = format!(
        "{}\n\n# 입력 task 단일 객체\n```yaml\n{}\n```\n\n# 추가 지시\n- `drafts.yaml`, `job.md`는 절대 직접 수정하지 말고 코드 생성/수정 내용만 출력한다.\n- 상태 전이(work/complete/error, planned/work/check/fail 이동)는 Rust 오케스트레이터가 수행하므로 출력하지 않는다.",
        prompt_base, task_yaml
    );
    let raw = crate::run_codex_exec_capture_with_timeout(&prompt, IMPL_DRAFT_LLM_TIMEOUT_SEC)?;
    if raw.trim().is_empty() {
        return Err(format!("empty llm output for draft {}", item.name));
    }
    Ok(())
}

struct ImplRunResult {
    succeeded: Vec<String>,
    failed: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::{
        job_task_state_change, move_job_task_item, set_draft_item_state, CodeDraftsDoc, DraftItemDoc, JobDoc,
    };

    #[test]
    fn set_draft_item_state_updates_target_only() {
        let mut doc = CodeDraftsDoc {
            draft: vec![
                DraftItemDoc {
                    name: "alpha".to_string(),
                    state: "planned".to_string(),
                    ..Default::default()
                },
                DraftItemDoc {
                    name: "beta".to_string(),
                    state: "planned".to_string(),
                    ..Default::default()
                },
            ],
        };
        set_draft_item_state(&mut doc, "alpha", "worked").expect("state update");
        assert_eq!(doc.draft[0].state, "work");
        assert_eq!(doc.draft[1].state, "planned");
    }

    #[test]
    fn set_draft_item_state_errors_on_missing_item() {
        let mut doc = CodeDraftsDoc::default();
        let err = set_draft_item_state(&mut doc, "missing", "work").expect_err("expected error");
        assert!(err.contains("draft item not found"));
    }

    #[test]
    fn move_job_task_item_rehomes_without_duplicates() {
        let mut job = JobDoc::default();
        job.task.planned.push("todo_create".to_string());
        move_job_task_item(&mut job, "todo_create", "work").expect("move work");
        move_job_task_item(&mut job, "todo_create", "check").expect("move check");
        assert!(job.task.planned.is_empty());
        assert!(job.task.work.is_empty());
        assert_eq!(job.task.check, vec!["todo_create".to_string()]);
    }

    #[test]
    fn job_task_state_change_accepts_alias() {
        let mut job = JobDoc::default();
        job_task_state_change(&mut job, "todo_create", "worked").expect("worked alias");
        assert_eq!(job.task.work, vec!["todo_create".to_string()]);
    }
}
