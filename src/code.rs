use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const CODE_SUBCOMMAND_TIMEOUT_SEC: u64 = 600;
const IMPL_DRAFT_LLM_SOFT_TIMEOUT_SEC: u64 = 180;
const IMPL_DRAFT_LLM_STALL_TIMEOUT_SEC: u64 = 180;
const IMPL_DRAFT_LLM_HARD_TIMEOUT_SEC: u64 = 900;
const LONG_WAIT_REPORT_SEC: u64 = 60;
const ADD_ORC_DRAFTS_SOFT_TIMEOUT_SEC: u64 = 150;
const CREATE_JOB_MD_TIMEOUT_SEC: u64 = 120;

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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub(crate) struct JobChecklistSection {
    pub name: String,
    #[serde(default)]
    pub items: Vec<String>,
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
    #[serde(default)]
    pub checklist: Vec<String>,
    #[serde(default)]
    pub check_sections: Vec<JobChecklistSection>,
    #[serde(default)]
    pub check_evidence: Vec<String>,
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
struct ProjectMdMeta {
    architecture: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ArchitectureContract {
    skill_id: String,
    skill_path: PathBuf,
    constraints: Vec<String>,
    checks: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CheckCodeSkillSpec {
    skill_path: PathBuf,
    uses_logic: bool,
    uses_ui: bool,
    uses_persistence: bool,
    uses_reentry: bool,
    uses_negative: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ImplDraftProgressSnapshot {
    pub draft_name: String,
    pub status: String,
    pub elapsed_sec: u64,
    pub detail: String,
    pub updated_at: u64,
    pub soft_timeout_sec: u64,
    pub stall_timeout_sec: u64,
    pub hard_timeout_sec: u64,
}

// --- Path Utilities ---

pub(crate) fn job_md_path() -> PathBuf {
    PathBuf::from("job.md")
}

fn job_md_path_from(root: &Path) -> PathBuf {
    root.join("job.md")
}

pub(crate) fn drafts_yaml_path() -> PathBuf {
    Path::new(".project").join("drafts.yaml")
}

fn drafts_yaml_path_from(root: &Path) -> PathBuf {
    root.join(".project").join("drafts.yaml")
}

fn impl_progress_runtime_dir(root: &Path) -> PathBuf {
    root.join(".project").join("runtime").join("impl_progress")
}

fn impl_progress_index_path(root: &Path) -> PathBuf {
    root.join(".project")
        .join("runtime")
        .join("impl_progress.json")
}

fn ensure_project_dir_from(root: &Path) -> Result<PathBuf, String> {
    let dir = root.join(".project");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create .project: {}", e))?;
    Ok(dir)
}

fn project_md_path_from(root: &Path) -> PathBuf {
    root.join(".project").join("project.md")
}

fn now_unix_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn impl_progress_file_name(draft_name: &str) -> String {
    let normalized = normalize_feature_key(draft_name);
    if normalized.is_empty() {
        "unknown.json".to_string()
    } else {
        format!("{}.json", normalized)
    }
}

fn save_impl_progress_snapshot(
    root: &Path,
    snapshot: &ImplDraftProgressSnapshot,
) -> Result<(), String> {
    let runtime_dir = impl_progress_runtime_dir(root);
    fs::create_dir_all(&runtime_dir)
        .map_err(|e| format!("failed to create impl progress runtime dir: {}", e))?;
    let snapshot_path = runtime_dir.join(impl_progress_file_name(&snapshot.draft_name));
    let body = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("failed to encode impl progress snapshot: {}", e))?;
    fs::write(&snapshot_path, body)
        .map_err(|e| format!("failed to write impl progress snapshot: {}", e))?;

    let mut snapshots = load_impl_progress_snapshots(root)?;
    snapshots.retain(|item| item.draft_name != snapshot.draft_name);
    snapshots.push(snapshot.clone());
    snapshots.sort_by(|a, b| a.draft_name.cmp(&b.draft_name));
    let index_body = serde_json::to_string_pretty(&snapshots)
        .map_err(|e| format!("failed to encode impl progress index: {}", e))?;
    fs::write(impl_progress_index_path(root), index_body)
        .map_err(|e| format!("failed to write impl progress index: {}", e))?;
    Ok(())
}

fn load_impl_progress_snapshots(root: &Path) -> Result<Vec<ImplDraftProgressSnapshot>, String> {
    let path = impl_progress_index_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read impl progress index: {}", e))?;
    serde_json::from_str(&raw).map_err(|e| format!("failed to parse impl progress index: {}", e))
}

pub(crate) fn ensure_project_dir() -> Result<PathBuf, String> {
    ensure_project_dir_from(Path::new("."))
}

// --- IO Utilities ---

pub(crate) fn load_job_doc() -> Result<JobDoc, String> {
    load_job_doc_from(Path::new("."))
}

fn load_job_doc_from(root: &Path) -> Result<JobDoc, String> {
    let path = job_md_path_from(root);
    if !path.exists() {
        return Ok(JobDoc::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("failed to read job.md: {}", e))?;
    parse_job_md(&raw)
}

pub(crate) fn save_job_doc(doc: &JobDoc) -> Result<(), String> {
    save_job_doc_from(Path::new("."), doc)
}

fn save_job_doc_from(root: &Path, doc: &JobDoc) -> Result<(), String> {
    let path = job_md_path_from(root);
    let body = render_job_md(doc);
    fs::write(&path, body).map_err(|e| format!("failed to write job.md: {}", e))
}

pub(crate) fn load_drafts_doc() -> Result<CodeDraftsDoc, String> {
    load_drafts_doc_from(Path::new("."))
}

fn load_drafts_doc_from(root: &Path) -> Result<CodeDraftsDoc, String> {
    let path = drafts_yaml_path_from(root);
    if !path.exists() {
        return Ok(CodeDraftsDoc::default());
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("failed to read drafts.yaml: {}", e))?;
    serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse drafts.yaml: {}", e))
}

fn backup_dir_from(root: &Path) -> PathBuf {
    root.join(".project").join("runtime").join("backups")
}

fn backup_file_with_label(root: &Path, source: &Path, label: &str) -> Result<PathBuf, String> {
    let dir = backup_dir_from(root);
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create backup dir: {}", e))?;
    let file_name = source
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "artifact".to_string());
    let target = dir.join(format!("{}-{}-{}", now_unix_sec(), label, file_name));
    fs::copy(source, &target).map_err(|e| {
        format!(
            "failed to back up {} to {}: {}",
            source.display(),
            target.display(),
            e
        )
    })?;
    Ok(target)
}

fn normalize_job_md_headings(raw: &str) -> String {
    raw.lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.eq_ignore_ascii_case("# requriements")
                || trimmed.eq_ignore_ascii_case("# requirements")
            {
                "# requirement".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_job_doc_for_drafts(root: &Path) -> Result<JobDoc, String> {
    let path = job_md_path_from(root);
    if !path.exists() {
        return Ok(JobDoc::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("failed to read job.md: {}", e))?;
    let heading_normalized = normalize_job_md_headings(&raw);
    let normalized = normalize_job_md_content(&heading_normalized)?;
    let doc = parse_job_md(&normalized)?;
    let canonical = render_job_md(&doc);
    if canonical != raw {
        fs::write(&path, canonical).map_err(|e| format!("failed to normalize job.md: {}", e))?;
    }
    Ok(doc)
}

fn backup_stale_drafts_if_needed(root: &Path, job: &JobDoc) -> Result<CodeDraftsDoc, String> {
    let path = drafts_yaml_path_from(root);
    if !path.exists() {
        return Ok(CodeDraftsDoc::default());
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("failed to read drafts.yaml: {}", e))?;
    let current_keys: Vec<String> = job
        .requirement
        .iter()
        .map(|req| normalize_feature_key(&req.name))
        .filter(|key| !key.is_empty())
        .collect();
    match serde_yaml::from_str::<CodeDraftsDoc>(&raw) {
        Ok(doc) => {
            let has_explicit_draft_key = raw
                .lines()
                .any(|line| line.trim_start().starts_with("draft:"));
            if !raw.trim().is_empty() && doc.draft.is_empty() && !has_explicit_draft_key {
                let backup = backup_file_with_label(root, &path, "legacy-drafts")?;
                fs::write(&path, "draft: []\n")
                    .map_err(|e| format!("failed to reset legacy drafts.yaml: {}", e))?;
                append_job_problem(
                    root,
                    &format!(
                        "legacy drafts artifact backed up before add_orc_drafts: {}",
                        backup.display()
                    ),
                )?;
                return Ok(CodeDraftsDoc::default());
            }
            if current_keys.is_empty() || doc.draft.is_empty() {
                return Ok(doc);
            }
            let has_overlap = doc
                .draft
                .iter()
                .any(|draft| current_keys.iter().any(|key| key == &draft.name));
            if has_overlap {
                return Ok(doc);
            }
            let backup = backup_file_with_label(root, &path, "stale-drafts")?;
            fs::write(&path, "draft: []\n")
                .map_err(|e| format!("failed to reset stale drafts.yaml: {}", e))?;
            append_job_problem(
                root,
                &format!(
                    "stale drafts artifact backed up before add_orc_drafts: {}",
                    backup.display()
                ),
            )?;
            Ok(CodeDraftsDoc::default())
        }
        Err(_) => {
            let backup = backup_file_with_label(root, &path, "legacy-drafts")?;
            fs::write(&path, "draft: []\n")
                .map_err(|e| format!("failed to reset legacy drafts.yaml: {}", e))?;
            append_job_problem(
                root,
                &format!(
                    "legacy drafts artifact backed up before add_orc_drafts: {}",
                    backup.display()
                ),
            )?;
            Ok(CodeDraftsDoc::default())
        }
    }
}

pub(crate) fn save_drafts_doc(doc: &CodeDraftsDoc) -> Result<(), String> {
    save_drafts_doc_from(Path::new("."), doc)
}

fn save_drafts_doc_from(root: &Path, doc: &CodeDraftsDoc) -> Result<(), String> {
    let path = drafts_yaml_path_from(root);
    ensure_project_dir_from(root)?;
    let raw =
        serde_yaml::to_string(doc).map_err(|e| format!("failed to encode drafts.yaml: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write drafts.yaml: {}", e))
}

pub(crate) fn read_template(name: &str) -> Result<String, String> {
    let path = crate::source_root()
        .join("assets")
        .join("templates")
        .join(name);
    fs::read_to_string(&path).map_err(|e| format!("failed to read template {}: {}", name, e))
}

pub(crate) fn read_prompt(name: &str) -> Result<String, String> {
    let path = crate::source_root()
        .join("assets")
        .join("prompts")
        .join(name);
    fs::read_to_string(&path).map_err(|e| format!("failed to read prompt {}: {}", name, e))
}

// --- Core Workflow Functions ---

/// Step 1: Initialize project.md
pub(crate) fn init_orc_project(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args);
    let project_root = resolve_project_root(&opts)?;
    ensure_project_dir_from(&project_root)?;
    let target = project_root.join(".project").join("project.md");
    let body = build_initial_project_md(&opts, &project_root)?;
    fs::write(&target, body).map_err(|e| format!("failed to write project.md: {}", e))?;

    if opts.auto {
        let bootstrap_output = run_project_bootstrap(&project_root)?;
        let bootstrap_report_path = project_root.join(".project").join("bootstrap.md");
        fs::write(&bootstrap_report_path, bootstrap_output)
            .map_err(|e| format!("failed to write .project/bootstrap.md: {}", e))?;
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
    init_orc_job_from(Path::new("."))
}

fn init_orc_job_from(root: &Path) -> Result<String, String> {
    let path = job_md_path_from(root);
    let mut doc = load_job_doc_from(root)?;
    let had_existing_job = path.exists();
    if has_non_empty_requirements(&doc) {
        return Ok("job.md already has requirement".to_string());
    }

    doc.requirement = collect_project_md_requirements(root)?;
    save_job_doc_from(root, &doc)?;
    if had_existing_job {
        Ok("init_orc_job repaired requirement".to_string())
    } else {
        Ok("init_orc_job completed".to_string())
    }
}

pub(crate) fn create_job_md() -> Result<String, String> {
    let project_md_path = project_md_path_from(Path::new("."));
    if !project_md_path.exists() {
        return Err("missing .project/project.md".to_string());
    }
    let (seed_path, source_label) = resolve_job_md_source_path()?;

    let prompt_template = read_job_md_prompt_template()?;
    let project_md = fs::read_to_string(&project_md_path)
        .map_err(|e| format!("failed to read {}: {}", project_md_path.display(), e))?;
    let architecture_contract = load_architecture_contract_from_root(Path::new("."))?;
    let seed_body = fs::read_to_string(&seed_path)
        .map_err(|e| format!("failed to read {}: {}", seed_path.display(), e))?;
    let prompt = build_create_job_md_prompt(
        &prompt_template,
        &project_md,
        architecture_contract.as_ref(),
        source_label,
        &seed_body,
    );

    let raw = crate::run_codex_exec_capture_with_timeout(&prompt, CREATE_JOB_MD_TIMEOUT_SEC)?;
    let normalized = normalize_job_md_content(&raw)?;
    fs::write(Path::new(crate::JOB_MD_PATH), normalized)
        .map_err(|e| format!("failed to write {}: {}", crate::JOB_MD_PATH, e))?;
    Ok("create_job_md completed".to_string())
}

pub(crate) fn create_input_md() -> Result<String, String> {
    add_orc_drafts_from(Path::new("."))?;
    Ok("create_input_md completed".to_string())
}

pub(crate) async fn auto_add_function(message: &str) -> Result<String, String> {
    let normalized = message.trim();
    if normalized.is_empty() {
        return Err("auto_add_function requires non-empty message".to_string());
    }
    let project_md_path = Path::new(".project").join("project.md");
    if !project_md_path.exists() {
        return Err("missing .project/project.md".to_string());
    }
    if !job_md_path().exists() {
        init_orc_job()?;
    }
    merge_message_into_job_requirements(normalized)?;
    let job_result = create_job_md()?;
    let draft_result = add_orc_drafts()?;
    let impl_result = impl_orc_code().await?;
    let check_result = check_orc_code()?;
    Ok(format!(
        "auto_add_function completed | {} | {} | {} | {}",
        job_result, draft_result, impl_result, check_result
    ))
}

fn read_job_md_prompt_template() -> Result<String, String> {
    let candidates = [
        crate::source_root()
            .join("assets")
            .join("presets")
            .join("code")
            .join("prompts")
            .join("build_job_md_auto.txt"),
        crate::source_root()
            .join("assets")
            .join("presets")
            .join("mono")
            .join("prompts")
            .join("build_job_md_auto.txt"),
    ];
    for path in candidates {
        if path.exists() {
            return fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {}", path.display(), e));
        }
    }
    Err("missing build_job_md_auto prompt template".to_string())
}

fn resolve_job_md_source_path() -> Result<(PathBuf, &'static str), String> {
    let job_path = job_md_path();
    if job_path.exists() {
        return Ok((job_path, "job.md"));
    }

    Err("missing job.md (expected seed input, run init_orc_job/create_input_md 이전 단계에서 job.md를 생성하세요)".to_string())
}

fn merge_message_into_job_requirements(message: &str) -> Result<(), String> {
    let mut job = load_job_doc()?;
    let feature_names = auto_feature_names_from_message(message);
    for feature_name in feature_names {
        merge_requirement_rule(&mut job, &feature_name, message);
    }
    save_job_doc(&job)
}

fn auto_feature_names_from_message(message: &str) -> Vec<String> {
    let inferred = infer_initial_features(Some(message));
    let mut features: Vec<String> = inferred
        .into_iter()
        .filter(|feature| feature != "bootstrap_runtime")
        .collect();
    if features.is_empty() {
        let fallback = normalize_feature_key(message);
        if !fallback.is_empty() {
            features.push(fallback);
        }
    }
    if features.is_empty() {
        features.push("auto_requested_feature".to_string());
    }
    features
}

fn merge_requirement_rule(job: &mut JobDoc, feature_name: &str, message: &str) {
    if let Some(existing) = job
        .requirement
        .iter_mut()
        .find(|req| normalize_feature_key(&req.name) == feature_name)
    {
        if !existing.rules.iter().any(|rule| rule.trim() == message) {
            existing.rules.push(message.to_string());
        }
        if existing.name.trim().is_empty() {
            existing.name = feature_name.to_string();
        }
        return;
    }

    job.requirement.push(JobRequirement {
        name: feature_name.to_string(),
        steps: Vec::new(),
        rules: vec![message.to_string()],
    });
}

fn run_project_bootstrap(project_root: &Path) -> Result<String, String> {
    let prompt_template = match read_project_bootstrap_prompt() {
        Ok(prompt) => prompt,
        Err(_) => {
            let seed = read_bootstrap_seed(project_root)?;
            return Ok(format!(
                "BOOTSTRAP_DONE: auto bootstrap placeholder generated for project `{}`",
                seed.name
            ));
        }
    };
    let seed = read_bootstrap_seed(project_root)?;
    let prompt = prompt_template
        .replace("{{project_name}}", &seed.name)
        .replace("{{project_root}}", &seed.root)
        .replace("{{spec}}", &seed.spec)
        .replace("{{preset}}", "code");
    let output = crate::run_codex_exec_capture_with_timeout(&prompt, CODE_SUBCOMMAND_TIMEOUT_SEC)
        .unwrap_or_else(|_| {
            format!(
                "BOOTSTRAP_DONE: auto bootstrap placeholder generated for project `{}`",
                seed.name
            )
        });
    Ok(output.trim().to_string())
}

fn read_project_bootstrap_prompt() -> Result<String, String> {
    let candidates = [
        crate::source_root()
            .join("assets")
            .join("presets")
            .join("code")
            .join("prompts")
            .join("bootstrap.txt"),
        crate::source_root()
            .join("assets")
            .join("presets")
            .join("mono")
            .join("prompts")
            .join("bootstrap.txt"),
    ];
    for path in candidates {
        if path.exists() {
            return fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {}", path.display(), e));
        }
    }
    Err("missing bootstrap prompt".to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BootstrapSeed {
    name: String,
    spec: String,
    root: String,
}

fn read_bootstrap_seed(project_root: &Path) -> Result<BootstrapSeed, String> {
    let project_md_path = project_root.join(".project").join("project.md");
    let raw = fs::read_to_string(&project_md_path)
        .map_err(|e| format!("failed to read {}: {}", project_md_path.display(), e))?;
    let mut seed = BootstrapSeed {
        name: project_root
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string()),
        spec: String::new(),
        root: project_root.display().to_string(),
    };
    for line in raw.lines() {
        let trimmed = line.trim();
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        match key.trim().to_ascii_lowercase().as_str() {
            "name" if !value.trim().is_empty() => seed.name = value.trim().to_string(),
            "spec" => seed.spec = value.trim().to_string(),
            "path" if !value.trim().is_empty() => seed.root = value.trim().to_string(),
            _ => {}
        }
    }
    Ok(seed)
}

fn build_create_job_md_prompt(
    template: &str,
    project_md: &str,
    architecture_contract: Option<&ArchitectureContract>,
    source_label: &str,
    seed_body: &str,
) -> String {
    let mut normalized = format!("{}\n\n# project.md\n{}", template.trim(), project_md.trim());
    if let Some(contract) = architecture_contract {
        let constraints = if contract.constraints.is_empty() {
            "- (none)".to_string()
        } else {
            contract
                .constraints
                .iter()
                .map(|item| format!("- {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let checks = if contract.checks.is_empty() {
            "- (none)".to_string()
        } else {
            contract
                .checks
                .iter()
                .map(|item| format!("- {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        };
        normalized.push_str(&format!(
            "\n\n# architecture skill\nname: {}\npath: {}\n\n## constraints\n{}\n\n## checks\n{}",
            contract.skill_id,
            contract.skill_path.display(),
            constraints,
            checks
        ));
    }
    normalized.push_str(&format!("\n\n# {}\n{}", source_label, seed_body.trim()));
    if !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

fn normalize_job_md_content(raw: &str) -> Result<String, String> {
    let body = if raw.contains("```") {
        crate::extract_markdown_block(raw)
    } else {
        raw.trim().to_string()
    };
    let normalized = body.trim().to_string();
    if normalized.is_empty() {
        return Err("create_job_md received empty output".to_string());
    }
    if normalized.contains("# requirement") {
        return Ok(normalized);
    }
    if let Some(doc) = parse_outline_requirements_to_job_doc(&normalized) {
        return Ok(render_job_md(&doc));
    }
    Ok(normalized)
}

fn parse_outline_requirements_to_job_doc(raw: &str) -> Option<JobDoc> {
    let mut doc = JobDoc::default();
    let mut current = JobRequirement::default();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            if !current.name.trim().is_empty() {
                doc.requirement.push(current);
                current = JobRequirement::default();
            }
            let title = trimmed
                .trim_start_matches('#')
                .trim_start_matches('#')
                .trim();
            if !title.is_empty() {
                current.name = title.to_string();
            }
            continue;
        }
        if trimmed.starts_with("- ") {
            current.rules.push(trimmed[2..].trim().to_string());
            continue;
        }
        if trimmed.starts_with("> ") {
            current.steps.push(trimmed[2..].trim().to_string());
        }
    }

    if !current.name.trim().is_empty() {
        doc.requirement.push(current);
    }

    if doc.requirement.is_empty() {
        None
    } else {
        Some(doc)
    }
}

/// Step 4: Add Drafts from Job
pub(crate) fn add_orc_drafts() -> Result<String, String> {
    add_orc_drafts_from(Path::new("."))
}

fn add_orc_drafts_from(root: &Path) -> Result<String, String> {
    let mut job = normalize_job_doc_for_drafts(root)?;
    let mut drafts = backup_stale_drafts_if_needed(root, &job)?;
    let architecture_contract = match load_architecture_contract_from_root(root) {
        Ok(value) => value,
        Err(error) => {
            append_job_problem(root, &error)?;
            return Err(error);
        }
    };

    let mut added = 0;
    let mut skipped_due_budget = 0;
    let started_at = Instant::now();
    for req in &job.requirement {
        if started_at.elapsed().as_secs() >= ADD_ORC_DRAFTS_SOFT_TIMEOUT_SEC {
            skipped_due_budget += 1;
            continue;
        }
        let key = normalize_feature_key(&req.name);
        if key.is_empty() {
            continue;
        }
        if !job.task.planned.contains(&key) {
            job.task.planned.push(key.clone());
        }
        if drafts.draft.iter().any(|d| d.name == key) {
            continue;
        }

        let mut item = build_draft_item_from_requirement(req);
        if let Some(contract) = architecture_contract.as_ref() {
            merge_architecture_contract_into_draft_item(&mut item, contract);
        }
        item.name = key.clone();
        item.state = "planned".to_string();
        drafts.draft.push(item);
        added += 1;
    }

    if let Some(contract) = architecture_contract.as_ref() {
        for draft in drafts.draft.iter_mut() {
            merge_architecture_contract_into_draft_item(draft, contract);
        }
    }

    ensure_add_orc_drafts_produced_targets(&job, &drafts)?;

    save_job_doc_from(root, &job)?;
    save_drafts_doc_from(root, &drafts)?;
    Ok(format!(
        "add_orc_drafts completed: added {} items, deferred {} items (budget)",
        added, skipped_due_budget
    ))
}

fn ensure_add_orc_drafts_produced_targets(
    job: &JobDoc,
    drafts: &CodeDraftsDoc,
) -> Result<(), String> {
    if drafts.draft.is_empty() {
        if job.requirement.is_empty() {
            return Err(
                "add_orc_drafts produced 0 draft items: job.md requirement section is empty"
                    .to_string(),
            );
        }
        let requirement_names: Vec<String> = job
            .requirement
            .iter()
            .map(|req| req.name.trim())
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string())
            .collect();
        if requirement_names.is_empty() {
            return Err(
                "add_orc_drafts produced 0 draft items: job.md requirements have no usable names"
                    .to_string(),
            );
        }
        return Err(format!(
            "add_orc_drafts produced 0 draft items from requirements: {}",
            requirement_names.join(", ")
        ));
    }

    let actionable_count = drafts
        .draft
        .iter()
        .filter(|item| matches!(item.state.as_str(), "planned" | "work" | "worked"))
        .count();
    if actionable_count == 0 {
        return Err(
            "add_orc_drafts produced 0 actionable draft items: drafts.yaml has no planned/work items"
                .to_string(),
        );
    }

    Ok(())
}

fn build_draft_item_from_requirement(req: &JobRequirement) -> DraftItemDoc {
    let key = normalize_feature_key(&req.name);
    let normalized_steps: Vec<String> = req
        .steps
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let normalized_rules: Vec<String> = req
        .rules
        .iter()
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
        .map(|r| r.to_string())
        .collect();
    DraftItemDoc {
        name: key.clone(),
        state: "planned".to_string(),
        item_type: "action".to_string(),
        domain: vec!["core".to_string()],
        depends_on: vec![],
        scope: vec![],
        rule: normalized_rules,
        step: if normalized_steps.is_empty() {
            vec!["trigger -> process -> result".to_string()]
        } else {
            normalized_steps
        },
        tasks: vec![format!("implement {}", key)],
        constraints: vec![format!(
            "{} -> {} : requirement 기반 draft item 생성",
            key, key
        )],
        check: vec![format!("verify {}", key)],
    }
}

/// Step 5: Implement Code (Parallel)
pub(crate) async fn impl_orc_code() -> Result<String, String> {
    let mut drafts = load_drafts_doc()?;
    let mut job = load_job_doc()?;
    let targets: Vec<DraftItemDoc> = drafts
        .draft
        .iter()
        .filter(|d| d.state == "planned" || d.state == "work" || d.state == "worked")
        .cloned()
        .collect();

    if targets.is_empty() {
        return Ok("no drafts to implement".to_string());
    }

    (drafts, job) = transition_impl_start(&drafts, &job, &targets)?;
    save_drafts_doc(&drafts)?;
    save_job_doc(&job)?;

    let result = impl_orc_code_parallel(targets).await?;
    (drafts, job) = transition_impl_result(&drafts, &job, &result.succeeded, &result.failed)?;
    save_drafts_doc(&drafts)?;
    save_job_doc(&job)?;
    Ok(format!(
        "impl_orc_code completed: success={} fail={}",
        result.succeeded.len(),
        result.failed.len()
    ))
}

/// Step 6: Check Code
pub(crate) fn check_orc_code() -> Result<String, String> {
    let check_skill = load_check_code_skill_spec()?;
    let mut job = load_job_doc()?;
    let drafts = load_drafts_doc()?;
    let architecture_contract = match load_architecture_contract_from_root(Path::new(".")) {
        Ok(value) => value,
        Err(error) => {
            append_job_problem(Path::new("."), &error)?;
            return Err(error);
        }
    };
    let (check_sections, checklist) =
        build_job_checklist(&job, &drafts, architecture_contract.as_ref(), &check_skill);
    job.check_sections = check_sections;
    job.checklist = checklist;
    let check_evidence = if job.task.check.is_empty() {
        None
    } else {
        Some(parse_check_evidence_lines(&job.check_evidence)?)
    };
    let gate_evaluation = if job.task.check.is_empty() {
        None
    } else {
        Some(crate::check_gate::evaluate_hard_gates(
            &crate::check_gate::HardGateInput {
                verify_targets: job.task.check.clone(),
                problems: job.problems.clone(),
                check_section_names: job
                    .check_sections
                    .iter()
                    .map(|section| section.name.clone())
                    .collect(),
                check_evidence_lines: job.check_evidence.clone(),
            },
        )?)
    };

    if let Some(evidence) = check_evidence.as_ref() {
        for item in &evidence.unresolved {
            push_unique(
                &mut job.problems,
                format!("- job.md check evidence unresolved: {}", item),
            );
        }
    }
    if let Some(gate) = gate_evaluation.as_ref() {
        for item in &gate.failures {
            push_unique(&mut job.problems, format!("- hard gate: {}", item));
        }
    }

    let has_unresolved_problems =
        job.problems.iter().any(|item| !item.trim().is_empty()) || !job.task.fail.is_empty();
    if !has_unresolved_problems {
        let verify_targets = job.task.check.clone();
        for name in verify_targets {
            move_job_task_item(&mut job, &name, "complete")?;
        }
    }
    save_job_doc(&job)?;
    let removed = cleanup_drafts_yaml_after_success(Path::new("."))?;
    if removed {
        Ok(format!(
            "check_orc_code completed: verify={} checklist={} execution={} unresolved={} hard_gate_failed={} mode={} skill={} drafts.yaml removed",
            job.task.completed.len(),
            job.checklist.len(),
            check_evidence.as_ref().map_or(0, |value| value.checked.len()),
            job.problems.len() + job.task.fail.len(),
            gate_evaluation.as_ref().map_or(0, |value| value.failures.len()),
            gate_evaluation
                .as_ref()
                .and_then(|value| value.mode)
                .map(|value| value.as_str())
                .unwrap_or("none"),
            check_skill.skill_path.display()
        ))
    } else {
        Ok(format!(
            "check_orc_code completed: verify={} checklist={} execution={} unresolved={} hard_gate_failed={} mode={} skill={}",
            job.task.completed.len(),
            job.checklist.len(),
            check_evidence.as_ref().map_or(0, |value| value.checked.len()),
            job.problems.len() + job.task.fail.len(),
            gate_evaluation.as_ref().map_or(0, |value| value.failures.len()),
            gate_evaluation
                .as_ref()
                .and_then(|value| value.mode)
                .map(|value| value.as_str())
                .unwrap_or("none"),
            check_skill.skill_path.display()
        ))
    }
}

fn cleanup_drafts_yaml_after_success(root: &Path) -> Result<bool, String> {
    let job = load_job_doc_from(root)?;
    let drafts = load_drafts_doc_from(root)?;
    if !should_cleanup_drafts_yaml(&job, &drafts) {
        return Ok(false);
    }
    let path = drafts_yaml_path_from(root);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path).map_err(|e| format!("failed to remove {}: {}", path.display(), e))?;
    Ok(true)
}

fn should_cleanup_drafts_yaml(job: &JobDoc, drafts: &CodeDraftsDoc) -> bool {
    if job.task.completed.is_empty() {
        return false;
    }
    if !job.task.planned.is_empty() || !job.task.work.is_empty() || !job.task.check.is_empty() {
        return false;
    }
    if !job.task.fail.is_empty() {
        return false;
    }
    if drafts.draft.is_empty() {
        return false;
    }
    drafts.draft.iter().all(|item| {
        normalize_draft_state(item.state.as_str()).map_or(false, |state| {
            state == "complete"
                && job.task.completed.iter().any(|name| {
                    normalize_feature_key(name) == normalize_feature_key(&item.name)
                })
        })
    })
}

pub(crate) fn get_workspace_state(root: &Path) -> &'static str {
    let project_md = root.join(".project").join("project.md");
    let job_md = root.join("job.md");
    let drafts_yaml = root.join(".project").join("drafts.yaml");

    if !project_md.exists() {
        return "uninitialized";
    }
    if !job_md.exists() {
        return "initialized";
    }
    if !drafts_yaml.exists() {
        return "configured";
    }
    "ready"
}

pub(crate) fn flow_rust_orchestra(root: &Path, args: &[String]) -> Result<String, String> {
    if !args.is_empty() {
        return Err(format!(
            "cli_rust_orchestra does not accept arguments: {}",
            args.join(" ")
        ));
    }

    let state = get_workspace_state(root);
    if state != "ready" {
        return Err(format!("workspace state is {} (required: ready)", state));
    }

    add_orc_drafts_from(root)?;
    ensure_draft_item_exists(root, "cli_rust_orchestra")?;

    Ok(build_rust_orchestra_result(state))
}

fn build_rust_orchestra_result(state: &str) -> String {
    format!(
        "trigger: {} -> process: validate_workspace+add_orc_drafts -> result: cli_rust_orchestra completed",
        state
    )
}

fn ensure_draft_item_exists(root: &Path, name: &str) -> Result<(), String> {
    let drafts = load_drafts_doc_from(root)?;
    if drafts.draft.iter().any(|item| item.name == name) {
        return Ok(());
    }
    Err(format!(
        "required draft item not found after add_orc_drafts: {}",
        name
    ))
}

// --- Internal Implementation Helpers ---

fn parse_job_md(raw: &str) -> Result<JobDoc, String> {
    let mut doc = JobDoc::default();
    let mut section = "";
    let mut req = JobRequirement::default();
    let mut task_list = "";
    let mut checklist_section = String::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# plan") {
            section = "plan";
            continue;
        } else if trimmed.starts_with("# requirement") {
            section = "req";
            continue;
        } else if trimmed.starts_with("# task") {
            if !req.name.is_empty() {
                doc.requirement.push(req.clone());
                req = JobRequirement::default();
            }
            section = "task";
            continue;
        } else if trimmed.starts_with("# problems") {
            if !req.name.is_empty() {
                doc.requirement.push(req.clone());
                req = JobRequirement::default();
            }
            section = "prob";
            continue;
        } else if trimmed.starts_with("# check evidence") {
            if !req.name.is_empty() {
                doc.requirement.push(req.clone());
                req = JobRequirement::default();
            }
            section = "check_evidence";
            continue;
        } else if trimmed.starts_with("# check") {
            if !req.name.is_empty() {
                doc.requirement.push(req.clone());
                req = JobRequirement::default();
            }
            section = "checklist";
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
                    if !req.name.is_empty() {
                        doc.requirement.push(req.clone());
                    }
                    req = JobRequirement {
                        name: trimmed[3..].trim().to_string(),
                        ..Default::default()
                    };
                } else if trimmed.starts_with("- ") {
                    req.rules.push(trimmed[2..].trim().to_string());
                } else if trimmed.starts_with("> ") {
                    req.steps.push(trimmed[2..].trim().to_string());
                } else if trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
                    && trimmed.contains('.')
                {
                    req.steps
                        .push(trimmed.split_once('.').unwrap().1.trim().to_string());
                }
            }
            "task" => {
                if trimmed.starts_with("## planned") {
                    task_list = "planned";
                } else if trimmed.starts_with("## work") {
                    task_list = "work";
                } else if trimmed.starts_with("## verify") || trimmed.starts_with("## check") {
                    task_list = "check";
                } else if trimmed.starts_with("## complete") || trimmed.starts_with("## completed")
                {
                    task_list = "completed";
                } else if trimmed.starts_with("## fail") {
                    task_list = "fail";
                } else if trimmed.starts_with("- ") {
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
                if trimmed.starts_with("- ") {
                    doc.problems.push(trimmed.to_string());
                }
            }
            "checklist" => {
                if trimmed.starts_with("## ") {
                    checklist_section = trimmed[3..].trim().to_string();
                    if !checklist_section.is_empty()
                        && !doc
                            .check_sections
                            .iter()
                            .any(|entry| entry.name == checklist_section)
                    {
                        doc.check_sections.push(JobChecklistSection {
                            name: checklist_section.clone(),
                            items: Vec::new(),
                        });
                    }
                } else if trimmed.starts_with("- ") {
                    let item = trimmed[2..].trim().to_string();
                    doc.checklist.push(item.clone());
                    if !checklist_section.is_empty() {
                        if let Some(entry) = doc
                            .check_sections
                            .iter_mut()
                            .find(|entry| entry.name == checklist_section)
                        {
                            entry.items.push(item);
                        }
                    }
                } else if trimmed.starts_with('#') {
                    checklist_section.clear();
                }
            }
            "check_evidence" => {
                if trimmed.starts_with("- [") {
                    doc.check_evidence.push(trimmed.to_string());
                }
            }
            _ => {}
        }
    }
    if !req.name.is_empty() {
        doc.requirement.push(req);
    }
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
        for (i, s) in r.steps.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, s));
        }
        for ru in &r.rules {
            out.push_str(&format!("- {}\n", ru));
        }
    }
    out.push_str("\n# task\n");
    out.push_str("## planned\n");
    for i in &doc.task.planned {
        out.push_str(&format!("- {}\n", i));
    }
    out.push_str("## work\n");
    for i in &doc.task.work {
        out.push_str(&format!("- {}\n", i));
    }
    out.push_str("## verify\n");
    for i in &doc.task.check {
        out.push_str(&format!("- {}\n", i));
    }
    out.push_str("## complete\n");
    for i in &doc.task.completed {
        out.push_str(&format!("- {}\n", i));
    }
    out.push_str("## fail\n");
    for i in &doc.task.fail {
        out.push_str(&format!("- {}\n", i));
    }
    out.push_str("\n# problems\n");
    for p in &doc.problems {
        out.push_str(&format!("{}\n", p));
    }
    out.push_str("\n# check\n");
    if doc.check_sections.is_empty() {
        for item in &doc.checklist {
            out.push_str(&format!("- {}\n", item));
        }
    } else {
        let mut remaining = doc.checklist.clone();
        for section in &doc.check_sections {
            out.push_str(&format!("## {}\n", section.name));
            for item in &section.items {
                out.push_str(&format!("- {}\n", item));
                if let Some(index) = remaining.iter().position(|entry| entry == item) {
                    remaining.remove(index);
                }
            }
        }
        if !remaining.is_empty() {
            out.push_str("## checklist\n");
            for item in &remaining {
                out.push_str(&format!("- {}\n", item));
            }
        }
    }
    out.push_str("\n# check evidence\n");
    for item in &doc.check_evidence {
        out.push_str(&format!("{}\n", item));
    }
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
        "check" | "verify" => doc.task.check.push(name.to_string()),
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

pub(crate) fn move_job_task_item(
    doc: &mut JobDoc,
    name: &str,
    to_list: &str,
) -> Result<(), String> {
    job_task_state_change(doc, name, to_list)
}

fn transition_impl_start(
    drafts: &CodeDraftsDoc,
    job: &JobDoc,
    targets: &[DraftItemDoc],
) -> Result<(CodeDraftsDoc, JobDoc), String> {
    let mut next_drafts = drafts.clone();
    let mut next_job = job.clone();
    for item in targets {
        set_draft_item_state(&mut next_drafts, &item.name, "work")?;
        move_job_task_item(&mut next_job, &item.name, "work")?;
    }
    Ok((next_drafts, next_job))
}

fn transition_impl_result(
    drafts: &CodeDraftsDoc,
    job: &JobDoc,
    succeeded: &[String],
    failed: &[(String, String)],
) -> Result<(CodeDraftsDoc, JobDoc), String> {
    let mut next_drafts = drafts.clone();
    let mut next_job = job.clone();

    for name in succeeded {
        set_draft_item_state(&mut next_drafts, name, "complete")?;
        move_job_task_item(&mut next_job, name, "check")?;
    }
    for (name, reason) in failed {
        set_draft_item_state(&mut next_drafts, name, "error")?;
        move_job_task_item(&mut next_job, name, "fail")?;
        let trimmed_reason = reason.trim();
        if !trimmed_reason.is_empty() {
            next_job
                .problems
                .push(format!("- {} : {}", name, trimmed_reason));
        }
    }

    Ok((next_drafts, next_job))
}

fn push_unique(list: &mut Vec<String>, item: String) {
    let trimmed = item.trim();
    if trimmed.is_empty() {
        return;
    }
    if !list.iter().any(|existing| existing == trimmed) {
        list.push(trimmed.to_string());
    }
}

fn find_requirement_for_task<'a>(job: &'a JobDoc, task_name: &str) -> Option<&'a JobRequirement> {
    let task_key = normalize_feature_key(task_name);
    job.requirement
        .iter()
        .find(|requirement| normalize_feature_key(&requirement.name) == task_key)
}

fn find_draft_for_task<'a>(drafts: &'a CodeDraftsDoc, task_name: &str) -> Option<&'a DraftItemDoc> {
    let task_key = normalize_feature_key(task_name);
    drafts
        .draft
        .iter()
        .find(|draft| normalize_feature_key(&draft.name) == task_key)
}

fn format_checklist_entry(input: &str, output: &str, description: &str) -> String {
    format!(
        "{} -> {} : {}",
        input.trim(),
        output.trim(),
        description.trim()
    )
}

fn classify_checklist_entry(entry: &str) -> &'static str {
    let lowered = entry.to_ascii_lowercase();
    if lowered.contains("reload")
        || lowered.contains("restart")
        || lowered.contains("reopen")
        || lowered.contains("re-entry")
        || lowered.contains("reentry")
    {
        return "reentry_checklist";
    }
    if lowered.contains("persist")
        || lowered.contains("save")
        || lowered.contains("storage")
        || lowered.contains("stored")
    {
        return "persistence_checklist";
    }
    if lowered.contains("forbid")
        || lowered.contains("negative")
        || lowered.contains("reject")
        || lowered.contains("prevent")
        || lowered.contains("duplicate")
        || lowered.contains("missing")
    {
        return "negative_checklist";
    }
    if lowered.contains("render")
        || lowered.contains("visible")
        || lowered.contains("layout")
        || lowered.contains("current.png")
        || lowered.contains("browser")
        || lowered.contains("ui")
    {
        return "ui_checklist";
    }
    "logic_checklist"
}

fn normalize_problem_check_entry(problem: &str) -> String {
    let trimmed = problem.trim().trim_start_matches("- ").trim();
    if trimmed.contains("->") && trimmed.contains(':') {
        return trimmed.to_string();
    }
    let input = trimmed
        .split_once(':')
        .map(|(left, _)| left.trim())
        .filter(|left| !left.is_empty())
        .unwrap_or("problem");
    format_checklist_entry(input, "resolved", trimmed)
}

fn build_job_checklist(
    job: &JobDoc,
    drafts: &CodeDraftsDoc,
    architecture_contract: Option<&ArchitectureContract>,
    check_skill: &CheckCodeSkillSpec,
) -> (Vec<JobChecklistSection>, Vec<String>) {
    let mut checklist = Vec::new();

    for problem in &job.problems {
        push_unique(&mut checklist, normalize_problem_check_entry(problem));
    }

    for task_name in &job.task.check {
        if let Some(draft) = find_draft_for_task(drafts, task_name) {
            for item in &draft.check {
                let entry = if item.contains("->") && item.contains(':') {
                    item.trim().to_string()
                } else {
                    format_checklist_entry(task_name, "verified", item)
                };
                push_unique(&mut checklist, entry);
            }
            for item in &draft.constraints {
                let entry = if item.contains("->") && item.contains(':') {
                    item.trim().to_string()
                } else {
                    format_checklist_entry(task_name, "verified", item)
                };
                push_unique(&mut checklist, entry);
            }
        }

        if let Some(requirement) = find_requirement_for_task(job, task_name) {
            for step in &requirement.steps {
                push_unique(
                    &mut checklist,
                    format_checklist_entry(task_name, "verified", step),
                );
            }
            for rule in &requirement.rules {
                push_unique(
                    &mut checklist,
                    format_checklist_entry(task_name, "verified", rule),
                );
            }
        }

        if let Some(contract) = architecture_contract {
            for item in &contract.constraints {
                push_unique(
                    &mut checklist,
                    format_checklist_entry(task_name, "verified", item),
                );
            }
            for item in &contract.checks {
                push_unique(
                    &mut checklist,
                    format_checklist_entry(task_name, "verified", item),
                );
            }
        }

        if !checklist
            .iter()
            .any(|item| item.starts_with(&format!("{} ->", task_name.trim())))
        {
            push_unique(
                &mut checklist,
                format_checklist_entry(task_name, "verified", &format!("verify {}", task_name)),
            );
        }
    }

    let mut sections = Vec::new();
    let mut section_names = vec!["logic_checklist".to_string()];
    if check_skill.uses_ui {
        section_names.push("ui_checklist".to_string());
    }
    if check_skill.uses_persistence {
        section_names.push("persistence_checklist".to_string());
    }
    if check_skill.uses_reentry {
        section_names.push("reentry_checklist".to_string());
    }
    if check_skill.uses_negative {
        section_names.push("negative_checklist".to_string());
    }
    for section_name in section_names {
        let items = checklist
            .iter()
            .filter(|entry| classify_checklist_entry(entry) == section_name)
            .cloned()
            .collect::<Vec<_>>();
        if !items.is_empty() || section_name == "logic_checklist" {
            sections.push(JobChecklistSection {
                name: section_name,
                items,
            });
        }
    }

    (sections, checklist)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CheckEvidence {
    checked: Vec<String>,
    unresolved: Vec<String>,
}

fn parse_check_evidence_lines(lines: &[String]) -> Result<CheckEvidence, String> {
    let records = crate::check_gate::parse_evidence_records(lines)?;
    let mut evidence = CheckEvidence::default();
    for record in records {
        if record.checked {
            evidence.checked.push(record.detail);
        } else {
            evidence.unresolved.push(record.detail);
        }
    }
    Ok(evidence)
}

fn append_job_problem(root: &Path, problem: &str) -> Result<(), String> {
    let trimmed = problem.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let mut job = load_job_doc_from(root)?;
    push_unique(&mut job.problems, trimmed.to_string());
    save_job_doc_from(root, &job)
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
        if in_section && line.trim().starts_with('#') {
            in_section = false;
        }
        if !in_section {
            out.push(line.to_string());
        }
    }
    if !found {
        out.push(header.to_string());
        out.push(body.to_string());
    }
    out.join("\n")
}

fn has_non_empty_requirements(doc: &JobDoc) -> bool {
    doc.requirement
        .iter()
        .any(|req| !req.name.trim().is_empty())
}

fn collect_project_md_requirements(root: &Path) -> Result<Vec<JobRequirement>, String> {
    let project_md_path = project_md_path_from(root);
    let project_md = fs::read_to_string(&project_md_path)
        .map_err(|e| format!("failed to read {}: {}", project_md_path.display(), e))?;
    let features = extract_plain_list_under_header(&project_md, "# features");
    let requirements = features
        .into_iter()
        .map(|name| JobRequirement {
            name,
            ..Default::default()
        })
        .filter(|req| !req.name.trim().is_empty())
        .collect::<Vec<_>>();
    if requirements.is_empty() {
        return Err(format!(
            "init_orc_job could not derive requirement from {} # features",
            project_md_path.display()
        ));
    }
    Ok(requirements)
}

fn extract_plain_list_under_header(markdown: &str, header: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with('#') {
            break;
        }
        if in_section && trimmed.starts_with("- ") {
            out.push(trimmed[2..].trim().to_string());
        }
    }
    out
}

fn parse_project_md_meta(markdown: &str) -> ProjectMdMeta {
    let mut meta = ProjectMdMeta::default();
    let mut in_architecture = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("# architecture") {
            in_architecture = true;
            continue;
        }
        if in_architecture && trimmed.starts_with('#') {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            if in_architecture && key.trim().eq_ignore_ascii_case("name") {
                meta.architecture = value.trim().to_string();
            }
        }
    }
    meta
}

fn load_architecture_contract_from_root(
    root: &Path,
) -> Result<Option<ArchitectureContract>, String> {
    let project_md_path = project_md_path_from(root);
    if !project_md_path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&project_md_path)
        .map_err(|e| format!("failed to read {}: {}", project_md_path.display(), e))?;
    let meta = parse_project_md_meta(&raw);
    if meta.architecture.trim().is_empty() {
        return Ok(None);
    }
    let skill_id = meta.architecture.trim().to_string();
    let skill_path = resolve_architecture_skill_path(&skill_id).ok_or_else(|| {
        format!(
            "architecture skill not found: {} (expected /home/tree/ai/skills/{}/SKILL.md or /home/tree/.codex/skills/{}/SKILL.md)",
            skill_id, skill_id, skill_id
        )
    })?;
    let body = fs::read_to_string(&skill_path).map_err(|e| {
        format!(
            "failed to read architecture skill {}: {}",
            skill_path.display(),
            e
        )
    })?;
    parse_architecture_contract(&skill_id, &skill_path, &body).map(Some)
}

fn resolve_architecture_skill_path(skill_id: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new("/home/tree/ai/skills")
            .join(skill_id)
            .join("SKILL.md"),
        Path::new("/home/tree/.codex/skills")
            .join(skill_id)
            .join("SKILL.md"),
    ];
    candidates.into_iter().find(|path| path.exists())
}

fn parse_architecture_contract(
    skill_id: &str,
    skill_path: &Path,
    raw: &str,
) -> Result<ArchitectureContract, String> {
    let mut in_contract = false;
    let mut section = "";
    let mut constraints = Vec::new();
    let mut checks = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## ORC Architecture Contract") {
            in_contract = true;
            section = "";
            continue;
        }
        if in_contract
            && trimmed.starts_with("## ")
            && !trimmed.eq_ignore_ascii_case("## ORC Architecture Contract")
        {
            break;
        }
        if !in_contract {
            continue;
        }
        if trimmed.eq_ignore_ascii_case("### Constraints") {
            section = "constraints";
            continue;
        }
        if trimmed.eq_ignore_ascii_case("### Checks") {
            section = "checks";
            continue;
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            match section {
                "constraints" => constraints.push(item.to_string()),
                "checks" => checks.push(item.to_string()),
                _ => {}
            }
        }
    }

    if constraints.is_empty() && checks.is_empty() {
        return Err(format!(
            "architecture skill missing ORC contract entries: {}",
            skill_path.display()
        ));
    }

    Ok(ArchitectureContract {
        skill_id: skill_id.to_string(),
        skill_path: skill_path.to_path_buf(),
        constraints,
        checks,
    })
}

fn merge_architecture_contract_into_draft_item(
    item: &mut DraftItemDoc,
    contract: &ArchitectureContract,
) {
    for constraint in &contract.constraints {
        push_unique(&mut item.constraints, constraint.clone());
    }
    for check in &contract.checks {
        push_unique(&mut item.check, check.clone());
    }
}

fn normalize_feature_key(name: &str) -> String {
    let mut normalized = String::new();
    let chars: Vec<char> = name.trim().chars().collect();

    for (index, ch) in chars.iter().enumerate() {
        let is_alnum = ch.is_alphanumeric();

        if ch.is_ascii_uppercase() && index > 0 {
            let prev = chars[index - 1];
            let next_is_lower = chars
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_lowercase());
            if prev.is_ascii_lowercase()
                || prev.is_ascii_digit()
                || (prev.is_ascii_uppercase() && next_is_lower)
            {
                normalized.push('_');
            }
        }

        if is_alnum {
            normalized.extend(ch.to_lowercase());
        } else {
            normalized.push('_');
        }
    }

    normalized
        .split('_')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[derive(Default)]
struct CommonOpts {
    name: Option<String>,
    description: Option<String>,
    spec: Option<String>,
    path: Option<String>,
    message: Option<String>,
    auto: bool,
}

fn parse_common_opts(args: &[String]) -> CommonOpts {
    let mut opts = CommonOpts::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                opts.name = args.get(i).cloned();
            }
            "-d" => {
                i += 1;
                opts.description = args.get(i).cloned();
            }
            "-s" => {
                i += 1;
                opts.spec = args.get(i).cloned();
            }
            "-p" => {
                i += 1;
                opts.path = args.get(i).cloned();
            }
            "-m" => {
                i += 1;
                opts.message = args.get(i).cloned();
            }
            "-a" => {
                opts.auto = true;
            }
            _ => {}
        }
        i += 1;
    }
    opts
}

fn resolve_project_root(opts: &CommonOpts) -> Result<PathBuf, String> {
    if let Some(path) = &opts.path {
        let root = PathBuf::from(path);
        fs::create_dir_all(&root)
            .map_err(|e| format!("failed to create project root {}: {}", root.display(), e))?;
        return Ok(root);
    }
    std::env::current_dir().map_err(|e| format!("failed to get current dir: {}", e))
}

fn build_initial_project_md(opts: &CommonOpts, project_root: &Path) -> Result<String, String> {
    let name = opts
        .name
        .clone()
        .or_else(|| {
            project_root
                .file_name()
                .map(|v| v.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "project".to_string());
    let description = opts
        .description
        .clone()
        .or_else(|| opts.message.clone())
        .unwrap_or_default();
    let spec = opts
        .spec
        .clone()
        .or_else(|| infer_spec_from_message(opts.message.as_deref().unwrap_or_default()));

    Ok(render_project_md(
        &name,
        &description,
        &spec.unwrap_or_default(),
        &project_root.display().to_string(),
        &infer_initial_features(opts.message.as_deref()),
    ))
}

fn render_project_md(
    name: &str,
    description: &str,
    spec: &str,
    path: &str,
    features: &[String],
) -> String {
    let feature_block = if features.is_empty() {
        "- bootstrap_runtime".to_string()
    } else {
        features
            .iter()
            .map(|feature| format!("- {}", feature))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "# info\nname: {name}\ndescription: {description}\nspec: {spec}\npath: {path}\nstate: init\n\n# architecture\nname: \n\n# features\n{feature_block}\n\n# rules\n- 프로젝트 내부의 공통 규칙\n\n# constraints\n- 프로젝트 내부의 공통 제약\n\n# domains\n## core\n### states\n- init\n### action\n- bootstrap\n### rules\n- spec 기준으로 초기 실행 환경을 준비한다.\n### constraints\n- 최소 골격만 생성한다.\n"
    )
}

fn infer_spec_from_message(message: &str) -> Option<String> {
    let lower = message.to_ascii_lowercase();
    let known = [
        "rust",
        "cargo",
        "react",
        "vite",
        "next",
        "astro",
        "typescript",
        "javascript",
        "node",
        "express",
        "tauri",
        "svelte",
        "vue",
        "zustand",
    ];
    let tokens: Vec<&str> = known
        .into_iter()
        .filter(|token| lower.contains(token))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(", "))
    }
}

fn infer_initial_features(message: Option<&str>) -> Vec<String> {
    let Some(message) = message else {
        return vec!["bootstrap_runtime".to_string()];
    };
    let lower = message.to_ascii_lowercase();
    let mut features = Vec::new();
    if lower.contains("todo") {
        features.push("todo_app".to_string());
    }
    if lower.contains("hello world") {
        features.push("hello_world_screen".to_string());
    }
    if lower.contains("bootstrap") || features.is_empty() {
        features.push("bootstrap_runtime".to_string());
    }
    if lower.contains("zustand") {
        features.push("zustand_store_setup".to_string());
    }
    features
}

fn load_check_code_skill_spec() -> Result<CheckCodeSkillSpec, String> {
    let skill_path = Path::new("/home/tree/ai/skills/check-code/SKILL.md");
    let prompt_path = crate::source_root().join("assets").join("prompts").join("check_code.md");
    let path = if skill_path.exists() {
        skill_path.to_path_buf()
    } else {
        prompt_path
    };
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read check-code skill {}: {}", path.display(), e))?;
    let lowered = raw.to_ascii_lowercase();
    Ok(CheckCodeSkillSpec {
        skill_path: path,
        uses_logic: lowered.contains("logic_checklist"),
        uses_ui: lowered.contains("ui_checklist"),
        uses_persistence: lowered.contains("persistence_checklist"),
        uses_reentry: lowered.contains("reentry_checklist"),
        uses_negative: lowered.contains("negative_checklist"),
    })
}

// --- Parallel Implementation Logic (Simplified/Modernized) ---

async fn impl_orc_code_parallel(items: Vec<DraftItemDoc>) -> Result<ImplRunResult, String> {
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
    let cwd = std::env::current_dir().map_err(|e| format!("failed to resolve cwd: {}", e))?;
    let started = Instant::now();
    write_impl_draft_progress(&cwd, &item.name, "running", 0, "impl draft started")?;
    let prompt_base = read_prompt("build_parallel.md")?;
    let task_yaml = serde_yaml::to_string(item)
        .map_err(|e| format!("failed to encode draft item yaml: {}", e))?;
    let prompt = format!(
        "{}\n\n# 입력 task 단일 객체\n```yaml\n{}\n```\n\n# 추가 지시\n- `drafts.yaml`, `job.md`는 절대 직접 수정하지 말고 코드 생성/수정 내용만 출력한다.\n- 상태 전이(work/complete/error, planned/work/check/fail 이동)는 Rust 오케스트레이터가 수행하므로 출력하지 않는다.",
        prompt_base, task_yaml
    );
    let trace_label = format!("impl_orc_code [{}]", item.name);
    let raw = crate::chat::run_codex_exec_capture_in_dir_with_progress_watch(
        &cwd,
        &prompt,
        IMPL_DRAFT_LLM_HARD_TIMEOUT_SEC,
        crate::chat::LlmProgressWatch {
            soft_timeout_sec: IMPL_DRAFT_LLM_SOFT_TIMEOUT_SEC,
            stall_timeout_sec: IMPL_DRAFT_LLM_STALL_TIMEOUT_SEC,
            hard_timeout_sec: IMPL_DRAFT_LLM_HARD_TIMEOUT_SEC,
        },
        &trace_label,
        1,
    )
    .map_err(|error| {
        let _ = write_impl_draft_progress(
            &cwd,
            &item.name,
            "failed",
            started.elapsed().as_secs(),
            &error,
        );
        error
    })?;
    if raw.trim().is_empty() {
        let _ = write_impl_draft_progress(
            &cwd,
            &item.name,
            "failed",
            started.elapsed().as_secs(),
            &format!("empty llm output for draft {}", item.name),
        );
        return Err(format!("empty llm output for draft {}", item.name));
    }
    if llm_impl_output_indicates_failure(&raw) {
        let detail = format!(
            "llm reported implementation failure for {}: {}",
            item.name,
            raw.lines().next().unwrap_or("unknown failure")
        );
        let _ = write_impl_draft_progress(
            &cwd,
            &item.name,
            "failed",
            started.elapsed().as_secs(),
            &detail,
        );
        return Err(format!(
            "llm reported implementation failure for {}: {}",
            item.name,
            raw.lines().next().unwrap_or("unknown failure")
        ));
    }
    write_impl_draft_progress(
        &cwd,
        &item.name,
        "completed",
        started.elapsed().as_secs(),
        "impl draft completed",
    )?;
    Ok(())
}

fn impl_timeout_watch() -> (u64, u64, u64) {
    (
        IMPL_DRAFT_LLM_SOFT_TIMEOUT_SEC,
        IMPL_DRAFT_LLM_STALL_TIMEOUT_SEC,
        IMPL_DRAFT_LLM_HARD_TIMEOUT_SEC,
    )
}

fn impl_trace_label_draft_name(timeout_label: &str) -> Option<String> {
    timeout_label
        .strip_prefix("impl_orc_code [")
        .and_then(|rest| rest.strip_suffix(']'))
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

pub(crate) fn write_impl_draft_progress(
    root: &Path,
    draft_name: &str,
    status: &str,
    elapsed_sec: u64,
    detail: &str,
) -> Result<(), String> {
    let (soft_timeout_sec, stall_timeout_sec, hard_timeout_sec) = impl_timeout_watch();
    let snapshot = ImplDraftProgressSnapshot {
        draft_name: draft_name.to_string(),
        status: status.to_string(),
        elapsed_sec,
        detail: detail.trim().to_string(),
        updated_at: now_unix_sec(),
        soft_timeout_sec,
        stall_timeout_sec,
        hard_timeout_sec,
    };
    save_impl_progress_snapshot(root, &snapshot)
}

pub(crate) fn update_impl_draft_progress_from_watch(
    root: &Path,
    timeout_label: &str,
    status: &str,
    elapsed_sec: u64,
    detail: &str,
) {
    let Some(draft_name) = impl_trace_label_draft_name(timeout_label) else {
        return;
    };
    let _ = write_impl_draft_progress(root, &draft_name, status, elapsed_sec, detail);
}

fn llm_impl_output_indicates_failure(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("read-only")
        || lower.contains("read only")
        || lower.contains("실패 사유만 보고")
        || lower.contains("구현을 진행할 수 있는 상태가 아닙니다")
        || lower.contains("변경하지 않았습니다")
        || lower.contains("cannot proceed")
        || lower.contains("unable to proceed")
        || lower.contains("failed to")
}

struct ImplRunResult {
    succeeded: Vec<String>,
    failed: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::{
        add_orc_drafts, auto_feature_names_from_message, build_create_job_md_prompt,
        build_draft_item_from_requirement, build_initial_project_md, check_orc_code,
        cleanup_drafts_yaml_after_success, create_input_md,
        ensure_add_orc_drafts_produced_targets, flow_rust_orchestra, get_workspace_state,
        infer_spec_from_message, job_task_state_change, llm_impl_output_indicates_failure,
        load_architecture_contract_from_root, load_drafts_doc, load_impl_progress_snapshots,
        merge_requirement_rule, move_job_task_item, normalize_job_md_content, parse_common_opts,
        parse_outline_requirements_to_job_doc, parse_project_md_meta, read_bootstrap_seed,
        set_draft_item_state, transition_impl_result, transition_impl_start,
        update_impl_draft_progress_from_watch, BootstrapSeed, CodeDraftsDoc, CommonOpts,
        DraftItemDoc, JobChecklistSection, JobDoc, JobRequirement,
    };
    use std::env;
    use std::fs;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    fn with_locked_workspace<T>(test_name: &str, run: impl FnOnce() -> T) -> T {
        static WORKSPACE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = WORKSPACE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = Path::new("/tmp").join(format!("mono_manager_{}", test_name));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test root");

        let prev_dir = env::current_dir().expect("get current dir");
        env::set_current_dir(&root).expect("set test dir");
        let result = catch_unwind(AssertUnwindSafe(run));
        env::set_current_dir(prev_dir).expect("restore current dir");

        let _ = fs::remove_dir_all(&root);
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

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
        job_task_state_change(&mut job, "todo_create", "verify").expect("verify alias");
        assert_eq!(job.task.check, vec!["todo_create".to_string()]);
    }

    #[test]
    fn transition_impl_start_uses_copy_on_write() {
        let drafts = CodeDraftsDoc {
            draft: vec![
                DraftItemDoc {
                    name: "cli_impl_orc_code".to_string(),
                    state: "planned".to_string(),
                    ..Default::default()
                },
                DraftItemDoc {
                    name: "cli_help".to_string(),
                    state: "planned".to_string(),
                    ..Default::default()
                },
            ],
        };
        let mut job = JobDoc::default();
        job.task.planned.push("cli_impl_orc_code".to_string());
        job.task.planned.push("cli_help".to_string());
        let targets = vec![DraftItemDoc {
            name: "cli_impl_orc_code".to_string(),
            ..Default::default()
        }];

        let (next_drafts, next_job) =
            transition_impl_start(&drafts, &job, &targets).expect("start transition");

        assert_eq!(drafts.draft[0].state, "planned");
        assert_eq!(job.task.work.len(), 0);
        assert_eq!(next_drafts.draft[0].state, "work");
        assert_eq!(next_drafts.draft[1].state, "planned");
        assert_eq!(next_job.task.work, vec!["cli_impl_orc_code".to_string()]);
        assert_eq!(next_job.task.planned, vec!["cli_help".to_string()]);
    }

    #[test]
    fn transition_impl_result_moves_success_and_fail() {
        let drafts = CodeDraftsDoc {
            draft: vec![
                DraftItemDoc {
                    name: "cli_impl_orc_code".to_string(),
                    state: "work".to_string(),
                    ..Default::default()
                },
                DraftItemDoc {
                    name: "cli_help".to_string(),
                    state: "work".to_string(),
                    ..Default::default()
                },
            ],
        };
        let mut job = JobDoc::default();
        job.task.work.push("cli_impl_orc_code".to_string());
        job.task.work.push("cli_help".to_string());

        let (next_drafts, next_job) = transition_impl_result(
            &drafts,
            &job,
            &["cli_impl_orc_code".to_string()],
            &[("cli_help".to_string(), "failed".to_string())],
        )
        .expect("finish transition");

        assert_eq!(next_drafts.draft[0].state, "complete");
        assert_eq!(next_drafts.draft[1].state, "error");
        assert_eq!(next_job.task.check, vec!["cli_impl_orc_code".to_string()]);
        assert_eq!(next_job.task.fail, vec!["cli_help".to_string()]);
        assert_eq!(next_job.problems, vec!["- cli_help : failed".to_string()]);
    }

    #[test]
    fn check_orc_code_builds_checklist_and_completes_verify_without_problems() {
        with_locked_workspace("check_code_verify_complete", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n1. create task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: todo_app\n    state: complete\n    constraints:\n      - \"todo_app -> visible_result : add task renders in UI\"\n    check:\n      - \"todo_app -> persisted : reload keeps todo item\"\n",
            )
            .expect("write drafts");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n1. create task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n\n# check evidence\n- [x] todo_app -> verified : unit test passed | data_source=real | execution=unit | artifact=none\n- [x] todo_app -> visible_result : browser verified | data_source=real | execution=browser | artifact=.project/screenshot/todo_app.png\n- [x] todo_app -> persisted : reload keeps todo item | data_source=real-equivalent | execution=browser | artifact=.project/screenshot/todo_app.png\n",
            )
            .expect("rewrite job with check evidence");

            let result = check_orc_code().expect("run check");
            let job = super::load_job_doc().expect("load job");

            assert!(result.contains("checklist=4"));
            assert!(result.contains("execution=3"));
            assert!(result.contains("hard_gate_failed=0"));
            assert!(result.contains("skill=/home/tree/ai/skills/check-code/SKILL.md"));
            assert!(job.task.check.is_empty());
            assert_eq!(job.task.completed, vec!["todo_app".to_string()]);
            assert_eq!(
                job.checklist,
                vec![
                    "todo_app -> verified : create task".to_string(),
                    "todo_app -> verified : add task".to_string(),
                    "todo_app -> visible_result : add task renders in UI".to_string(),
                    "todo_app -> persisted : reload keeps todo item".to_string(),
                ]
            );
            assert_eq!(
                job.check_sections,
                vec![
                    JobChecklistSection {
                        name: "logic_checklist".to_string(),
                        items: vec![
                            "todo_app -> verified : create task".to_string(),
                            "todo_app -> verified : add task".to_string(),
                        ],
                    },
                    JobChecklistSection {
                        name: "ui_checklist".to_string(),
                        items: vec!["todo_app -> visible_result : add task renders in UI".to_string()],
                    },
                    JobChecklistSection {
                        name: "reentry_checklist".to_string(),
                        items: vec!["todo_app -> persisted : reload keeps todo item".to_string()],
                    },
                ]
            );
        });
    }

    #[test]
    fn parse_job_md_preserves_check_sections() {
        let job = super::parse_job_md(
            "# requirement\n## todo_app\n\n# task\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n\n# check\n## logic_checklist\n- todo_app -> verified : handler updates state\n## ui_checklist\n- todo_app -> rendered : input is visible\n\n# check evidence\n- [x] todo_app -> rendered : browser verified\n",
        )
        .expect("parse job");

        assert_eq!(
            job.check_sections,
            vec![
                JobChecklistSection {
                    name: "logic_checklist".to_string(),
                    items: vec!["todo_app -> verified : handler updates state".to_string()],
                },
                JobChecklistSection {
                    name: "ui_checklist".to_string(),
                    items: vec!["todo_app -> rendered : input is visible".to_string()],
                },
            ]
        );
        assert_eq!(
            job.checklist,
            vec![
                "todo_app -> verified : handler updates state".to_string(),
                "todo_app -> rendered : input is visible".to_string(),
            ]
        );
    }

    #[test]
    fn render_job_md_keeps_check_sections_and_plain_items() {
        let job = JobDoc {
            check_sections: vec![
                JobChecklistSection {
                    name: "logic_checklist".to_string(),
                    items: vec!["todo_app -> verified : handler updates state".to_string()],
                },
                JobChecklistSection {
                    name: "ui_checklist".to_string(),
                    items: vec!["todo_app -> rendered : input is visible".to_string()],
                },
            ],
            checklist: vec![
                "todo_app -> verified : handler updates state".to_string(),
                "todo_app -> rendered : input is visible".to_string(),
                "todo_app -> persisted : reload keeps item".to_string(),
            ],
            ..Default::default()
        };

        let rendered = super::render_job_md(&job);

        assert!(rendered.contains("# check\n## logic_checklist\n- todo_app -> verified : handler updates state\n## ui_checklist\n- todo_app -> rendered : input is visible\n## checklist\n- todo_app -> persisted : reload keeps item\n"));
    }

    #[test]
    fn check_orc_code_requires_execution_checklist_evidence() {
        with_locked_workspace("check_code_requires_execution_evidence", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: todo_app\n    state: complete\n    check:\n      - \"todo_app -> persisted : reload keeps todo item\"\n",
            )
            .expect("write drafts");

            let err = check_orc_code().expect_err("missing checklist should fail");

            assert!(err.contains("missing job.md check evidence entries"));
        });
    }

    #[test]
    fn check_orc_code_keeps_verify_when_problems_exist() {
        with_locked_workspace("check_code_verify_problem_loop", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n- todo_app : save fails on reload\n",
            )
            .expect("write job");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: todo_app\n    state: complete\n    check:\n      - \"todo_app -> persisted : reload keeps todo item\"\n",
            )
            .expect("write drafts");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n- todo_app : save fails on reload\n\n# check evidence\n- [x] todo_app -> verified : unit test passed | data_source=real | execution=unit | artifact=none\n",
            )
            .expect("rewrite job with check evidence");

            check_orc_code().expect("run check");
            let job = super::load_job_doc().expect("load job");

            assert_eq!(job.task.check, vec!["todo_app".to_string()]);
            assert!(job.task.completed.is_empty());
            assert!(job
                .checklist
                .contains(&"todo_app -> resolved : todo_app : save fails on reload".to_string()));
        });
    }

    #[test]
    fn check_orc_code_keeps_verify_when_execution_checklist_has_unresolved_items() {
        with_locked_workspace("check_code_unresolved_execution_checklist", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: todo_app\n    state: complete\n    check:\n      - \"todo_app -> persisted : reload keeps todo item\"\n",
            )
            .expect("write drafts");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n\n# check evidence\n- [ ] todo_app -> persisted : reload keeps todo item | data_source=real-equivalent | execution=browser | artifact=.project/screenshot/todo.png\n",
            )
            .expect("rewrite job with check evidence");

            let result = check_orc_code().expect("run check");
            let job = super::load_job_doc().expect("load job");

            assert!(result.contains("execution=0"));
            assert_eq!(job.task.check, vec!["todo_app".to_string()]);
            assert!(job.task.completed.is_empty());
            assert!(job
                .problems
                .iter()
                .any(|item| item.contains("job.md check evidence unresolved")));
        });
    }

    #[test]
    fn check_orc_code_blocks_ui_complete_without_real_browser_artifact() {
        with_locked_workspace("check_code_blocks_fixture_ui_completion", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## todo_app\n- add task\n\n# task\n## planned\n## work\n## verify\n- todo_app\n## complete\n## fail\n\n# problems\n\n# check\n## ui_checklist\n- todo_app -> visible_result : add task renders in UI\n\n# check evidence\n- [x] todo_app -> visible_result : browser verified | data_source=fixture | execution=browser | artifact=.project/screenshot/ui.png\n",
            )
            .expect("write job");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: todo_app\n    state: complete\n    constraints:\n      - \"todo_app -> visible_result : add task renders in UI\"\n",
            )
            .expect("write drafts");

            let result = check_orc_code().expect("run check");
            let job = super::load_job_doc().expect("load job");

            assert!(result.contains("hard_gate_failed=1"));
            assert!(result.contains("mode=fixture_only"));
            assert_eq!(job.task.check, vec!["todo_app".to_string()]);
            assert!(job.task.completed.is_empty());
            assert!(job
                .problems
                .iter()
                .any(|item| item.contains("ui_checklist requires browser evidence")));
        });
    }

    #[test]
    fn should_cleanup_drafts_yaml_rejects_unrelated_completed_drafts() {
        let mut job = JobDoc::default();
        job.task.completed = vec!["todo_app".to_string()];
        let drafts = CodeDraftsDoc {
            draft: vec![
                DraftItemDoc {
                    name: "todo_app".to_string(),
                    state: "complete".to_string(),
                    ..Default::default()
                },
                DraftItemDoc {
                    name: "other_feature".to_string(),
                    state: "complete".to_string(),
                    ..Default::default()
                },
            ],
        };

        assert!(!super::should_cleanup_drafts_yaml(&job, &drafts));
    }

    #[test]
    fn build_draft_item_from_requirement_normalizes_constraint_name() {
        let req = JobRequirement {
            name: "Rust CLI Workspace".to_string(),
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.name, "rust_cli_workspace");
        assert!(item.scope.is_empty());
        assert_eq!(item.tasks, vec!["implement rust_cli_workspace".to_string()]);
        assert_eq!(
            item.constraints,
            vec![
                "rust_cli_workspace -> rust_cli_workspace : requirement 기반 draft item 생성"
                    .to_string()
            ]
        );
        assert_eq!(item.check, vec!["verify rust_cli_workspace".to_string()]);
    }

    #[test]
    fn build_draft_item_from_requirement_keeps_cli_create_job_md_contract() {
        let req = JobRequirement {
            name: "cli_create_job_md".to_string(),
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.name, "cli_create_job_md");
        assert!(item.scope.is_empty());
        assert_eq!(item.tasks, vec!["implement cli_create_job_md".to_string()]);
        assert_eq!(
            item.constraints,
            vec![
                "cli_create_job_md -> cli_create_job_md : requirement 기반 draft item 생성"
                    .to_string()
            ]
        );
        assert_eq!(item.check, vec!["verify cli_create_job_md".to_string()]);
    }

    #[test]
    fn build_draft_item_from_requirement_normalizes_project_documentation_from_camel_case() {
        let req = JobRequirement {
            name: "ProjectDocumentation".to_string(),
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.name, "project_documentation");
        assert!(item.scope.is_empty());
        assert_eq!(
            item.constraints,
            vec![
                "project_documentation -> project_documentation : requirement 기반 draft item 생성"
                    .to_string()
            ]
        );
    }

    #[test]
    fn build_draft_item_from_requirement_normalizes_rust_cli_workspace_from_acronym_camel_case() {
        let req = JobRequirement {
            name: "RustCLIWorkspace".to_string(),
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.name, "rust_cli_workspace");
        assert!(item.scope.is_empty());
        assert_eq!(item.tasks, vec!["implement rust_cli_workspace".to_string()]);
        assert_eq!(
            item.constraints,
            vec![
                "rust_cli_workspace -> rust_cli_workspace : requirement 기반 draft item 생성"
                    .to_string()
            ]
        );
        assert_eq!(item.check, vec!["verify rust_cli_workspace".to_string()]);
    }

    #[test]
    fn build_draft_item_from_requirement_uses_default_step_when_input_steps_are_blank() {
        let req = JobRequirement {
            name: "rust_cli_workspace".to_string(),
            steps: vec!["   ".to_string(), "".to_string()],
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.step, vec!["trigger -> process -> result".to_string()]);
    }

    #[test]
    fn build_draft_item_from_requirement_filters_blank_rules() {
        let req = JobRequirement {
            name: "rust_cli_workspace".to_string(),
            rules: vec![
                "".to_string(),
                "   ".to_string(),
                "must pass unit tests".to_string(),
            ],
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.rule, vec!["must pass unit tests".to_string()]);
    }

    #[test]
    fn build_draft_item_from_requirement_keeps_korean_requirement_name() {
        let req = JobRequirement {
            name: "도메인 패널 정렬 수정".to_string(),
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.name, "도메인_패널_정렬_수정");
        assert_eq!(
            item.tasks,
            vec!["implement 도메인_패널_정렬_수정".to_string()]
        );
        assert_eq!(
            item.constraints,
            vec![
                "도메인_패널_정렬_수정 -> 도메인_패널_정렬_수정 : requirement 기반 draft item 생성"
                    .to_string()
            ]
        );
    }

    #[test]
    fn build_draft_item_from_requirement_filters_blank_rules_for_rust_cli_workspace() {
        let req = JobRequirement {
            name: "rust_cli_workspace".to_string(),
            rules: vec!["".to_string(), "   ".to_string(), "must_keep".to_string()],
            ..Default::default()
        };

        let item = build_draft_item_from_requirement(&req);

        assert_eq!(item.rule, vec!["must_keep".to_string()]);
    }

    #[test]
    fn add_orc_drafts_skips_requirement_with_empty_feature_key() {
        with_locked_workspace("empty_feature_key", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# plan\n\n# requirement\n## !!!\n\n# task\n## planned\n## work\n## check\n## completed\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(".project/drafts.yaml", "draft: []\n").expect("write drafts");

            let err = add_orc_drafts().expect_err("run add_orc_drafts");

            assert!(err.contains("produced 0 draft items"));
        });
    }

    #[test]
    fn ensure_add_orc_drafts_produced_targets_rejects_empty_requirement_state() {
        let err = ensure_add_orc_drafts_produced_targets(&JobDoc::default(), &CodeDraftsDoc::default())
            .expect_err("empty requirements must fail");

        assert!(err.contains("requirement section is empty"));
    }

    #[test]
    fn add_orc_drafts_keeps_existing_cli_impl_orc_code_and_backfills_planned_task() {
        with_locked_workspace("cli_impl_orc_backfill_planned", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## cli_impl_orc_code\n\n# task\n## planned\n## work\n## check\n## completed\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: cli_impl_orc_code\n    state: planned\n    type: action\n    domain:\n      - core\n    depends_on: []\n    scope:\n      - feature:cli_impl_orc_code\n    rule: []\n    step:\n      - trigger -> process -> result\n    tasks:\n      - implement cli_impl_orc_code\n    constraints:\n      - \"cli_impl_orc_code -> cli_impl_orc_code : requirement 기반 draft item 생성\"\n    check:\n      - verify cli_impl_orc_code\n",
            )
            .expect("write drafts");

            add_orc_drafts().expect("run add_orc_drafts");
            let job = super::load_job_doc().expect("load job");
            assert_eq!(job.task.planned, vec!["cli_impl_orc_code".to_string()]);
        });
    }

    #[test]
    fn add_orc_drafts_creates_draft_item_from_korean_requirement() {
        with_locked_workspace("korean_requirement_draft_item", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## 도메인 패널 정렬 수정\n- 헤더와 본문 패널의 좌우 기준선을 맞춘다\n> current.png 기준으로 정렬을 고친다\n\n# task\n## planned\n## work\n## check\n## completed\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(".project/drafts.yaml", "draft: []\n").expect("write drafts");

            add_orc_drafts().expect("run add_orc_drafts");
            let drafts = load_drafts_doc().expect("load drafts");
            let item = drafts
                .draft
                .iter()
                .find(|draft| draft.name == "도메인_패널_정렬_수정")
                .expect("korean draft item");

            assert_eq!(
                item.rule,
                vec!["헤더와 본문 패널의 좌우 기준선을 맞춘다".to_string()]
            );
            assert_eq!(
                item.step,
                vec!["current.png 기준으로 정렬을 고친다".to_string()]
            );
            assert_eq!(
                item.tasks,
                vec!["implement 도메인_패널_정렬_수정".to_string()]
            );
        });
    }

    #[test]
    fn add_orc_drafts_normalizes_job_md_requirement_heading_before_build() {
        with_locked_workspace("normalize_job_requirement_heading", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# plan\n\n# requirements\n## todo_app\n- add task\n\n# task\n## planned\n## work\n## verify\n## complete\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(".project/drafts.yaml", "draft: []\n").expect("write drafts");

            add_orc_drafts().expect("run add_orc_drafts");

            let normalized = fs::read_to_string("job.md").expect("read normalized job");
            assert!(normalized.contains("# requirement"));
            assert!(!normalized.contains("# requirements"));
            let drafts = load_drafts_doc().expect("load drafts");
            assert!(drafts.draft.iter().any(|item| item.name == "todo_app"));
        });
    }

    #[test]
    fn add_orc_drafts_backs_up_stale_legacy_drafts_before_rebuild() {
        with_locked_workspace("backup_stale_legacy_drafts", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## current_png_complaint\n- align layout\n\n# task\n## planned\n## work\n## verify\n## complete\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(
                ".project/drafts.yaml",
                "planned:\n  - legacy_task\nfailed:\n  - old_item\n",
            )
            .expect("write legacy drafts");

            add_orc_drafts().expect("run add_orc_drafts");

            let drafts = load_drafts_doc().expect("load drafts");
            assert!(drafts
                .draft
                .iter()
                .any(|item| item.name == "current_png_complaint"));
            let backup_dir = Path::new(".project").join("runtime").join("backups");
            let backups = fs::read_dir(&backup_dir)
                .expect("backup dir")
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>();
            assert!(backups.iter().any(|name| name.contains("legacy-drafts-drafts.yaml")));
        });
    }

    #[test]
    fn update_impl_draft_progress_from_watch_writes_runtime_snapshot_for_korean_name() {
        with_locked_workspace("impl_progress_korean", || {
            fs::create_dir_all(".project").expect("create .project");

            update_impl_draft_progress_from_watch(
                Path::new("."),
                "impl_orc_code [도메인_패널_정렬_수정]",
                "slow_progress",
                360,
                "implementation still progressing after soft timeout",
            );

            let snapshots = load_impl_progress_snapshots(Path::new(".")).expect("load snapshots");
            assert_eq!(snapshots.len(), 1);
            assert_eq!(snapshots[0].draft_name, "도메인_패널_정렬_수정");
            assert_eq!(snapshots[0].status, "slow_progress");
            assert_eq!(snapshots[0].elapsed_sec, 360);
        });
    }

    #[test]
    fn parse_common_opts_reads_path_and_message() {
        let opts = parse_common_opts(&[
            "-p".to_string(),
            "/tmp/demo".to_string(),
            "-m".to_string(),
            "react vite hello".to_string(),
            "-a".to_string(),
        ]);

        assert_eq!(opts.path.as_deref(), Some("/tmp/demo"));
        assert_eq!(opts.message.as_deref(), Some("react vite hello"));
        assert!(opts.auto);
    }

    #[test]
    fn build_initial_project_md_keeps_todo_feature_from_message() {
        let opts = CommonOpts {
            path: Some("/tmp/react_todo".to_string()),
            message: Some("Build a React todo app with add toggle delete".to_string()),
            ..Default::default()
        };

        let body = build_initial_project_md(&opts, Path::new("/tmp/react_todo"))
            .expect("build project md");

        assert!(body.contains("- todo_app"));
    }

    #[test]
    fn llm_impl_output_indicates_failure_for_read_only_report() {
        assert!(llm_impl_output_indicates_failure(
            "실패 사유만 보고합니다.\n작업 환경이 read-only라서 파일 생성이 불가능합니다."
        ));
        assert!(!llm_impl_output_indicates_failure(
            "implemented todo_app and wrote package.json"
        ));
    }

    #[test]
    fn auto_feature_names_from_message_drops_bootstrap_when_todo_exists() {
        assert_eq!(
            auto_feature_names_from_message("Build a react todo app"),
            vec!["todo_app".to_string()]
        );
    }

    #[test]
    fn merge_requirement_rule_appends_message_without_duplicates() {
        let mut job = JobDoc {
            requirement: vec![JobRequirement {
                name: "todo_app".to_string(),
                steps: Vec::new(),
                rules: vec!["Build a React todo app".to_string()],
            }],
            ..Default::default()
        };

        merge_requirement_rule(&mut job, "todo_app", "Build a React todo app");
        merge_requirement_rule(&mut job, "todo_app", "Add delete support");

        assert_eq!(job.requirement.len(), 1);
        assert_eq!(
            job.requirement[0].rules,
            vec![
                "Build a React todo app".to_string(),
                "Add delete support".to_string()
            ]
        );
    }

    #[test]
    fn infer_spec_from_message_extracts_known_frameworks() {
        assert_eq!(
            infer_spec_from_message("Build a React + Vite app with Zustand"),
            Some("react, vite, zustand".to_string())
        );
    }

    #[test]
    fn normalize_job_md_content_wraps_outline_into_job_doc() {
        let normalized = normalize_job_md_content(
            "# todo_app\n- add task\n- delete task\n> create task\n> remove task",
        )
        .expect("normalize outline");

        assert!(normalized.contains("# requirement"));
        assert!(normalized.contains("## todo_app"));
        assert!(normalized.contains("1. create task"));
        assert!(normalized.contains("- add task"));
    }

    #[test]
    fn parse_outline_requirements_to_job_doc_reads_rules_and_steps() {
        let doc = parse_outline_requirements_to_job_doc(
            "# todo_app\n- add task\n> create task\n## filter_panel\n- filter tasks\n> click filter",
        )
        .expect("outline doc");

        assert_eq!(doc.requirement.len(), 2);
        assert_eq!(doc.requirement[0].name, "todo_app");
        assert_eq!(doc.requirement[0].rules, vec!["add task".to_string()]);
        assert_eq!(doc.requirement[0].steps, vec!["create task".to_string()]);
        assert_eq!(doc.requirement[1].name, "filter_panel");
    }

    #[test]
    fn build_initial_project_md_uses_message_and_project_root() {
        let opts = CommonOpts {
            path: Some("/tmp/orc_skill_project".to_string()),
            message: Some("Build a React + Vite app with Zustand".to_string()),
            ..Default::default()
        };

        let body = build_initial_project_md(&opts, Path::new("/tmp/orc_skill_project"))
            .expect("build project md");

        assert!(body.contains("# info"));
        assert!(body.contains("description: Build a React + Vite app with Zustand"));
        assert!(body.contains("spec: react, vite, zustand"));
        assert!(body.contains("path: /tmp/orc_skill_project"));
        assert!(body.contains("- bootstrap_runtime"));
        assert!(body.contains("- zustand_store_setup"));
    }

    #[test]
    fn read_bootstrap_seed_prefers_saved_project_md_values() {
        with_locked_workspace("bootstrap_seed_reads_project_md", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                ".project/project.md",
                "# info\nname: flow_google\ndescription: demo\nspec: react, vite\npath: /tmp/flow_google\n",
            )
            .expect("write project md");

            let seed = read_bootstrap_seed(Path::new(".")).expect("read bootstrap seed");

            assert_eq!(
                seed,
                BootstrapSeed {
                    name: "flow_google".to_string(),
                    spec: "react, vite".to_string(),
                    root: "/tmp/flow_google".to_string(),
                }
            );
        });
    }

    #[test]
    fn create_input_md_generates_cli_create_input_md_draft_from_requirement() {
        with_locked_workspace("create_input_md_generates_draft", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# requirement\n## cli_create_input_md\n\n# task\n## planned\n## work\n## check\n## completed\n## fail\n\n# problems\n",
            )
            .expect("write job");
            fs::write(".project/drafts.yaml", "draft: []\n").expect("write drafts");

            create_input_md().expect("run create_input_md");

            let drafts = load_drafts_doc().expect("load drafts");
            assert_eq!(drafts.draft.len(), 1);
            assert_eq!(drafts.draft[0].name, "cli_create_input_md");
            assert_eq!(
                drafts.draft[0].constraints,
                vec![
                    "cli_create_input_md -> cli_create_input_md : requirement 기반 draft item 생성"
                        .to_string()
                ]
            );
        });
    }

    #[test]
    fn init_orc_job_creates_requirement_from_project_features() {
        with_locked_workspace("init_orc_job_creates_requirement", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                ".project/project.md",
                "# features\n- todo_app\n- filter_panel\n",
            )
            .expect("write project");

            let result = super::init_orc_job().expect("init job");
            let job = super::load_job_doc().expect("load job");

            assert_eq!(result, "init_orc_job completed");
            assert_eq!(job.requirement.len(), 2);
            assert_eq!(job.requirement[0].name, "todo_app");
            assert_eq!(job.requirement[1].name, "filter_panel");
        });
    }

    #[test]
    fn init_orc_job_preserves_existing_requirement_when_job_exists() {
        with_locked_workspace("init_orc_job_preserves_existing_requirement", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(".project/project.md", "# features\n- overwritten_feature\n")
                .expect("write project");
            fs::write(
                "job.md",
                "# requirement\n## keep_existing_requirement\n- preserve this\n\n# task\n## planned\n## work\n## verify\n## complete\n## fail\n\n# problems\n",
            )
            .expect("write job");

            let result = super::init_orc_job().expect("init job");
            let job = super::load_job_doc().expect("load job");

            assert_eq!(result, "job.md already has requirement");
            assert_eq!(job.requirement.len(), 1);
            assert_eq!(job.requirement[0].name, "keep_existing_requirement");
            assert_eq!(job.requirement[0].rules, vec!["preserve this".to_string()]);
        });
    }

    #[test]
    fn init_orc_job_repairs_empty_requirement_from_project_features() {
        with_locked_workspace("init_orc_job_repairs_empty_requirement", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(".project/project.md", "# features\n- repaired_feature\n")
                .expect("write project");
            fs::write(
                "job.md",
                "# requirement\n\n# task\n## planned\n## work\n## verify\n## complete\n## fail\n\n# problems\n",
            )
            .expect("write job");

            let result = super::init_orc_job().expect("init job");
            let job = super::load_job_doc().expect("load job");

            assert_eq!(result, "init_orc_job repaired requirement");
            assert_eq!(job.requirement.len(), 1);
            assert_eq!(job.requirement[0].name, "repaired_feature");
        });
    }

    #[test]
    fn init_orc_job_rejects_empty_project_features_instead_of_writing_empty_requirement() {
        with_locked_workspace("init_orc_job_rejects_empty_features", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(".project/project.md", "# info\nname: demo\n").expect("write project");

            let err = super::init_orc_job().expect_err("empty features must fail");

            assert!(err.contains("could not derive requirement"));
            assert!(!Path::new("job.md").exists());
        });
    }

    #[test]
    fn get_workspace_state_returns_ready_when_required_files_exist() {
        let root = Path::new("/tmp/cli_rust_orchestra_ready_state");
        let _ = fs::remove_dir_all(root);
        fs::create_dir_all(root.join(".project")).expect("create .project");
        fs::write(root.join(".project").join("project.md"), "# info\n").expect("write project.md");
        fs::write(root.join("job.md"), "# task\n").expect("write job.md");
        fs::write(root.join(".project").join("drafts.yaml"), "draft: []\n")
            .expect("write drafts.yaml");

        assert_eq!(get_workspace_state(root), "ready");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn flow_rust_orchestra_blocks_when_workspace_not_ready() {
        let root = Path::new("/tmp/cli_rust_orchestra_not_ready");
        let _ = fs::remove_dir_all(root);
        fs::create_dir_all(root).expect("create root");

        let err = flow_rust_orchestra(root, &[]).expect_err("workspace must not be ready");
        assert!(err.contains("workspace state is uninitialized"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn flow_rust_orchestra_rejects_arguments_with_echoed_values() {
        let root = Path::new("/tmp/cli_rust_orchestra_reject_args");
        let _ = fs::remove_dir_all(root);
        fs::create_dir_all(root).expect("create root");

        let err = flow_rust_orchestra(root, &[String::from("--dry-run"), String::from("1")])
            .expect_err("args must be rejected");
        assert_eq!(
            err,
            "cli_rust_orchestra does not accept arguments: --dry-run 1"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn flow_rust_orchestra_fails_when_cli_rust_orchestra_requirement_is_missing() {
        let root = Path::new("/tmp/cli_rust_orchestra_missing_requirement");
        let _ = fs::remove_dir_all(root);
        fs::create_dir_all(root.join(".project")).expect("create .project");
        fs::write(root.join(".project").join("project.md"), "# info\n").expect("write project.md");
        fs::write(
            root.join("job.md"),
            "# requirement\n## cli_help\n\n# task\n## planned\n## work\n## check\n## completed\n## fail\n\n# problems\n",
        )
        .expect("write job.md");
        fs::write(root.join(".project").join("drafts.yaml"), "draft: []\n")
            .expect("write drafts.yaml");

        let err = flow_rust_orchestra(root, &[]).expect_err("missing requirement must fail");
        assert_eq!(
            err,
            "required draft item not found after add_orc_drafts: cli_rust_orchestra"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn flow_rust_orchestra_returns_trigger_process_result_message() {
        let root = Path::new("/tmp/cli_rust_orchestra_ready_result");
        let _ = fs::remove_dir_all(root);
        fs::create_dir_all(root.join(".project")).expect("create .project");
        fs::write(root.join(".project").join("project.md"), "# info\n").expect("write project.md");
        fs::write(
            root.join("job.md"),
            "# requirement\n## cli_rust_orchestra\n\n# task\n## planned\n## work\n## check\n## completed\n## fail\n\n# problems\n",
        )
        .expect("write job.md");
        fs::write(root.join(".project").join("drafts.yaml"), "draft: []\n")
            .expect("write drafts.yaml");

        let output = flow_rust_orchestra(root, &[]).expect("workspace ready");
        assert_eq!(
            output,
            "trigger: ready -> process: validate_workspace+add_orc_drafts -> result: cli_rust_orchestra completed"
        );
        let drafts_raw = fs::read_to_string(root.join(".project").join("drafts.yaml"))
            .expect("read drafts.yaml");
        assert!(drafts_raw.contains("name: cli_rust_orchestra"));
        assert!(drafts_raw.contains(
            "cli_rust_orchestra -> cli_rust_orchestra : requirement 기반 draft item 생성"
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_drafts_yaml_after_success_removes_file_when_pipeline_is_complete() {
        with_locked_workspace("cleanup_drafts_on_success", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# task\n## planned\n## work\n## check\n## completed\n- feature_a\n## fail\n\n# problems\n",
            )
            .expect("write job.md");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: feature_a\n    state: complete\n",
            )
            .expect("write drafts");

            let removed =
                cleanup_drafts_yaml_after_success(Path::new(".")).expect("cleanup success");
            assert!(removed);
            assert!(!Path::new(".project/drafts.yaml").exists());
        });
    }

    #[test]
    fn cleanup_drafts_yaml_after_success_keeps_file_when_fail_exists() {
        with_locked_workspace("cleanup_drafts_on_fail", || {
            fs::create_dir_all(".project").expect("create .project");
            fs::write(
                "job.md",
                "# task\n## planned\n## work\n## check\n## completed\n- feature_a\n## fail\n- feature_b\n\n# problems\n- feature_b: failed\n",
            )
            .expect("write job.md");
            fs::write(
                ".project/drafts.yaml",
                "draft:\n  - name: feature_a\n    state: complete\n",
            )
            .expect("write drafts");

            let removed =
                cleanup_drafts_yaml_after_success(Path::new(".")).expect("cleanup decision");
            assert!(!removed);
            assert!(Path::new(".project/drafts.yaml").exists());
        });
    }

    #[test]
    fn build_create_job_md_prompt_contains_required_sections() {
        let prompt = build_create_job_md_prompt(
            "template",
            "# info\nname: demo\n",
            None,
            "job.md",
            "drafts:\n  planned:\n    - cli_create_job_md\n",
        );
        assert!(prompt.contains("template"));
        assert!(prompt.contains("# project.md"));
        assert!(prompt.contains("# job.md"));
    }

    #[test]
    fn parse_project_md_meta_reads_architecture_name() {
        let meta = parse_project_md_meta(
            "# info\nname: demo\n\n# architecture\nname: architecture-layered\n",
        );
        assert_eq!(meta.architecture, "architecture-layered");
    }

    #[test]
    fn load_architecture_contract_from_root_reads_sample_skill() {
        with_locked_workspace("load_architecture_contract", || {
            fs::create_dir_all(".project").expect("create project dir");
            fs::write(
                ".project/project.md",
                "# info\nname: demo\n\n# architecture\nname: architecture-layered\n",
            )
            .expect("write project.md");

            let contract = load_architecture_contract_from_root(Path::new("."))
                .expect("load contract")
                .expect("contract present");
            assert_eq!(contract.skill_id, "architecture-layered");
            assert!(contract
                .constraints
                .iter()
                .any(|item| item.contains("src/domain/** -> src/infrastructure/**")));
            assert!(contract
                .checks
                .iter()
                .any(|item| item.contains("presentation -> repository implementation")));
        });
    }

    #[test]
    fn normalize_job_md_content_extracts_markdown_block() {
        let raw = "```markdown\n# feature\n- rule\n> step\n```";
        let normalized = normalize_job_md_content(raw).expect("normalize");
        assert!(normalized.contains("# requirement"));
        assert!(normalized.contains("## feature"));
        assert!(normalized.contains("1. step"));
        assert!(normalized.contains("- rule"));
    }
}
