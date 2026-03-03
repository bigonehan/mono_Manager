mod config;
mod cli;
mod chat;
mod draft;
mod parallel;
mod plan;
mod project;
mod tmux;
mod ui;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub(crate) use draft::{DraftDoc, DraftsListDoc, DraftTask, PlannedItem};

const REGISTRY_PATH: &str = "configs/project.yaml";
const LEGACY_REGISTRY_PATH: &str = "configs/Project.yaml";
const EXEC_LOG_PATH: &str = ".project/log.md";
const PROJECT_MD_PATH: &str = ".project/project.md";
const PRIMARY_DRAFTS_LIST_FILE: &str = "drafts_list.yaml";
const INPUT_MD_PATH: &str = "input.md";
const TODOS_YAML_PATH: &str = ".project/todos.yaml";
const FEATURE_NAME_SKILL_PATH: &str = "/home/tree/ai/skills/rule-naming/SKILL.md";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectRecord {
    #[serde(default)]
    id: String,
    name: String,
    path: String,
    description: String,
    created_at: String,
    updated_at: String,
    selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectRegistry {
    #[serde(default, rename = "recentActivepane")]
    recent_active_pane: Option<String>,
    #[serde(default)]
    projects: Vec<ProjectRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParallelFeatureTask {
    pub(crate) name: String,
    pub(crate) draft_path: PathBuf,
    pub(crate) depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TodoDoc {
    #[serde(default)]
    tasks: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TodoItem {
    name: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    step: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    draft_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct GeneratedTodoItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    step: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    draft_path: String,
}

fn calc_now_unix() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

fn calc_primary_drafts_list_path(project_root: &Path) -> PathBuf {
    project_root.join(".project").join(PRIMARY_DRAFTS_LIST_FILE)
}

fn action_resolve_drafts_list_path(project_root: &Path) -> Result<PathBuf, String> {
    let meta = project_root.join(".project");
    fs::create_dir_all(&meta)
        .map_err(|e| format!("failed to create {}: {}", meta.display(), e))?;
    Ok(calc_primary_drafts_list_path(project_root))
}

fn action_save_drafts_list_primary_with_legacy_mirror(
    project_root: &Path,
    doc: &DraftsListDoc,
) -> Result<(), String> {
    let primary = action_resolve_drafts_list_path(project_root)?;
    action_save_drafts_list(&primary, doc)
}

fn calc_generate_project_id(existing: &HashSet<String>) -> String {
    const ALNUM: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    for _ in 0..512 {
        let mut out = String::with_capacity(4);
        for _ in 0..4 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            let idx = (seed as usize) % ALNUM.len();
            out.push(ALNUM[idx] as char);
        }
        if !existing.contains(&out) {
            return out;
        }
    }
    "0000".to_string()
}

fn action_normalize_registry(registry: &mut ProjectRegistry) -> bool {
    let mut changed = false;
    let mut ids: HashSet<String> = registry
        .projects
        .iter()
        .filter_map(|p| if p.id.is_empty() { None } else { Some(p.id.clone()) })
        .collect();
    for project in &mut registry.projects {
        if project.id.is_empty() {
            let id = calc_generate_project_id(&ids);
            ids.insert(id.clone());
            project.id = id;
            changed = true;
        }
    }
    if let Some(recent_id) = &registry.recent_active_pane {
        if !registry.projects.iter().any(|p| &p.id == recent_id) {
            registry.recent_active_pane = None;
            changed = true;
        }
    }
    changed
}

pub(crate) fn calc_model_supports_dangerous_flag(model_bin: &str) -> bool {
    model_bin.eq_ignore_ascii_case("codex")
}

pub(crate) fn action_default_model_bin() -> String {
    action_load_app_config()
        .and_then(|c| c.ai.as_ref().and_then(|a| a.model.as_ref()).cloned())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "codex".to_string())
}

fn action_read_one_line(prompt: &str) -> Result<String, String> {
    print!("{}", prompt);
    io::stdout()
        .flush()
        .map_err(|e| format!("failed to flush stdout: {}", e))?;
    let mut buf = Vec::new();
    io::stdin()
        .lock()
        .read_until(b'\n', &mut buf)
        .map_err(|e| format!("failed to read stdin: {}", e))?;
    let input = String::from_utf8_lossy(&buf);
    Ok(input.trim().to_string())
}

fn action_read_multiline_until_blank(prompt: &str) -> Result<String, String> {
    println!("{}", prompt);
    println!("(붙여넣기 가능, 입력 종료: 빈 줄 1회)");
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    let mut lines = Vec::new();
    loop {
        let mut buf = Vec::new();
        let n = lock
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("failed to read stdin: {}", e))?;
        if n == 0 {
            break;
        }
        let line = String::from_utf8_lossy(&buf)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if line.trim().is_empty() {
            break;
        }
        lines.push(line);
    }
    Ok(lines.join("\n").trim().to_string())
}

pub(crate) fn show_current_state(state: &str, description: &str) {
    println!("[{}]{}", state, description);
}

pub(crate) fn action_run_codex_exec_capture_with_timeout(
    prompt: &str,
    timeout_sec: u64,
) -> Result<String, String> {
    chat::action_run_codex_exec_capture_with_timeout(prompt, timeout_sec)
}

fn action_run_codex_exec_capture(prompt: &str) -> Result<String, String> {
    chat::action_run_codex_exec_capture(prompt)
}

fn action_run_codex_exec_capture_in_dir(dir: &Path, prompt: &str) -> Result<String, String> {
    chat::action_run_codex_exec_capture_in_dir(dir, prompt)
}

pub(crate) fn action_run_codex_exec_capture_in_dir_with_timeout(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
) -> Result<String, String> {
    chat::action_run_codex_exec_capture_in_dir_with_timeout(dir, prompt, timeout_sec)
}

fn action_run_llm_exec_capture(llm: &str, prompt: &str) -> Result<String, String> {
    chat::action_run_llm_exec_capture(llm, prompt)
}

fn calc_extract_markdown_block(raw: &str) -> String {
    if let Some(start) = raw.find("```markdown") {
        let rest = &raw[start + 11..];
        if let Some(end) = rest.find("```") {
            return rest[..end].trim().to_string();
        }
    }
    if let Some(start) = raw.find("# info") {
        return raw[start..].trim().to_string();
    }
    raw.trim().to_string()
}

fn action_validate_project_md_format(project_md: &str) -> Result<(), String> {
    let required_headers = [
        "# info",
        "## rule",
        "## plan",
        "## features",
        "## structure",
        "# Domains",
        "# UI",
        "# Step",
        "# Constraints",
        "# Verification",
        "# Gate Checklist",
    ];
    for header in required_headers {
        if !project_md.lines().any(|line| line.trim().eq_ignore_ascii_case(header)) {
            return Err(format!("project.md format invalid: missing header `{}`", header));
        }
    }
    let has_flow = project_md
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("# Flow"));
    let has_stage = project_md
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("# Stage"));
    if !has_flow && !has_stage {
        return Err("project.md format invalid: missing header `# Flow` or `# Stage`".to_string());
    }
    for banned in ["- 제안 도메인:", "- 근거:", "- 책임:"] {
        if project_md.contains(banned) {
            return Err(format!(
                "project.md format invalid: banned domains summary style `{}`",
                banned
            ));
        }
    }
    if !project_md.contains("### domain") {
        return Err("project.md format invalid: missing `### domain` block".to_string());
    }
    for required in [
        "- **name**:",
        "- **description**:",
        "- **state**:",
        "- **action**:",
        "- **rule**:",
        "- **variable**:",
    ] {
        if !project_md.contains(required) {
            return Err(format!(
                "project.md format invalid: missing domain field `{}`",
                required
            ));
        }
    }
    Ok(())
}

fn action_normalize_project_md_min_sections(project_md: &str) -> String {
    let mut out = project_md.trim().to_string();
    let required_headers = [
        "# info",
        "## rule",
        "## plan",
        "## features",
        "## structure",
        "# Domains",
        "# UI",
        "# Step",
        "# Constraints",
        "# Verification",
        "# Gate Checklist",
    ];
    for header in required_headers {
        if !out.lines().any(|line| line.trim().eq_ignore_ascii_case(header)) {
            out.push_str(&format!("\n\n{}\n- TODO\n", header));
        }
    }
    let has_flow = out
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("# Flow"));
    let has_stage = out
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("# Stage"));
    if !has_flow && !has_stage {
        out.push_str("\n\n# Flow\n- TODO\n");
    }
    if !out.contains("### domain") {
        out.push_str(
            "\n\n### domain\n- **name**: `core`\n- **description**: 기본 도메인\n- **state**: 초안\n- **action**: 생성\n",
        );
    } else {
        for required in [
            "- **name**:",
            "- **description**:",
            "- **state**:",
            "- **action**:",
            "- **rule**:",
            "- **variable**:",
        ] {
            if !out.contains(required) {
                out.push_str(&format!("\n{}\n- TODO\n", required));
            }
        }
    }
    out
}

fn calc_render_template_pairs(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in pairs {
        let plain = format!("{{{{{}}}}}", key);
        let spaced = format!("{{{{ {} }}}}", key);
        rendered = rendered.replace(&plain, value).replace(&spaced, value);
    }
    rendered
}

fn calc_collect_unresolved_placeholders(rendered: &str, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .filter_map(|key| {
            let plain = format!("{{{{{}}}}}", key);
            let spaced = format!("{{{{ {} }}}}", key);
            if rendered.contains(&plain) || rendered.contains(&spaced) {
                Some((*key).to_string())
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn action_append_failure_log(task_name: &str, reason: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(EXEC_LOG_PATH).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(EXEC_LOG_PATH)
        .map_err(|e| format!("failed to open {}: {}", EXEC_LOG_PATH, e))?;
    writeln!(
        f,
        "- task 이름: {} | 실패 시각: {} | 실패 사유: {}",
        task_name,
        calc_now_unix(),
        reason
    )
    .map_err(|e| format!("failed to write {}: {}", EXEC_LOG_PATH, e))
}

fn action_load_registry(path: &Path) -> Result<ProjectRegistry, String> {
    if !path.exists() {
        let legacy = action_legacy_registry_path();
        if legacy.exists() {
            let raw = fs::read_to_string(&legacy)
                .map_err(|e| format!("failed to read {}: {}", legacy.display(), e))?;
            let mut parsed: ProjectRegistry =
                serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse yaml: {}", e))?;
            action_normalize_registry(&mut parsed);
            return Ok(parsed);
        }
        return Ok(ProjectRegistry::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut parsed: ProjectRegistry =
        serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse yaml: {}", e))?;
    action_normalize_registry(&mut parsed);
    Ok(parsed)
}

fn action_save_registry(path: &Path, registry: &ProjectRegistry) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let raw = serde_yaml::to_string(registry).map_err(|e| format!("yaml encode error: {}", e))?;
    fs::write(path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn calc_default_project_path() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn calc_is_existing_project(path: &Path) -> bool {
    path.join(".project").exists()
}

fn calc_select_only(registry: &ProjectRegistry, target: &str) -> ProjectRegistry {
    let projects = registry
        .projects
        .iter()
        .map(|p| ProjectRecord {
            selected: p.name == target,
            ..p.clone()
        })
        .collect();
    let mut updated = ProjectRegistry {
        recent_active_pane: registry.recent_active_pane.clone(),
        projects,
    };
    if let Some(selected) = updated.projects.iter().find(|p| p.selected) {
        updated.recent_active_pane = Some(selected.id.clone());
    }
    updated
}

fn action_upsert_project(
    registry: &ProjectRegistry,
    name: &str,
    path: &Path,
    description: &str,
) -> ProjectRegistry {
    let now = calc_now_unix();
    let mut updated = registry.projects.clone();
    let existing_ids: HashSet<String> = updated
        .iter()
        .filter_map(|p| if p.id.is_empty() { None } else { Some(p.id.clone()) })
        .collect();
    if let Some(existing) = updated.iter_mut().find(|p| p.name == name) {
        existing.path = path.display().to_string();
        existing.description = description.to_string();
        existing.updated_at = now;
        if existing.id.is_empty() {
            existing.id = calc_generate_project_id(&existing_ids);
        }
        return ProjectRegistry {
            recent_active_pane: registry.recent_active_pane.clone(),
            projects: updated,
        };
    }

    updated.push(ProjectRecord {
        id: calc_generate_project_id(&existing_ids),
        name: name.to_string(),
        path: path.display().to_string(),
        description: description.to_string(),
        created_at: now.clone(),
        updated_at: now,
        selected: false,
    });
    ProjectRegistry {
        recent_active_pane: registry.recent_active_pane.clone(),
        projects: updated,
    }
}

fn action_delete_project(registry: &ProjectRegistry, name: &str) -> ProjectRegistry {
    let projects = registry
        .projects
        .iter()
        .filter(|p| p.name != name)
        .cloned()
        .collect();
    let mut updated = ProjectRegistry {
        recent_active_pane: registry.recent_active_pane.clone(),
        projects,
    };
    if let Some(recent_id) = &updated.recent_active_pane {
        if !updated.projects.iter().any(|p| &p.id == recent_id) {
            updated.recent_active_pane = None;
        }
    }
    updated
}

fn action_ensure_project_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("failed to create {}: {}", path.display(), e))
}

fn list_projects() -> Result<String, String> {
    let registry = action_load_registry(&action_registry_path())?;
    Ok(ui::render_project_list(&registry.projects))
}

fn ui() -> Result<String, String> {
    let registry_path = action_registry_path();
    let mut registry = action_load_registry(&registry_path)?;
    let normalized = action_normalize_registry(&mut registry);
    action_save_registry(&registry_path, &registry)?;
    let result = ui::run_ui(&mut registry.projects, &mut registry.recent_active_pane)?;
    if normalized {
        registry.recent_active_pane = registry
            .recent_active_pane
            .as_ref()
            .and_then(|id| registry.projects.iter().find(|p| &p.id == id).map(|p| p.id.clone()));
    }
    if result.changed || normalized {
        action_save_registry(&registry_path, &registry)?;
    }
    if let Some(project_name) = result.auto_mode_project {
        return auto_mode(Some(&project_name));
    }
    Ok(result.message)
}

fn action_collect_project_features(project_path: &Path) -> Result<Vec<String>, String> {
    let drafts_list_path = action_resolve_drafts_list_path(project_path)?;
    let doc = action_load_drafts_list(&drafts_list_path)?;
    let mut out = doc.features;
    for planned in doc.planned {
        if !out.iter().any(|v| v == &planned) {
            out.push(planned);
        }
    }
    Ok(out)
}

fn calc_extract_project_md_list_by_header(project_md: &str, header: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_features = false;
    for line in project_md.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) {
            in_features = true;
            continue;
        }
        if in_features && trimmed.starts_with('#') {
            break;
        }
        if !in_features {
            continue;
        }
        let body = if trimmed.starts_with("- ") {
            trimmed.trim_start_matches("- ").trim().to_string()
        } else if let Some((_, right)) = trimmed.split_once(". ") {
            if trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                right.trim().to_string()
            } else {
                continue;
            }
        } else {
            continue;
        };
        if body.is_empty() {
            continue;
        }
        let mut key = body
            .split('|')
            .next()
            .unwrap_or(&body)
            .split(':')
            .next()
            .unwrap_or(&body)
            .trim();
        if key.starts_with("func_")
            && key.len() == 13
            && key
                .chars()
                .skip(5)
                .all(|ch| ch.is_ascii_hexdigit())
            && body.contains(':')
        {
            if let Some((_, right)) = body.split_once(':') {
                key = right.trim();
            }
        }
        if key.is_empty() {
            continue;
        }
        if body.trim().eq_ignore_ascii_case("todo") {
            continue;
        }
        if !out.iter().any(|v| v == key) {
            out.push(key.to_string());
        }
    }
    out
}

fn calc_extract_project_md_domain_names(project_md: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in project_md.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("- **name**:") {
            continue;
        }
        let mut value = trimmed
            .trim_start_matches("- **name**:")
            .trim()
            .trim_matches('`')
            .to_ascii_lowercase();
        value = calc_feature_name_snake_like(&value);
        if value.is_empty() || out.iter().any(|v| v == &value) {
            continue;
        }
        out.push(value);
    }
    out
}

fn calc_is_fileish_feature_key(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("src_")
        || lower.contains("_src_")
        || lower.ends_with("_ts")
        || lower.ends_with("_tsx")
        || lower.ends_with("_js")
        || lower.ends_with("_jsx")
        || lower.ends_with("_rs")
        || lower.ends_with("_yaml")
        || lower.ends_with("_yml")
        || lower.ends_with("_md")
}

fn calc_is_feature_key_like(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 48
        && !trimmed.chars().any(|ch| ch.is_ascii_whitespace())
        && !calc_is_fileish_feature_key(trimmed)
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn calc_feature_name_snake_like(input: &str) -> String {
    let mut out = String::new();
    let mut prev_is_alnum = false;
    for ch in input.chars() {
        if ch.is_ascii_uppercase() {
            if prev_is_alnum && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_is_alnum = true;
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            if out.is_empty() {
                if ch.is_ascii_lowercase() {
                    out.push(ch);
                    prev_is_alnum = true;
                }
            } else {
                out.push(ch.to_ascii_lowercase());
                prev_is_alnum = true;
            }
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            prev_is_alnum = false;
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "new_feature".to_string()
    } else {
        out
    }
}

fn calc_is_valid_snake_feature_key(value: &str) -> bool {
    if value.len() < 3 {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    value.contains('_')
        && !value.contains("__")
        && !value.ends_with('_')
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn calc_fallback_feature_key(raw: &str) -> String {
    let mut key = calc_map_korean_feature_keywords(raw).unwrap_or_else(|| calc_feature_name_snake_like(raw));
    if key != "new_feature"
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !key.contains('_')
    {
        key = format!("{}_task", key);
    }
    if key == "new_feature" || !calc_is_valid_snake_feature_key(&key) {
        if let Some(mapped) = calc_map_korean_feature_keywords(raw) {
            if calc_is_valid_snake_feature_key(&mapped) {
                return mapped;
            }
        }
        let mut hash: u32 = 2166136261;
        for b in raw.as_bytes() {
            hash ^= *b as u32;
            hash = hash.wrapping_mul(16777619);
        }
        key = format!("func_{:08x}", hash);
    }
    key
}

fn calc_map_korean_feature_keywords(raw: &str) -> Option<String> {
    let mappings = [
        ("Task", "task"),
        ("task", "task"),
        ("Todo", "todo"),
        ("todo", "todo"),
        ("생성", "create"),
        ("목록", "list"),
        ("상태", "state"),
        ("표시", "render"),
        ("완료", "complete"),
        ("토글", "toggle"),
        ("수정", "update"),
        ("삭제", "delete"),
        ("필터", "filter"),
        ("검색", "search"),
        ("영속화", "persist"),
        ("저장소", "store"),
        ("시작", "start"),
        ("오버레이", "overlay"),
        ("렌더링", "render"),
        ("렌더", "render"),
        ("클릭", "click"),
        ("입력", "input"),
        ("연결", "connect"),
        ("버튼", "button"),
        ("점프", "jump"),
        ("동작", "motion"),
        ("처리", "handle"),
        ("저장", "store"),
        ("상태", "state"),
        ("승리", "win"),
        ("조건", "condition"),
        ("판정", "check"),
        ("화면", "screen"),
        ("메뉴", "menu"),
        ("구성", "setup"),
    ];
    let mut found: Vec<(usize, &str)> = Vec::new();
    for (ko, en) in mappings {
        if let Some(idx) = raw.find(ko) {
            found.push((idx, en));
        }
    }
    if found.is_empty() {
        return None;
    }
    found.sort_by_key(|(idx, _)| *idx);
    let mut tokens: Vec<String> = Vec::new();
    for (_, token) in found {
        if !tokens.iter().any(|v| v == token) {
            tokens.push(token.to_string());
        }
    }
    if tokens.len() == 1 {
        tokens.push("task".to_string());
    }
    let key = tokens.join("_");
    if calc_is_valid_snake_feature_key(&key) {
        Some(key)
    } else {
        None
    }
}

fn calc_sync_llm_enabled() -> bool {
    matches!(env::var("ORC_SYNC_LLM").ok().as_deref(), Some("1"))
}

fn calc_feature_name_prompt_rules_from_skill() -> String {
    let fallback = "FEATURE_NAME 규칙:\n\
- 출력은 정확히 한 줄: FEATURE_NAME: <name>\n\
- <name>은 소문자 snake_case만 허용\n\
- 반드시 동사_명사 형태로 작성\n\
- 공백/하이픈/한글/설명문 금지";
    let Ok(raw) = fs::read_to_string(FEATURE_NAME_SKILL_PATH) else {
        return fallback.to_string();
    };
    let marker = "## Prompt Snippet";
    let Some(idx) = raw.find(marker) else {
        return fallback.to_string();
    };
    let mut lines = Vec::new();
    for line in raw[idx + marker.len()..].lines() {
        if line.starts_with("## ") {
            break;
        }
        let trimmed = line.trim_end();
        if !trimmed.trim().is_empty() {
            lines.push(trimmed.to_string());
        }
    }
    if lines.is_empty() {
        fallback.to_string()
    } else {
        lines.join("\n")
    }
}

fn action_normalize_feature_key_with_llm(raw: &str) -> String {
    if calc_is_feature_key_like(raw) {
        return calc_feature_name_snake_like(raw);
    }
    if !calc_sync_llm_enabled() {
        return calc_fallback_feature_key(raw);
    }
    let prompt = format!(
        "다음 기능명을 코드 폴더 key로 쓸 짧은 영문 snake_case(동사_명사)로 변환해.\n\
규칙은 아래 skill snippet을 우선 준수해.\n\
{}\n\
입력: {}",
        calc_feature_name_prompt_rules_from_skill(),
        raw
    );
    match action_run_codex_exec_capture(&prompt) {
        Ok(output) => {
            let name = calc_extract_feature_name(&output, raw);
            if !calc_is_valid_snake_feature_key(&name) || name == "new_feature" {
                calc_fallback_feature_key(raw)
            } else {
                name
            }
        }
        Err(_) => calc_fallback_feature_key(raw),
    }
}

fn action_generate_planned_items_with_llm(
    raw_items: &[String],
    available_domains: &[String],
) -> Vec<PlannedItem> {
    let inputs: Vec<String> = raw_items
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect();
    if inputs.is_empty() {
        return Vec::new();
    }
    if !calc_sync_llm_enabled() {
        let mut out = Vec::new();
        for raw in inputs {
            let mut name = calc_fallback_feature_key(&raw);
            if out.iter().any(|v: &PlannedItem| v.name == name) {
                name = format!("{}{}", name, out.len() + 1);
            }
            out.push(PlannedItem { name, value: raw });
        }
        return out;
    }
    let bullet = inputs
        .iter()
        .map(|v| format!("- {}", v))
        .collect::<Vec<_>>()
        .join("\n");
    let domains_text = if available_domains.is_empty() {
        "- (none)".to_string()
    } else {
        available_domains
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prompt = format!(
        "다음 planned 기능 후보를 코드 구현 단위로 정리해.\n\
출력은 반드시 YAML만:\n\
planned_items:\n\
  - name: <verb_noun snake_case 영문 키>\n\
    value: <원문 의미를 유지한 한 줄 설명>\n\
생성 절차:\n\
1) 한국어 문장을 자연스러운 영문 문장으로 변환\n\
2) 영문 문장을 2~4개 핵심 토큰으로 축약\n\
3) 현재 가능한 도메인 목록에서 가장 적절한 도메인을 고름\n\
4) name은 `<domain>_<verb>_<noun>` 또는 `<verb>_<noun>` 형태로 생성\n\
규칙:\n\
1) name은 중복 없이 영문 소문자 snake_case(동사_명사)\n\
2) value는 한국어 가능, 1줄\n\
3) 불필요한 설명문 금지\n\
현재 가능한 도메인 목록:\n{}\n\
입력:\n{}",
        domains_text,
        bullet
    );
    let Ok(raw) = action_run_codex_exec_capture(&prompt) else {
        return Vec::new();
    };
    let yaml = calc_extract_yaml_block(&raw);
    #[derive(Debug, Deserialize)]
    struct PlannedItemsDoc {
        #[serde(default)]
        planned_items: Vec<PlannedItem>,
    }
    let Ok(doc) = serde_yaml::from_str::<PlannedItemsDoc>(&yaml) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in doc.planned_items {
        let name = calc_feature_name_snake_like(&item.name);
        let value = item.value.trim().to_string();
        if value.is_empty()
            || !calc_is_valid_snake_feature_key(&name)
            || out.iter().any(|v: &PlannedItem| v.name == name)
        {
            continue;
        }
        out.push(PlannedItem { name, value });
    }
    out
}

fn calc_is_placeholder_planned_item(item: &PlannedItem) -> bool {
    let name = item.name.trim();
    let value = item.value.trim();
    matches!(
        name,
        "project_project_md"
            | "features_project_md_features"
            | "features_project_features_work_draft_yaml"
            | "draft_yaml"
    ) || value.contains(".project/project.md")
        || value.contains("project.md features")
        || value.contains("draft.yaml")
}

fn calc_should_force_tasks_list_resync(doc: &DraftsListDoc) -> bool {
    if doc.planned.is_empty() || doc.planned_items.is_empty() {
        return false;
    }
    let placeholder_planned = doc.planned.iter().all(|name| {
        matches!(
            name.trim(),
            "project_project_md"
                | "features_project_md_features"
                | "features_project_features_work_draft_yaml"
                | "draft_yaml"
        )
    });
    let placeholder_items = doc.planned_items.iter().all(calc_is_placeholder_planned_item);
    placeholder_planned && placeholder_items
}

fn calc_should_force_tasks_list_resync_by_md(
    doc: &DraftsListDoc,
    features_keys: &[String],
    planned_keys: &[String],
) -> bool {
    if doc.planned_items.iter().any(|item| {
        let value = item.value.trim();
        value == "프로젝트 정보 입력" || value == "features 리스트 입력" || value == "draft.yaml 읽기"
    }) {
        return true;
    }
    if !features_keys.is_empty() {
        let md: HashSet<String> = features_keys
            .iter()
            .map(|v| calc_feature_name_snake_like(v))
            .collect();
        let current: HashSet<String> = doc
            .features
            .iter()
            .map(|v| calc_feature_name_snake_like(v))
            .collect();
        if current.is_empty() || current.intersection(&md).count() == 0 {
            return true;
        }
    }
    if !planned_keys.is_empty() {
        let md: HashSet<String> = planned_keys
            .iter()
            .map(|v| calc_feature_name_snake_like(v))
            .collect();
        let current: HashSet<String> = doc
            .planned
            .iter()
            .map(|v| calc_feature_name_snake_like(v))
            .collect();
        if current.is_empty() || current.intersection(&md).count() == 0 {
            return true;
        }
    }
    false
}

pub(crate) fn action_sync_project_tasks_list_from_project_md(project_root: &Path) -> Result<bool, String> {
    let project_md_path = project_root.join(PROJECT_MD_PATH);
    let project_md = match fs::read_to_string(&project_md_path) {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };
    let plan_keys = calc_extract_project_md_list_by_header(&project_md, "## plan");
    let feature_keys = calc_extract_project_md_list_by_header(&project_md, "## features");
    let mut domain_keys = calc_extract_project_md_domain_names(&project_md);
    let (features_keys, planned_keys) = if !plan_keys.is_empty() {
        // new format: features=features, plan=planned
        (feature_keys, plan_keys)
    } else {
        // compatibility fallback: features=planned
        (Vec::new(), feature_keys)
    };
    let drafts_list_path = action_resolve_drafts_list_path(project_root)?;
    let mut doc = action_load_drafts_list(&drafts_list_path)?;
    for domain in &doc.domains {
        let key = calc_feature_name_snake_like(domain);
        if key.is_empty() || domain_keys.iter().any(|v| v == &key) {
            continue;
        }
        domain_keys.push(key);
    }
    let force_resync = calc_should_force_tasks_list_resync(&doc)
        || calc_should_force_tasks_list_resync_by_md(&doc, &features_keys, &planned_keys);
    if doc.sync_initialized
        && (!doc.features.is_empty() || !doc.planned.is_empty() || !doc.planned_items.is_empty())
        && !force_resync
    {
        return Ok(false);
    }
    if force_resync {
        doc.features.clear();
        doc.planned.clear();
        doc.planned_items.clear();
        doc.sync_initialized = false;
    }
    let before_features_len = doc.features.len();
    let before_planned_len = doc.planned.len();
    let before_planned_items_len = doc.planned_items.len();
    let mut cache: HashMap<String, String> = HashMap::new();
    let mut normalize_cached = |raw: &str| -> String {
        if let Some(existing) = cache.get(raw) {
            return existing.clone();
        }
        let normalized = action_normalize_feature_key_with_llm(raw);
        cache.insert(raw.to_string(), normalized.clone());
        normalized
    };

    let mut next_features = Vec::new();
    for raw in doc.features.iter().chain(features_keys.iter()) {
        let key = normalize_cached(raw);
        if !next_features.iter().any(|v| v == &key) {
            next_features.push(key);
        }
    }

    let mut planned_items_map: HashMap<String, String> = doc
        .planned_items
        .iter()
        .map(|item| (item.name.clone(), item.value.clone()))
        .collect();
    for raw in &doc.planned {
        let key = normalize_cached(raw);
        planned_items_map
            .entry(key)
            .or_insert_with(|| raw.trim().to_string());
    }
    let md_sentence_items: Vec<String> = planned_keys
        .iter()
        .filter(|raw| !calc_is_feature_key_like(raw))
        .cloned()
        .collect();
    for item in action_generate_planned_items_with_llm(&md_sentence_items, &domain_keys) {
        planned_items_map.insert(item.name, item.value);
    }

    let mut next_planned = Vec::new();
    for raw in doc.planned.iter().chain(planned_keys.iter()) {
        let key = normalize_cached(raw);
        if next_features.iter().any(|v| v == &key) || next_planned.iter().any(|v| v == &key) {
            continue;
        }
        planned_items_map
            .entry(key.clone())
            .or_insert_with(|| raw.trim().to_string());
        next_planned.push(key);
    }

    let next_planned_items: Vec<PlannedItem> = next_planned
        .iter()
        .map(|name| PlannedItem {
            name: name.clone(),
            value: planned_items_map
                .get(name)
                .cloned()
                .unwrap_or_else(|| name.clone()),
        })
        .collect();

    doc.features = next_features;
    doc.planned = next_planned;
    doc.planned_items = next_planned_items;
    doc.sync_initialized = true;

    if doc.features.len() == before_features_len
        && doc.planned.len() == before_planned_len
        && doc.planned_items.len() == before_planned_items_len
        && doc.sync_initialized
    {
        action_sync_draft_state_doc(project_root, &mut doc);
        action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc)?;
        return Ok(false);
    }
    action_sync_draft_state_doc(project_root, &mut doc);
    action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc)?;
    Ok(true)
}

fn action_sync_tasks_list_on_ui_open(projects: &[ProjectRecord]) -> Result<(), String> {
    for project in projects {
        let root = Path::new(&project.path);
        let _ = action_sync_project_tasks_list_from_project_md(root)?;
    }
    Ok(())
}

fn action_run_command_in_dir(
    dir: &Path,
    cmd: &str,
    args: &[&str],
    what: &str,
) -> Result<String, String> {
    let output = Command::new(cmd)
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| format!("failed to execute {}: {}", what, e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!(
            "{} failed (code={:?}) stderr=`{}` stdout=`{}`",
            what,
            output.status.code(),
            stderr,
            stdout
        ))
    }
}

fn auto_mode(project_name: Option<&str>) -> Result<String, String> {
    project::auto_mode(project_name)
}

fn auto_bootstrap(description: &str, spec: &str) -> Result<String, String> {
    project::auto_bootstrap(description, spec)
}

fn auto_check() -> Result<String, String> {
    project::auto_check()
}

fn auto_improve(request: &str) -> Result<String, String> {
    project::auto_improve(request)
}

fn run_feedback() -> Result<String, String> {
    feedback()
}

fn draft_report() -> Result<String, String> {
    project::draft_report()
}

fn create_project(name: &str, path: Option<&str>, description: &str) -> Result<String, String> {
    project::create_project(name, path, description)
}

fn select_project(name: &str) -> Result<String, String> {
    project::select_project(name)
}

fn delete_project(name: &str) -> Result<String, String> {
    project::delete_project(name)
}

fn draft_create() -> Result<String, String> {
    draft::draft_create()
}

fn draft_add(feature_name: &str, request: Option<String>) -> Result<String, String> {
    draft::draft_add(feature_name, request)
}

fn draft_delete(feature_name: &str) -> Result<String, String> {
    draft::draft_delete(feature_name)
}

#[derive(Debug, Clone)]
struct AddFunctionObject {
    name: String,
    steps: Vec<String>,
    rules: Vec<String>,
}

fn calc_parse_add_function_objects(raw: &str) -> Vec<AddFunctionObject> {
    let mut out = Vec::new();
    let mut current: Option<AddFunctionObject> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix('#') {
            if let Some(obj) = current.take() {
                out.push(obj);
            }
            current = Some(AddFunctionObject {
                name: name.trim().to_string(),
                steps: Vec::new(),
                rules: Vec::new(),
            });
            continue;
        }
        if let Some(step) = trimmed.strip_prefix('>') {
            if current.is_none() {
                current = Some(AddFunctionObject {
                    name: "new feature".to_string(),
                    steps: Vec::new(),
                    rules: Vec::new(),
                });
            }
            if let Some(obj) = current.as_mut() {
                obj.steps.push(step.trim().to_string());
            }
            continue;
        }
        if let Some(rule) = trimmed.strip_prefix('-') {
            if current.is_none() {
                current = Some(AddFunctionObject {
                    name: "new feature".to_string(),
                    steps: Vec::new(),
                    rules: Vec::new(),
                });
            }
            if let Some(obj) = current.as_mut() {
                obj.rules.push(rule.trim().to_string());
            }
            continue;
        }
        if current.is_none() {
            current = Some(AddFunctionObject {
                name: trimmed.to_string(),
                steps: vec![trimmed.to_string()],
                rules: Vec::new(),
            });
        } else if let Some(obj) = current.as_mut() {
            obj.steps.push(trimmed.to_string());
        }
    }
    if let Some(obj) = current.take() {
        out.push(obj);
    }
    out.into_iter()
        .map(|mut obj| {
            if obj.name.trim().is_empty() {
                obj.name = "new feature".to_string();
            }
            obj
        })
        .collect()
}

fn action_append_feature_to_project_md(feature_name: &str, display_name: &str) -> Result<(), String> {
    let path = Path::new(PROJECT_MD_PATH);
    let mut lines: Vec<String> = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", PROJECT_MD_PATH, e))?
        .lines()
        .map(|v| v.to_string())
        .collect();

    let feature_label = format!("{} : {}", feature_name, display_name.trim());
    if lines
        .iter()
        .any(|line| line.trim_start_matches("- ").trim().starts_with(feature_name))
    {
        return Ok(());
    }

    let header_idx = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("## plan"));
    let idx = if let Some(i) = header_idx {
        i
    } else {
        lines.push(String::new());
        lines.push("## plan".to_string());
        lines.push(String::new());
        lines.len() - 2
    };

    let mut end = idx + 1;
    while end < lines.len() {
        let t = lines[end].trim();
        if t.starts_with('#') {
            break;
        }
        end += 1;
    }
    lines.insert(end, format!("- {}", feature_label));
    fs::write(path, lines.join("\n") + "\n")
        .map_err(|e| format!("failed to write {}: {}", PROJECT_MD_PATH, e))
}

fn action_validate_todo_doc(doc: &TodoDoc) -> Result<(), String> {
    if doc.tasks.is_empty() {
        return Err("todo.yaml format invalid: tasks is empty".to_string());
    }
    let mut seen = HashSet::new();
    for (idx, task) in doc.tasks.iter().enumerate() {
        if task.name.trim().is_empty() {
            return Err(format!("todo.yaml format invalid: tasks[{idx}].name is empty"));
        }
        if !calc_is_valid_snake_feature_key(&task.name) {
            return Err(format!(
                "todo.yaml format invalid: tasks[{idx}].name is not snake_case"
            ));
        }
        if !seen.insert(task.name.clone()) {
            return Err(format!(
                "todo.yaml format invalid: duplicated task name `{}`",
                task.name
            ));
        }
        if task.draft_path.trim().is_empty() {
            return Err(format!(
                "todo.yaml format invalid: tasks[{idx}].draft_path is empty"
            ));
        }
    }
    Ok(())
}

fn action_save_todo_doc(path: &Path, doc: &TodoDoc) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let raw = serde_yaml::to_string(doc).map_err(|e| format!("todo yaml encode error: {}", e))?;
    fs::write(path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn action_load_todo_doc(path: &Path) -> Result<TodoDoc, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let doc: TodoDoc =
        serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    action_validate_todo_doc(&doc)?;
    Ok(doc)
}

fn action_build_todo_from_input_md() -> Result<String, String> {
    let input_path = Path::new(INPUT_MD_PATH);
    if !input_path.exists() {
        return Err(format!("{} not found", INPUT_MD_PATH));
    }
    let request_raw = fs::read_to_string(input_path)
        .map_err(|e| format!("failed to read {}: {}", INPUT_MD_PATH, e))?;
    let objects = calc_parse_add_function_objects(&request_raw);
    if objects.is_empty() {
        return Err("input.md parse failed: expected `# / > / -` format".to_string());
    }
    let todo_prompt_path = action_resolve_build_funciton_todo_prompt_path()?;
    let todo_prompt_template = fs::read_to_string(&todo_prompt_path)
        .map_err(|e| format!("failed to read {}: {}", todo_prompt_path.display(), e))?;
    let mut todo_doc = TodoDoc { tasks: Vec::new() };
    for obj in &objects {
        let rule_text = if obj.rules.is_empty() {
            "- (none)".to_string()
        } else {
            obj.rules
                .iter()
                .map(|v| format!("- {}", v))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let step_text = if obj.steps.is_empty() {
            "- (none)".to_string()
        } else {
            obj.steps
                .iter()
                .map(|v| format!("- {}", v))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let todo_prompt = todo_prompt_template
            .replace("{name}", &obj.name)
            .replace("{rule}", &rule_text)
            .replace("{step}", &step_text);
        let todo_raw = action_run_codex_exec_capture(&todo_prompt)?;
        let todo_yaml = action_normalize_todo_item_yaml(&calc_extract_yaml_block(&todo_raw))?;
        let generated: GeneratedTodoItem = serde_yaml::from_str(&todo_yaml)
            .map_err(|e| format!("generated todo item yaml invalid: {}", e))?;
        let fallback_name = calc_feature_name_snake_like(&obj.name);
        let mapped_name = if generated.name.trim().is_empty() {
            fallback_name
        } else {
            calc_feature_name_snake_like(&generated.name)
        };
        let item = TodoItem {
            name: mapped_name,
            display_name: if generated.display_name.trim().is_empty() {
                obj.name.clone()
            } else {
                generated.display_name
            },
            rule: generated.rule,
            step: generated.step,
            depends_on: generated.depends_on,
            draft_path: generated.draft_path,
        };
        todo_doc.tasks.push(item);
    }
    if todo_doc.tasks.is_empty() {
        return Err("generated todo yaml invalid: tasks is empty".to_string());
    }

    let prompt_path = action_resolve_build_funciton_prompt_path()?;
    let prompt_template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;

    for task in &mut todo_doc.tasks {
        if task.display_name.trim().is_empty() {
            task.display_name = task.name.clone();
        }
        let prompt = prompt_template
            .replace("{{name}}", &task.display_name)
            .replace(
                "{{rule}}",
                &if task.rule.is_empty() {
                    "- (none)".to_string()
                } else {
                    task.rule
                        .iter()
                        .map(|v| format!("- {}", v))
                        .collect::<Vec<_>>()
                        .join("\n")
                },
            )
            .replace(
                "{{step}}",
                &if task.step.is_empty() {
                    "- (none)".to_string()
                } else {
                    task.step
                        .iter()
                        .map(|v| format!("- {}", v))
                        .collect::<Vec<_>>()
                        .join("\n")
                },
            );
        let draft_raw = action_run_codex_exec_capture(&prompt)?;
        let draft_yaml = action_normalize_draft_task_step_yaml(&calc_extract_yaml_block(&draft_raw))?;
        let draft_doc: DraftDoc = serde_yaml::from_str(&draft_yaml)
            .map_err(|e| format!("generated draft yaml invalid: {}", e))?;
        let draft_issues = action_validate_draft_doc(&draft_doc);
        if !draft_issues.is_empty() {
            return Err(format!(
                "generated draft yaml invalid: {}",
                draft_issues.join(" | ")
            ));
        }
        let draft_path = ui::action_resolve_feature_draft_path(&task.name);
        if let Some(parent) = draft_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
        }
        fs::write(&draft_path, draft_yaml)
            .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
        task.depends_on = draft_doc.depends_on;
        task.draft_path = draft_path.display().to_string();
    }

    action_validate_todo_doc(&todo_doc)?;
    let todo_path = Path::new(TODOS_YAML_PATH);
    action_save_todo_doc(todo_path, &todo_doc)?;
    Ok(format!(
        "todo generated from {}: {} item(s) -> {}",
        INPUT_MD_PATH,
        todo_doc.tasks.len(),
        TODOS_YAML_PATH
    ))
}

fn action_resolve_build_funciton_prompt_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let file_name = "build-funciton.txt";
    let candidates = [
        root.join("assets").join("code").join("prompts").join(file_name),
        PathBuf::from("assets").join("code").join("prompts").join(file_name),
        root.join("src").join("assets").join("code").join("prompts").join(file_name),
        PathBuf::from("src").join("assets").join("code").join("prompts").join(file_name),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "build-function prompt not found: {} (source root: {})",
        file_name,
        root.display()
    ))
}

fn action_resolve_build_funciton_todo_prompt_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let file_name = "build-funciton-todo.txt";
    let candidates = [
        root.join("assets").join("code").join("prompts").join(file_name),
        PathBuf::from("assets").join("code").join("prompts").join(file_name),
        root.join("src").join("assets").join("code").join("prompts").join(file_name),
        PathBuf::from("src").join("assets").join("code").join("prompts").join(file_name),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "build-function todo prompt not found: {} (source root: {})",
        file_name,
        root.display()
    ))
}

fn action_ensure_project_md_exists(project_root: &Path) -> Result<Option<String>, String> {
    let project_dir = project_root.join(".project");
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;
    let project_md_path = project_dir.join("project.md");
    if project_md_path.exists() {
        return Ok(None);
    }
    let created = action_generate_project_md_from_workspace(project_root)?;
    if !project_md_path.exists() {
        return Err(format!(
            "failed to create {} from workspace",
            project_md_path.display()
        ));
    }
    Ok(Some(format!(
        "initialized missing project.md from workspace: {} | {}",
        project_md_path.display(),
        created
    )))
}

fn action_collect_workspace_file_hints(project_root: &Path) -> Result<Vec<String>, String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>, depth: usize) -> Result<(), String> {
        if depth > 4 || out.len() >= 60 {
            return Ok(());
        }
        let entries =
            fs::read_dir(dir).map_err(|e| format!("failed to read {}: {}", dir.display(), e))?;
        for entry in entries {
            if out.len() >= 60 {
                break;
            }
            let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name == "target" || name == ".project" {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, out, depth + 1)?;
                continue;
            }
            let rel = path
                .strip_prefix(base)
                .map(|v| v.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            out.push(rel);
        }
        Ok(())
    }

    let mut files = Vec::new();
    walk(project_root, project_root, &mut files, 0)?;
    files.sort();
    Ok(files)
}

fn action_infer_workspace_spec(file_hints: &[String]) -> String {
    let has = |name: &str| file_hints.iter().any(|v| v == name || v.ends_with(name));
    if has("Cargo.toml") {
        return "rust".to_string();
    }
    if has("package.json") {
        if has("next.config.js") || has("next.config.ts") {
            return "node, nextjs".to_string();
        }
        return "node".to_string();
    }
    if has("pyproject.toml") || has("requirements.txt") {
        return "python".to_string();
    }
    "workspace".to_string()
}

fn action_generate_project_md_from_workspace(project_root: &Path) -> Result<String, String> {
    let file_hints = action_collect_workspace_file_hints(project_root)?;
    let project_name = project_root
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("project");
    let file_text = if file_hints.is_empty() {
        "- (empty)".to_string()
    } else {
        file_hints
            .iter()
            .take(40)
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let description = format!(
        "현재 폴더 파일을 기준으로 생성된 프로젝트입니다.\n주요 파일:\n{}",
        file_text
    );
    let spec = action_infer_workspace_spec(&file_hints);
    let goal = "현재 워크스페이스 파일 구조를 기반으로 project.md 설계를 초기화한다.";
    action_generate_project_plan(
        project_root,
        project_name,
        &description,
        &spec,
        goal,
        &[],
        "",
        None,
        true,
    )
}

fn action_validate_todo_feedback_markdown(markdown: &str) -> Result<(), String> {
    let required = ["# 구현 기능", "## 문제 해결", "## 미해결", "## 개선점"];
    for header in required {
        if !markdown
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case(header))
        {
            return Err(format!(
                "feedback markdown format invalid: missing header `{}`",
                header
            ));
        }
    }
    Ok(())
}

fn action_write_todo_feedback(task_names: &[String], run_summary: &str) -> Result<String, String> {
    let task_text = if task_names.is_empty() {
        "- (none)".to_string()
    } else {
        task_names
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prompt = format!(
        "너는 구현 피드백 작성기다.\n\
입력:\n\
- 구현 대상 task 목록:\n{}\n\
- 실행 결과 요약: {}\n\n\
출력 규칙:\n\
1) markdown 본문만 출력.\n\
2) 아래 헤더를 정확히 유지.\n\
   - # 구현 기능\n\
   - ## 문제 해결\n\
   - ## 미해결\n\
   - ## 개선점\n\
3) 각 섹션마다 `- ` 불릿 최소 1개.\n\
4) 결과는 실제 실행 요약과 연결되게 작성.",
        task_text, run_summary
    );
    let raw = action_run_codex_exec_capture_with_timeout(&prompt, 120)?;
    let feedback_md = raw.trim().to_string();
    action_validate_todo_feedback_markdown(&feedback_md)?;
    let out_path = Path::new(".project").join("feedback.md");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    fs::write(&out_path, feedback_md + "\n")
        .map_err(|e| format!("failed to write {}: {}", out_path.display(), e))?;
    Ok(format!("feedback saved: {}", out_path.display()))
}

fn calc_extract_next_input_markdown_block(raw: &str) -> Option<String> {
    if let Some(start) = raw.find("```md") {
        let rest = &raw[start + 5..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    if let Some(start) = raw.find("```markdown") {
        let rest = &raw[start + 11..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('#') || line.starts_with('>') || line.starts_with('-'))
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn feedback() -> Result<String, String> {
    let feedback_path = Path::new(".project").join("feedback.md");
    if !feedback_path.exists() {
        return Err(format!("{} not found", feedback_path.display()));
    }
    let feedback_md = fs::read_to_string(&feedback_path)
        .map_err(|e| format!("failed to read {}: {}", feedback_path.display(), e))?;
    let input_body = fs::read_to_string(INPUT_MD_PATH).unwrap_or_default();
    let prompt = format!(
        "너는 다음 구현 사이클 결정기다.\n\
피드백:\n{}\n\n\
현재 input.md:\n{}\n\n\
결정 규칙:\n\
- 추가 구현이 필요하면 ACTION: NEXT 와 함께 input.md 본문(`#`, `>`, `-` 형식)을 출력.\n\
- 모든 작업이 완료되면 ACTION: DONE 만 출력.\n\
- 출력 형식:\n\
ACTION: <NEXT|DONE>\n\
```md\n\
<NEXT일 때만 input.md 본문>\n\
```",
        feedback_md, input_body
    );
    let raw = action_run_codex_exec_capture_with_timeout(&prompt, 120)?;
    let upper = raw.to_ascii_uppercase();
    if upper.contains("ACTION: DONE") {
        let input_path = Path::new(INPUT_MD_PATH);
        if input_path.exists() {
            fs::remove_file(input_path)
                .map_err(|e| format!("failed to delete {}: {}", INPUT_MD_PATH, e))?;
        }
        return Ok(format!("feedback completed: removed {}", INPUT_MD_PATH));
    }
    if !upper.contains("ACTION: NEXT") {
        return Err("feedback decision invalid: missing ACTION: NEXT|DONE".to_string());
    }
    let next_input = calc_extract_next_input_markdown_block(&raw)
        .ok_or_else(|| "feedback NEXT output invalid: missing markdown block".to_string())?;
    if next_input.trim().is_empty() {
        return Err("feedback NEXT output invalid: empty input body".to_string());
    }
    fs::write(INPUT_MD_PATH, next_input + "\n")
        .map_err(|e| format!("failed to write {}: {}", INPUT_MD_PATH, e))?;
    Ok(format!("feedback completed: updated {}", INPUT_MD_PATH))
}

async fn build_function_auto() -> Result<String, String> {
    let mut cycle_logs = Vec::new();
    for cycle in 1..=5usize {
        if !Path::new(INPUT_MD_PATH).exists() {
            if cycle == 1 {
                return Err(format!("{} not found", INPUT_MD_PATH));
            }
            break;
        }
        show_current_state("plan", &format!("cycle {}: input.md -> todos.yaml 생성 시작", cycle));
        let todo_msg = action_build_todo_from_input_md()?;
        if let Some(msg) = action_ensure_project_md_exists(Path::new("."))? {
            cycle_logs.push(format!("cycle {} | {}", cycle, msg));
        }
        show_current_state("build", &format!("cycle {}: todo 병렬 구현 시작", cycle));
        let build_msg = parallel::run_parallel_todo().await?;
        let todo_doc = action_load_todo_doc(Path::new(TODOS_YAML_PATH))?;
        let todo_names: Vec<String> = todo_doc.tasks.iter().map(|v| v.name.clone()).collect();
        show_current_state("chcek", &format!("cycle {}: feedback.md 작성/검토 시작", cycle));
        let feedback_write_msg = action_write_todo_feedback(&todo_names, &build_msg)?;
        let feedback_msg = feedback()?;
        cycle_logs.push(format!(
            "cycle {} | {} | {} | {} | {}",
            cycle, todo_msg, build_msg, feedback_write_msg, feedback_msg
        ));
        if !Path::new(INPUT_MD_PATH).exists() {
            break;
        }
    }
    if Path::new(INPUT_MD_PATH).exists() {
        cycle_logs.push("stopped: max cycle reached with remaining input.md".to_string());
    }
    Ok(format!(
        "build-function-auto completed\n{}",
        cycle_logs.join("\n")
    ))
}

fn add_func(request_input: Option<String>) -> Result<String, String> {
    let project_md = fs::read_to_string(PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", PROJECT_MD_PATH, e))?;
    let project_info = calc_extract_project_info(&project_md);
    let project_rules = calc_extract_project_rules(&project_md);
    let request_raw = match request_input {
        Some(v) if !v.trim().is_empty() => v,
        _ => action_read_multiline_until_blank(
            "기능 입력(`# 이름`, `> step`, `- 규칙`) - 여러 객체 가능:",
        )?,
    };
    if request_raw.trim().is_empty() {
        return Err("add-func requires non-empty feature request".to_string());
    }
    let objects = calc_parse_add_function_objects(&request_raw);
    if objects.is_empty() {
        return Err("add-func parse failed: expected `# / > / -` format".to_string());
    }
    let mut created = Vec::new();
    let mut created_features = Vec::new();
    for obj in &objects {
        let draft_prompt = format!(
            "너는 rust-orc 프로젝트의 draft 작성기다.\nproject info:\n{}\n\nproject rules:\n- {}\n\n입력 객체:\n- name: {}\n- step:\n{}\n- rule:\n{}\n\n지시:\n- `draft.yaml`은 템플릿(`assets/code/templates/draft.yaml`)을 대상 폴더에 먼저 복사한 뒤, 주석/예시를 지우고 값만 수정해.\n- 규칙은 `$plan-drafts-code`, `$rule-naming` 스킬을 사용해.\n- YAML 중복 키를 절대 만들지 마(특히 `rule`/`contracts`).\n- `task` 키는 `name,type,domain,depends_on,scope,rule,step,touches,contracts`만 허용.\n- `contracts`는 `key=value` 또는 `key: value` 형식으로만 작성하고 `contract` 키는 금지.\n출력 형식:\nFEATURE_NAME: <snake_case>\n```yaml\n<draft.yaml 본문>\n```\n설명 문장 금지.",
            project_info,
            project_rules.join("\n- "),
            obj.name,
            if obj.steps.is_empty() {
                "- (none)".to_string()
            } else {
                obj.steps
                    .iter()
                    .map(|v| format!("- {}", v))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            if obj.rules.is_empty() {
                "- (none)".to_string()
            } else {
                obj.rules
                    .iter()
                    .map(|v| format!("- {}", v))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        );
        let draft_raw = action_run_codex_exec_capture(&draft_prompt)?;
        let feature_name = calc_extract_feature_name(&draft_raw, &obj.name);
        let draft_yaml = action_normalize_draft_task_step_yaml(&calc_extract_yaml_block(&draft_raw))?;
        let draft_doc: DraftDoc = serde_yaml::from_str(&draft_yaml)
            .map_err(|e| format!("generated draft yaml invalid: {}", e))?;
        let draft_issues = action_validate_draft_doc(&draft_doc);
        if !draft_issues.is_empty() {
            return Err(format!(
                "generated draft yaml invalid: {}",
                draft_issues.join(" | ")
            ));
        }

        let draft_path = ui::action_resolve_feature_draft_path(&feature_name);
        if let Some(parent) = draft_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
        }
        fs::write(&draft_path, draft_yaml)
            .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
        action_append_feature_to_project_md(&feature_name, &obj.name)?;
        add_feature_to_planned(&feature_name)?;
        created_features.push(feature_name.clone());
        created.push(format!("{} -> {}", feature_name, draft_path.display()));
    }
    let check_msg = action_run_check_code_after_draft_changes(&created_features, "add-function")?;

    Ok(format!(
        "add-func completed: {} item(s) | {}\n{}",
        created.len(),
        check_msg,
        created.join("\n")
    ))
}

fn add_plan(request_input: Option<String>) -> Result<String, String> {
    plan::add_plan(request_input)
}

fn plan_project(llm: Option<&str>) -> Result<String, String> {
    let _ = llm;
    Err("plan-project removed. use `create-project <name> [path] [description]`".to_string())
}

fn action_generate_project_plan(
    project_root: &Path,
    project_name: &str,
    description: &str,
    spec: &str,
    goal: &str,
    user_rules: &[String],
    feature_request: &str,
    llm: Option<&str>,
    auto_mode: bool,
) -> Result<String, String> {
    let llm_bin_owned = llm
        .map(|v| v.to_string())
        .unwrap_or_else(action_default_model_bin);
    let llm_bin = llm_bin_owned.as_str();
    let rules_text = if user_rules.is_empty() {
        "- (작성 필요)".to_string()
    } else {
        user_rules
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prompt = match action_resolve_project_md_prompt_path(auto_mode)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
    {
        Some(template) => calc_render_template_pairs(
            &template,
            &[
                ("project_name", project_name),
                ("description", description),
                ("spec", spec),
                ("goal", goal),
                ("rules_text", &rules_text),
            ],
        ),
        None => format!(
            "너는 project.md 생성기다.\n입력:\n- name: {}\n- description: {}\n- spec: {}\n- goal: {}\n- rules:\n{}\n\n지시:\n- 템플릿(`assets/code/templates/project.md`) 구조를 정확히 따른다.\n- 필수 헤더를 모두 포함한다: `# info`, `## rule`, `## plan`, `## features`, `## structure`, `# Domains`, `# Flow`(또는 `# Stage`), `# UI`, `# Step`, `# Constraints`, `# Verification`, `# Gate Checklist`.\n- `# Domains`에는 `### domain` 블록을 최소 1개 포함하고, 각 블록에 `- **name**:`, `- **description**:`, `- **state**:`, `- **action**:`, `- **rule**:`, `- **variable**:`를 모두 포함한다.\n- 규칙은 `$plan-project-code`, `$build_domain` 스킬을 사용해.\n- 특히 도메인 설계는 `/home/tree/ai/skills/build-domain/SKILL.md`를 참조해 결정한다.\n- `## plan`에는 최소 5개의 기능 항목을 반드시 작성해.\n- 최종 출력은 markdown 본문만.",
            project_name, description, spec, goal, rules_text
        ),
    };
    let generated = action_run_llm_exec_capture(llm_bin, &prompt)?;
    let mut project_md = calc_extract_markdown_block(&generated);
    if !feature_request.trim().is_empty() {
        let parsed_features = calc_parse_add_function_objects(&feature_request);
        if !parsed_features.is_empty() {
            let mut lines: Vec<String> = project_md.lines().map(|v| v.to_string()).collect();
            let header_idx = lines
                .iter()
                .position(|line| line.trim().eq_ignore_ascii_case("## plan"));
            let idx = if let Some(i) = header_idx {
                i
            } else {
                lines.push(String::new());
                lines.push("## plan".to_string());
                lines.push(String::new());
                lines.len() - 2
            };
            let mut end = idx + 1;
            while end < lines.len() {
                if lines[end].trim().starts_with('#') {
                    break;
                }
                end += 1;
            }
            for obj in parsed_features {
                let key = action_normalize_feature_key_with_llm(&obj.name);
                let rule_text = if obj.rules.is_empty() {
                    "(rule 없음)".to_string()
                } else {
                    obj.rules.join(", ")
                };
                let step_text = if obj.steps.is_empty() {
                    "(step 없음)".to_string()
                } else {
                    obj.steps.join(" -> ")
                };
                lines.insert(end, format!("- {} : {} > {}", key, rule_text, step_text));
                end += 1;
            }
            project_md = lines.join("\n");
        }
    }
    project_md = action_normalize_project_md_min_sections(&project_md);
    action_validate_project_md_format(&project_md)?;
    let project_md_path = project_root.join(PROJECT_MD_PATH);
    if let Some(parent) = project_md_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    fs::write(&project_md_path, &project_md)
        .map_err(|e| format!("failed to write {}: {}", project_md_path.display(), e))?;
    let _ = action_sync_project_tasks_list_from_project_md(project_root)?;
    let bootstrap_status =
        ui::action_apply_bootstrap_by_spec(project_root, project_name, spec)?;
    Ok(format!(
        "create-project completed with {} -> {} | {}",
        llm_bin,
        project_md_path.display(),
        bootstrap_status
    ))
}

fn detail_project(llm: Option<&str>) -> Result<String, String> {
    let llm_bin_owned = llm
        .map(|v| v.to_string())
        .unwrap_or_else(action_default_model_bin);
    let llm_bin = llm_bin_owned.as_str();
    let current = fs::read_to_string(PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", PROJECT_MD_PATH, e))?;
    let project_template_path = action_resolve_project_template_path()?;
    let project_template = fs::read_to_string(&project_template_path)
        .map_err(|e| format!("failed to read {}: {}", project_template_path.display(), e))?;
    let prompt_template_path = action_resolve_detail_project_prompt_path()?;
    let prompt_template = fs::read_to_string(&prompt_template_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_template_path.display(), e))?;
    let context_hint = action_read_one_line("보강할 내용 힌트(없으면 Enter): ")?;
    let prompt = calc_render_template_pairs(
        &prompt_template,
        &[
            ("project_template", &project_template),
            ("current_project_md", &current),
            ("user_context_hint", &context_hint),
        ],
    );
    let unresolved = calc_collect_unresolved_placeholders(
        &prompt,
        &["project_template", "current_project_md", "user_context_hint"],
    );
    if !unresolved.is_empty() {
        return Err(format!(
            "detail-project prompt has unresolved placeholders: {}",
            unresolved.join(", ")
        ));
    }
    let generated = action_run_llm_exec_capture(llm_bin, &prompt)?;
    let project_md = calc_extract_markdown_block(&generated);
    action_validate_project_md_format(&project_md)?;
    fs::write(PROJECT_MD_PATH, &project_md)
        .map_err(|e| format!("failed to write {}: {}", PROJECT_MD_PATH, e))?;
    Ok(format!(
        "detail-project completed with {} -> {}",
        llm_bin, PROJECT_MD_PATH
    ))
}

fn detail_project_with_inputs(
    description: &str,
    spec: &str,
    llm: Option<&str>,
) -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let inferred_name = cwd
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("project");
    action_generate_project_plan(
        Path::new("."),
        inferred_name,
        description,
        spec,
        description,
        &[],
        "",
        llm,
        false,
    )
}

fn action_parse_draft_tasks(feature_name: &str) -> Result<Vec<DraftTask>, String> {
    let draft_path = ui::action_resolve_feature_draft_path(feature_name);
    let raw = fs::read_to_string(&draft_path)
        .map_err(|e| format!("failed to read {}: {}", draft_path.display(), e))?;
    let doc: DraftDoc =
        serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse draft yaml: {}", e))?;
    Ok(doc.task)
}

fn calc_validate_task_dependencies(tasks: &[DraftTask]) -> (Vec<String>, Vec<String>) {
    let all_names: Vec<String> = tasks.iter().map(|t| t.name.clone()).collect();
    let mut runnable = Vec::new();
    let mut blocked = Vec::new();
    for task in tasks {
        let ready = task.depends_on.iter().all(|d| all_names.iter().any(|n| n == d));
        if ready {
            runnable.push(task.name.clone());
        } else {
            blocked.push(task.name.clone());
        }
    }
    (runnable, blocked)
}

fn validate_tasks(feature_name: &str) -> Result<String, String> {
    let tasks = action_parse_draft_tasks(feature_name)?;
    let (runnable, blocked) = calc_validate_task_dependencies(&tasks);
    Ok(ui::render_task_validation(&runnable, &blocked))
}

fn calc_is_auto_verifiable_rule(rule: &str) -> bool {
    let s = rule.trim();
    if s.is_empty() {
        return false;
    }
    let ops = ["==", "!=", ">=", "<=", " matches ", " contains ", " exists("];
    ops.iter().any(|op| s.contains(op))
        || (s.contains(':') && (s.contains("must") || s.contains("should") || s.contains("check")))
}

fn calc_is_structured_constraint(contract: &str) -> bool {
    let s = contract.trim();
    if s.is_empty() {
        return false;
    }
    let has_key_value = s.contains(':') || s.contains('=');
    let has_membership_form = s.contains(" in [");
    let has_operator = ["==", "!=", ">=", "<=", "=", " in ", " matches ", " exists("]
        .iter()
        .any(|op| s.contains(op));
    (has_key_value || has_membership_form) && has_operator
}

fn action_validate_draft_doc(doc: &DraftDoc) -> Vec<String> {
    let mut issues = Vec::new();
    if doc.task.is_empty() {
        issues.push("task is empty".to_string());
        return issues;
    }
    let mut names = HashSet::new();
    for (idx, task) in doc.task.iter().enumerate() {
        let label = format!("task[{}]", idx);
        if task.name.trim().is_empty() {
            issues.push(format!("{label}: name is empty"));
        }
        if !task.name.trim().is_empty() && !names.insert(task.name.clone()) {
            issues.push(format!("{label}: duplicated task name `{}`", task.name));
        }
        if !matches!(task.task_type.as_str(), "calc" | "action") {
            issues.push(format!("{label}: type must be `calc` or `action`"));
        }
        if task.domain.is_empty() || task.domain.iter().all(|v| v.trim().is_empty()) {
            issues.push(format!("{label}: domain is empty"));
        }
        if task.scope.is_empty() || task.scope.iter().all(|v| v.trim().is_empty()) {
            issues.push(format!("{label}: scope is empty"));
        }
        if task.step.is_empty() || task.step.iter().all(|v| v.trim().is_empty()) {
            issues.push(format!("{label}: step is empty"));
        }
        if task.rule.is_empty() {
            issues.push(format!("{label}: rule is empty"));
        } else {
            for (ridx, rule) in task.rule.iter().enumerate() {
                if !calc_is_auto_verifiable_rule(rule) {
                    issues.push(format!(
                        "{label}: rule[{ridx}] is not auto-verifiable (`{}`)",
                        rule
                    ));
                }
            }
        }
        for (cidx, contract) in task.contracts.iter().enumerate() {
            if !calc_is_structured_constraint(contract) {
                issues.push(format!(
                    "{label}: contracts[{cidx}] is not structured (`{}`)",
                    contract
                ));
            }
        }
    }
    let known: HashSet<String> = doc.task.iter().map(|t| t.name.clone()).collect();
    for task in &doc.task {
        for dep in &task.depends_on {
            if dep == &task.name {
                issues.push(format!("task `{}` has self dependency", task.name));
            } else if !known.contains(dep) && !calc_is_valid_snake_feature_key(dep) {
                issues.push(format!(
                    "task `{}` has unknown depends_on `{}`",
                    task.name, dep
                ));
            }
        }
    }
    issues
}

fn action_resolve_draft_yaml_template_path() -> Option<PathBuf> {
    let root = action_source_root();
    let candidates = [
        root.join("assets").join("code").join("templates").join("draft.yaml"),
        PathBuf::from("assets").join("code").join("templates").join("draft.yaml"),
        root.join("assets").join("templates").join("draft.yaml"),
        PathBuf::from("assets").join("templates").join("draft.yaml"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn action_fix_draft_with_llm(draft_path: &Path, raw: &str, issues: &[String]) -> Result<String, String> {
    let template = action_resolve_draft_yaml_template_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .unwrap_or_default();
    let prompt = format!(
        "다음 draft.yaml을 검사 결과에 맞게 수정해.\n\
지시:\n\
- template YAML을 대상 draft 경로에 먼저 복사한 뒤, 주석/예시를 지우고 값만 수정해.\n\
- 규칙은 `$plan-drafts-code`, `$check-code` 스킬을 사용해.\n\
- YAML 중복 키를 절대 만들지 마(특히 `rule`/`contracts` 중복 금지).\n\
- `task` 객체 키는 `name,type,domain,depends_on,scope,rule,step,touches,contracts`만 사용.\n\
- `contract`(단수) 키는 사용 금지, 반드시 `contracts`(복수)만 사용.\n\
- `contracts` 각 항목은 `key=value` 또는 `key: value`만 허용.\n\
- 최종 출력은 YAML 본문만.\n\
검사 결과:\n{}\n\n\
template:\n{}\n\n\
current:\n{}",
        issues
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n"),
        template,
        raw
    );
    let output = action_run_codex_exec_capture(&prompt)?;
    let fixed = calc_extract_yaml_block(&output);
    let _: DraftDoc = serde_yaml::from_str(&fixed)
        .map_err(|e| format!("llm fixed draft parse failed {}: {}", draft_path.display(), e))?;
    Ok(fixed)
}

pub(crate) fn action_check_and_improve_drafts_before_parallel() -> Result<String, String> {
    let root = Path::new(".project").join("feature");
    if !root.exists() {
        return Ok("check-draft skipped: no feature directory".to_string());
    }
    let mut checked = 0usize;
    let mut fixed = 0usize;
    let entries =
        fs::read_dir(&root).map_err(|e| format!("failed to read {}: {}", root.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        if !entry
            .file_type()
            .map_err(|e| format!("failed to read file type: {}", e))?
            .is_dir()
        {
            continue;
        }
        let dir = entry.path();
        let draft_path = [dir.join("draft.yaml"), dir.join("drafts.yaml")]
            .into_iter()
            .find(|p| p.exists());
        let Some(draft_path) = draft_path else { continue };
        checked += 1;
        let raw = fs::read_to_string(&draft_path)
            .map_err(|e| format!("failed to read {}: {}", draft_path.display(), e))?;
        let doc: DraftDoc = serde_yaml::from_str(&raw)
            .map_err(|e| format!("failed to parse draft {}: {}", draft_path.display(), e))?;
        let issues = action_validate_draft_doc(&doc);
        if issues.is_empty() {
            continue;
        }
        let fixed_yaml = action_fix_draft_with_llm(&draft_path, &raw, &issues)?;
        let fixed_doc: DraftDoc = serde_yaml::from_str(&fixed_yaml)
            .map_err(|e| format!("fixed draft parse failed {}: {}", draft_path.display(), e))?;
        let remain = action_validate_draft_doc(&fixed_doc);
        if !remain.is_empty() {
            return Err(format!(
                "check-draft unresolved {}: {}",
                draft_path.display(),
                remain.join(" | ")
            ));
        }
        fs::write(&draft_path, &fixed_yaml)
            .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
        fixed += 1;
    }
    Ok(format!("check-draft done: checked={}, fixed={}", checked, fixed))
}

pub(crate) fn action_load_app_config() -> Option<config::AppConfig> {
    let root = action_source_root();
    let candidates = [
        root.join("configs.yaml"),
        root.join("config.yaml"),
        root.join("assets").join("config").join("config.yaml"),
        root.join("src").join("assets").join("config").join("config.yaml"),
    ];
    for candidate in candidates {
        if let Ok(conf) = config::AppConfig::load_from_path(&candidate) {
            return Some(conf);
        }
    }
    None
}

fn calc_extract_project_info(project_md: &str) -> String {
    let mut in_info = false;
    let mut lines = Vec::new();
    for line in project_md.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("# info") {
            in_info = true;
            continue;
        }
        if in_info && trimmed.starts_with("## ") {
            break;
        }
        if in_info {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        project_md.to_string()
    } else {
        lines.join("\n").trim().to_string()
    }
}

fn calc_extract_project_rules(project_md: &str) -> Vec<String> {
    let mut in_rule = false;
    let mut out = Vec::new();
    for line in project_md.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## rule") {
            in_rule = true;
            continue;
        }
        if in_rule && trimmed.starts_with("## ") {
            break;
        }
        if in_rule && trimmed.starts_with("- ") {
            out.push(trimmed.trim_start_matches("- ").trim().to_string());
        }
    }
    out
}

fn calc_extract_bullet_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- "))
        .map(|line| line.trim_start_matches("- ").trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn calc_extract_yaml_block(raw: &str) -> String {
    if let Some(start) = raw.find("```yaml") {
        let rest = &raw[start + 7..];
        if let Some(end) = rest.find("```") {
            return rest[..end].trim().to_string();
        }
    }
    if let Some(start) = raw.find("rule:") {
        return raw[start..].trim().to_string();
    }
    raw.trim().to_string()
}

fn action_normalize_draft_task_step_yaml(raw_yaml: &str) -> Result<String, String> {
    fn value_to_text(v: &serde_yaml::Value) -> String {
        match v {
            serde_yaml::Value::String(s) => s.trim().to_string(),
            serde_yaml::Value::Mapping(map) => {
                let mut parts = Vec::new();
                for (k, val) in map {
                    let key = match k {
                        serde_yaml::Value::String(s) => s.clone(),
                        other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
                    };
                    let value = match val {
                        serde_yaml::Value::String(s) => s.clone(),
                        other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
                    };
                    if !key.is_empty() && !value.is_empty() {
                        parts.push(format!("{}: {}", key, value));
                    }
                }
                parts.join(" | ")
            }
            other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
        }
    }

    let mut root: serde_yaml::Value =
        serde_yaml::from_str(raw_yaml).map_err(|e| format!("generated draft yaml invalid: {}", e))?;
    let serde_yaml::Value::Mapping(root_map) = &mut root else {
        return Ok(raw_yaml.to_string());
    };
    let task_key = serde_yaml::Value::String("task".to_string());
    let Some(serde_yaml::Value::Sequence(tasks)) = root_map.get_mut(&task_key) else {
        return Ok(raw_yaml.to_string());
    };
    for task in tasks {
        let serde_yaml::Value::Mapping(task_map) = task else {
            continue;
        };
        let step_key = serde_yaml::Value::String("step".to_string());
        if let Some(serde_yaml::Value::Sequence(steps)) = task_map.get_mut(&step_key) {
            let mut normalized = Vec::with_capacity(steps.len());
            for step in steps.iter() {
                let text = value_to_text(step);
                if !text.is_empty() {
                    normalized.push(serde_yaml::Value::String(text));
                }
            }
            *steps = normalized;
        }

        let rule_key = serde_yaml::Value::String("rule".to_string());
        if let Some(serde_yaml::Value::Sequence(rules)) = task_map.get_mut(&rule_key) {
            let mut normalized = Vec::with_capacity(rules.len());
            for rule in rules.iter() {
                let mut text = value_to_text(rule);
                if !text.is_empty() && !calc_is_auto_verifiable_rule(&text) {
                    text = format!("check: {} should hold", text);
                }
                if !text.is_empty() {
                    normalized.push(serde_yaml::Value::String(text));
                }
            }
            *rules = normalized;
        }
    }
    serde_yaml::to_string(&root).map_err(|e| format!("generated draft yaml invalid: {}", e))
}

fn action_normalize_todo_item_yaml(raw_yaml: &str) -> Result<String, String> {
    fn value_to_text(v: &serde_yaml::Value) -> String {
        match v {
            serde_yaml::Value::String(s) => s.trim().to_string(),
            serde_yaml::Value::Mapping(map) => {
                let mut parts = Vec::new();
                for (k, val) in map {
                    let key = match k {
                        serde_yaml::Value::String(s) => s.clone(),
                        other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
                    };
                    let value = match val {
                        serde_yaml::Value::String(s) => s.clone(),
                        other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
                    };
                    if !key.is_empty() && !value.is_empty() {
                        parts.push(format!("{}: {}", key, value));
                    }
                }
                parts.join(" | ")
            }
            other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
        }
    }

    let mut root: serde_yaml::Value =
        serde_yaml::from_str(raw_yaml).map_err(|e| format!("generated todo item yaml invalid: {}", e))?;
    let serde_yaml::Value::Mapping(root_map) = &mut root else {
        return Ok(raw_yaml.to_string());
    };

    for key in ["rule", "step", "depends_on"] {
        let k = serde_yaml::Value::String(key.to_string());
        let Some(value) = root_map.get_mut(&k) else {
            continue;
        };
        match value {
            serde_yaml::Value::Sequence(items) => {
                let mut normalized = Vec::with_capacity(items.len());
                for item in items.iter() {
                    let text = value_to_text(item);
                    if !text.is_empty() {
                        normalized.push(serde_yaml::Value::String(text));
                    }
                }
                *items = normalized;
            }
            other => {
                let text = value_to_text(other);
                *other = if text.is_empty() {
                    serde_yaml::Value::Sequence(Vec::new())
                } else {
                    serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(text)])
                };
            }
        }
    }

    for key in ["name", "display_name", "draft_path"] {
        let k = serde_yaml::Value::String(key.to_string());
        if let Some(value) = root_map.get_mut(&k) {
            let text = value_to_text(value);
            *value = serde_yaml::Value::String(text);
        }
    }

    serde_yaml::to_string(&root).map_err(|e| format!("generated todo item yaml invalid: {}", e))
}

fn calc_extract_feature_name(raw: &str, fallback: &str) -> String {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("FEATURE_NAME:") {
            let candidate = calc_feature_name_snake_like(rest.trim());
            if !candidate.is_empty() {
                return candidate;
            }
        }
    }
    calc_feature_name_snake_like(fallback)
}

fn action_load_drafts_list(path: &Path) -> Result<DraftsListDoc, String> {
    if !path.exists() {
        return Ok(DraftsListDoc::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse drafts_list yaml: {}", e))
}

fn action_save_drafts_list(path: &Path, doc: &DraftsListDoc) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let raw = serde_yaml::to_string(doc).map_err(|e| format!("yaml encode error: {}", e))?;
    fs::write(path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn action_collect_generated_draft_feature_names(project_root: &Path) -> Vec<String> {
    let root = project_root.join(".project").join("feature");
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir = entry.path();
        let has_task = [
            dir.join("draft.yaml"),
            dir.join("tasks.yaml"),
            dir.join("draft.yaml"),
            dir.join("drafts.yaml"),
        ]
        .iter()
        .any(|p| p.exists());
        if !has_task {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            out.push(name.to_string());
        }
    }
    out.sort();
    out
}

fn action_sync_draft_state_doc(project_root: &Path, doc: &mut DraftsListDoc) {
    let generated = action_collect_generated_draft_feature_names(project_root);
    let generated_set: HashSet<&str> = generated.iter().map(String::as_str).collect();
    let pending = doc
        .planned
        .iter()
        .filter(|name| !generated_set.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    doc.draft_state.generated = generated;
    doc.draft_state.pending = pending;
}

fn action_resolve_project_md_path_for_flow() -> PathBuf {
    PathBuf::from(PROJECT_MD_PATH)
}

fn action_preflight_draft_create(path: &Path) -> Result<String, String> {
    let doc = action_load_drafts_list(path)?;
    if doc.planned.is_empty() {
        return Err("draft-preflight failed: drafts_list.yaml.planned is empty".to_string());
    }
    let features: HashSet<&str> = doc.features.iter().map(String::as_str).collect();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut overlap = Vec::new();
    let mut duplicate = Vec::new();
    let mut invalid_name = Vec::new();
    for name in &doc.planned {
        if features.contains(name.as_str()) {
            overlap.push(name.clone());
        }
        if !seen.insert(name.as_str()) {
            duplicate.push(name.clone());
        }
        if !calc_is_valid_snake_feature_key(name) {
            invalid_name.push(name.clone());
        }
    }
    if !overlap.is_empty() || !duplicate.is_empty() || !invalid_name.is_empty() {
        return Err(format!(
            "draft-preflight failed: overlap={:?}, duplicate={:?}, invalid_name={:?}",
            overlap, duplicate, invalid_name
        ));
    }
    Ok(format!("draft-preflight ok: planned={}", doc.planned.len()))
}

pub(crate) fn action_preflight_parallel_build(path: &Path) -> Result<String, String> {
    let doc = action_load_drafts_list(path)?;
    if doc.planned.is_empty() {
        return Err("parallel-preflight failed: drafts_list.yaml.planned is empty".to_string());
    }
    let mut missing = Vec::new();
    for name in &doc.planned {
        let dir = Path::new(".project").join("feature").join(name);
        let has_task = [
            dir.join("draft.yaml"),
            dir.join("tasks.yaml"),
            dir.join("draft.yaml"),
            dir.join("drafts.yaml"),
        ]
        .iter()
        .any(|p| p.exists());
        if !has_task {
            missing.push(name.clone());
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "parallel-preflight failed: missing draft/task file for planned={:?}",
            missing
        ));
    }
    Ok(format!(
        "parallel-preflight ok: planned={} files_ready={}",
        doc.planned.len(),
        doc.planned.len()
    ))
}

pub(crate) fn action_preflight_parallel_todo(path: &Path) -> Result<String, String> {
    let doc = action_load_todo_doc(path)?;
    let mut missing = Vec::new();
    for task in &doc.tasks {
        let task_path = Path::new(&task.draft_path);
        if !task_path.exists() {
            missing.push(task.name.clone());
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "parallel-todo-preflight failed: missing draft path for tasks={:?}",
            missing
        ));
    }
    Ok(format!(
        "parallel-todo-preflight ok: tasks={} files_ready={}",
        doc.tasks.len(),
        doc.tasks.len()
    ))
}

fn add_feature_to_planned(feature_name: &str) -> Result<(), String> {
    let path = action_resolve_drafts_list_path(Path::new("."))?;
    action_add_feature_to_planned_at(&path, feature_name)
}

fn action_add_feature_to_planned_doc(doc: &mut DraftsListDoc, feature_name: &str) -> bool {
    let mut changed = false;
    if !doc.features.iter().any(|v| v == feature_name) && !doc.planned.iter().any(|v| v == feature_name) {
        doc.planned.push(feature_name.to_string());
        changed = true;
    }
    if !doc.planned_items.iter().any(|v| v.name == feature_name) {
        doc.planned_items.push(PlannedItem {
            name: feature_name.to_string(),
            value: feature_name.to_string(),
        });
        changed = true;
    }
    changed
}

fn action_add_feature_to_planned_at(path: &Path, feature_name: &str) -> Result<(), String> {
    let mut doc = action_load_drafts_list(path)?;
    if action_add_feature_to_planned_doc(&mut doc, feature_name) {
        action_save_drafts_list(path, &doc)?;
    }
    Ok(())
}

fn action_promote_planned_to_features_doc(doc: &mut DraftsListDoc, items: &[String]) -> bool {
    let mut changed = false;
    for item in items {
        if doc.planned.iter().any(|v| v == item) {
            doc.planned.retain(|v| v != item);
            doc.planned_items.retain(|v| v.name != *item);
            changed = true;
        }
        if !doc.features.iter().any(|v| v == item) {
            doc.features.push(item.clone());
            changed = true;
        }
    }
    changed
}

fn calc_extract_list_key_from_markdown_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let body = if trimmed.starts_with("- ") {
        trimmed.trim_start_matches("- ").trim().to_string()
    } else if let Some((_, right)) = trimmed.split_once(". ") {
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            right.trim().to_string()
        } else {
            return None;
        }
    } else {
        return None;
    };
    if body.is_empty() {
        return None;
    }
    let head = body
        .split('|')
        .next()
        .unwrap_or(&body)
        .split(':')
        .next()
        .unwrap_or(&body)
        .trim();
    let key = calc_feature_name_snake_like(head);
    if calc_is_valid_snake_feature_key(&key) {
        Some(key)
    } else {
        None
    }
}

fn calc_markdown_section_bounds(lines: &[String], header: &str) -> Option<(usize, usize)> {
    let start = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case(header))?;
    let mut end = start + 1;
    while end < lines.len() {
        if lines[end].trim().starts_with('#') {
            break;
        }
        end += 1;
    }
    Some((start, end))
}

fn action_promote_project_md_plan_to_features(project_root: &Path, items: &[String]) -> Result<bool, String> {
    if items.is_empty() {
        return Ok(false);
    }
    let path = project_root.join(PROJECT_MD_PATH);
    if !path.exists() {
        return Ok(false);
    }
    let mut lines: Vec<String> = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?
        .lines()
        .map(|v| v.to_string())
        .collect();
    let targets: HashSet<String> = items
        .iter()
        .map(|v| calc_feature_name_snake_like(v))
        .filter(|v| calc_is_valid_snake_feature_key(v))
        .collect();
    if targets.is_empty() {
        return Ok(false);
    }
    let mut changed = false;
    if let Some((plan_start, plan_end)) = calc_markdown_section_bounds(&lines, "## plan") {
        let mut kept = Vec::new();
        for line in lines[(plan_start + 1)..plan_end].iter().cloned() {
            let key = calc_extract_list_key_from_markdown_line(&line);
            if key.as_ref().is_some_and(|k| targets.contains(k)) {
                changed = true;
                continue;
            }
            kept.push(line);
        }
        lines.splice((plan_start + 1)..plan_end, kept);
    }

    let mut features_bounds = calc_markdown_section_bounds(&lines, "## features");
    if features_bounds.is_none() {
        lines.push(String::new());
        lines.push("## features".to_string());
        lines.push(String::new());
        features_bounds = calc_markdown_section_bounds(&lines, "## features");
        changed = true;
    }
    if let Some((features_start, features_end)) = features_bounds {
        let mut existing: HashSet<String> = HashSet::new();
        for line in &lines[(features_start + 1)..features_end] {
            if let Some(key) = calc_extract_list_key_from_markdown_line(line) {
                existing.insert(key);
            }
        }
        let mut append_lines = Vec::new();
        for key in &targets {
            if existing.insert(key.clone()) {
                append_lines.push(format!("- {}", key));
                changed = true;
            }
        }
        if !append_lines.is_empty() {
            lines.splice(features_end..features_end, append_lines);
        }
    }
    if changed {
        fs::write(&path, lines.join("\n") + "\n")
            .map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    }
    Ok(changed)
}

fn action_promote_planned_to_features_at(path: &Path, items: &[String]) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }
    let mut doc = action_load_drafts_list(path)?;
    if action_promote_planned_to_features_doc(&mut doc, items) {
        action_save_drafts_list(path, &doc)?;
    }
    Ok(())
}

pub(crate) fn action_promote_planned_to_features(items: &[String]) -> Result<(), String> {
    let path = action_resolve_drafts_list_path(Path::new("."))?;
    action_promote_planned_to_features_at(&path, items)?;
    let _ = action_promote_project_md_plan_to_features(Path::new("."), items)?;
    Ok(())
}

pub(crate) fn action_move_finished_features_to_clear(items: &[String]) -> Result<String, String> {
    if items.is_empty() {
        return Ok("move-finished skipped: no completed feature".to_string());
    }
    let feature_root = Path::new(".project").join("feature");
    let clear_root = Path::new(".project").join("clear");
    fs::create_dir_all(&clear_root)
        .map_err(|e| format!("failed to create {}: {}", clear_root.display(), e))?;
    let mut moved = 0usize;
    for item in items {
        let src = feature_root.join(item);
        if !src.exists() {
            continue;
        }
        let dst = clear_root.join(item);
        if dst.exists() {
            fs::remove_dir_all(&dst)
                .map_err(|e| format!("failed to remove {}: {}", dst.display(), e))?;
        }
        fs::rename(&src, &dst)
            .map_err(|e| format!("failed to move {} -> {}: {}", src.display(), dst.display(), e))?;
        moved += 1;
    }
    Ok(format!("move-finished completed: moved={}", moved))
}

pub(crate) fn action_read_project_info() -> Result<String, String> {
    let path = action_resolve_project_md_path_for_flow();
    let project_md = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    Ok(calc_extract_project_info(&project_md))
}

fn calc_check_code_timeout_sec() -> u64 {
    action_load_app_config()
        .as_ref()
        .map_or(300, config::AppConfig::default_timeout_sec)
        .max(30)
}

fn action_append_check_code_runtime_log(stage: &str, detail: &str) {
    let runtime = Path::new(".project").join("runtime");
    if fs::create_dir_all(&runtime).is_err() {
        return;
    }
    let path = runtime.join("check-code.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {} | {}", calc_now_unix(), stage, detail);
    }
}

fn action_run_check_code_after_draft_changes(
    feature_names: &[String],
    trigger: &str,
) -> Result<String, String> {
    if feature_names.is_empty() {
        return Ok("check-code follow-up skipped: no draft target".to_string());
    }
    let mut target_lines = Vec::new();
    for name in feature_names {
        target_lines.push(format!(
            "- {}: .project/feature/{}/draft.yaml (or drafts.yaml)",
            name, name
        ));
    }
    let prompt = format!(
        "트리거: {}\n대상:\n{}\n\n지시:\n- `$check-code` 스킬을 사용해 점검/수정을 수행해.\n- YAML/Markdown 참조 파일이 있으면 먼저 읽고 값을 채워야 할 헤더/속성을 정리한 뒤 형식에 맞게 반영해.\n- 문제가 없으면 `NO_CHANGE`를 출력.",
        trigger,
        target_lines.join("\n")
    );
    let timeout_sec = calc_check_code_timeout_sec();
    action_append_check_code_runtime_log(
        "시작/프롬프트 전송",
        &format!("trigger={} timeout={}s", trigger, timeout_sec),
    );
    let debug_enabled = action_load_app_config()
        .as_ref()
        .is_none_or(config::AppConfig::debug_enabled);
    let wait_stop = Arc::new(AtomicBool::new(false));
    let heartbeat = if debug_enabled {
        let stop = Arc::clone(&wait_stop);
        Some(thread::spawn(move || {
            let mut elapsed = 0u64;
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(15));
                elapsed += 15;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                action_append_check_code_runtime_log(
                    "무응답 보호",
                    &format!("check-code LLM 응답 대기 중 ({}s)", elapsed),
                );
            }
        }))
    } else {
        None
    };
    let raw_result = action_run_codex_exec_capture_with_timeout(&prompt, timeout_sec);
    wait_stop.store(true, Ordering::Relaxed);
    if let Some(handle) = heartbeat {
        let _ = handle.join();
    }
    let raw = match raw_result {
        Ok(v) => v,
        Err(e) => {
            action_append_check_code_runtime_log("완료/실패", &format!("실패: {}", e));
            return Err(e);
        }
    };
    action_append_check_code_runtime_log("LLM 응답 수신", "check-code 응답 수신");
    let line = raw.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        action_append_check_code_runtime_log("완료/실패", "완료");
        Ok("check-code follow-up executed".to_string())
    } else {
        action_append_check_code_runtime_log("완료/실패", &format!("완료: {}", line));
        Ok(format!("check-code follow-up: {}", line))
    }
}

pub(crate) fn action_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn action_registry_path() -> PathBuf {
    action_source_root().join(REGISTRY_PATH)
}

fn action_legacy_registry_path() -> PathBuf {
    action_source_root().join(LEGACY_REGISTRY_PATH)
}

fn action_resolve_project_template_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let candidates = [
        root.join("assets").join("code").join("templates").join("project.md"),
        PathBuf::from("assets")
            .join("code")
            .join("templates")
            .join("project.md"),
        root.join("assets").join("templates").join("project.md"),
        PathBuf::from("assets").join("templates").join("project.md"),
        root.join("src").join("assets").join("templates").join("project.md"),
        PathBuf::from("src").join("assets").join("templates").join("project.md"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "project template not found (source root: {})",
        root.display()
    ))
}

fn action_resolve_drafts_list_template_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let candidates = [
        root.join("assets")
            .join("code")
            .join("templates")
            .join("drafts_list.yaml"),
        PathBuf::from("assets")
            .join("code")
            .join("templates")
            .join("drafts_list.yaml"),
        root.join("assets").join("templates").join("drafts_list.yaml"),
        PathBuf::from("assets").join("templates").join("drafts_list.yaml"),
        root.join("src").join("assets").join("templates").join("drafts_list.yaml"),
        PathBuf::from("src").join("assets").join("templates").join("drafts_list.yaml"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "drafts_list template not found (source root: {})",
        root.display()
    ))
}

fn action_resolve_detail_project_prompt_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let candidates = [
        root.join("assets")
            .join("code")
            .join("prompts")
            .join("detail-project.txt"),
        PathBuf::from("assets")
            .join("code")
            .join("prompts")
            .join("detail-project.txt"),
        root.join("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join("detail-project.txt"),
        PathBuf::from("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join("detail-project.txt"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "detail-project prompt not found (source root: {})",
        root.display()
    ))
}

pub(crate) fn action_resolve_project_md_prompt_path(auto_mode: bool) -> Result<PathBuf, String> {
    let root = action_source_root();
    let file_name = if auto_mode {
        "project-md-auto.txt"
    } else {
        "project-md-init.txt"
    };
    let candidates = [
        root.join("assets").join("code").join("prompts").join(file_name),
        PathBuf::from("assets").join("code").join("prompts").join(file_name),
        root.join("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join(file_name),
        PathBuf::from("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join(file_name),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "project-md prompt not found: {} (source root: {})",
        file_name,
        root.display()
    ))
}

pub(crate) fn action_resolve_task_template_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let candidates = [
        root.join("assets").join("code").join("prompts").join("tasks.txt"),
        PathBuf::from("assets")
            .join("code")
            .join("prompts")
            .join("tasks.txt"),
        root.join("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join("tasks.txt"),
        PathBuf::from("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join("tasks.txt"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "tasks template not found (source root: {})",
        root.display()
    ))
}

pub(crate) fn action_resolve_parallel_feedback_prompt_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let file_name = "parallel-feedback.txt";
    let candidates = [
        root.join("assets").join("code").join("prompts").join(file_name),
        PathBuf::from("assets").join("code").join("prompts").join(file_name),
        root.join("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join(file_name),
        PathBuf::from("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join(file_name),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "parallel feedback prompt not found: {} (source root: {})",
        file_name,
        root.display()
    ))
}

fn action_validate_parallel_feedback_markdown(markdown: &str) -> Result<(), String> {
    let required = [
        "# 구현 완료 피드백",
        "## 해결된 문제",
        "## 개선점",
        "## 다음 점검",
    ];
    for header in required {
        if !markdown
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case(header))
        {
            return Err(format!(
                "parallel feedback markdown format invalid: missing header `{}`",
                header
            ));
        }
    }
    Ok(())
}

pub(crate) fn action_write_parallel_feedback(
    finished_items: &[String],
    failed_count: usize,
    move_msg: &str,
) -> Result<String, String> {
    let prompt_path = action_resolve_parallel_feedback_prompt_path()?;
    let template = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("failed to read {}: {}", prompt_path.display(), e))?;
    let finished_text = if finished_items.is_empty() {
        "- (none)".to_string()
    } else {
        finished_items
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prompt = template
        .replace("{{finished_items}}", &finished_text)
        .replace("{{failed_count}}", &failed_count.to_string())
        .replace("{{move_msg}}", move_msg);
    let raw = action_run_codex_exec_capture_with_timeout(&prompt, 120)?;
    let feedback_md = raw.trim().to_string();
    action_validate_parallel_feedback_markdown(&feedback_md)?;
    let out_path = Path::new(".project").join("feedback.md");
    fs::write(&out_path, feedback_md + "\n")
        .map_err(|e| format!("failed to write {}: {}", out_path.display(), e))?;
    Ok(format!(
        "parallel feedback saved: {}",
        out_path.display()
    ))
}

fn action_is_directory_empty(path: &Path) -> Result<bool, String> {
    let mut entries =
        fs::read_dir(path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    Ok(entries.next().is_none())
}

pub(crate) fn action_initialize_parallel_workspace_if_empty(path: &Path) -> Result<Option<String>, String> {
    if !action_is_directory_empty(path)? {
        return Ok(None);
    }

    let project_dir = path.join(".project");
    fs::create_dir_all(project_dir.join("feature"))
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;
    fs::create_dir_all(project_dir.join("clear"))
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;

    let project_template_path = action_resolve_project_template_path()?;
    let template = fs::read_to_string(&project_template_path).map_err(|e| {
        format!(
            "failed to read project template {}: {}",
            project_template_path.display(),
            e
        )
    })?;
    fs::write(project_dir.join("project.md"), template).map_err(|e| {
        format!(
            "failed to write {}: {}",
            project_dir.join("project.md").display(),
            e
        )
    })?;
    let _ = action_sync_project_tasks_list_from_project_md(path)?;

    let drafts_list_path = project_dir.join("drafts_list.yaml");
    let drafts_template_path = action_resolve_drafts_list_template_path()?;
    let draft_template = fs::read_to_string(&drafts_template_path).map_err(|e| {
        format!(
            "failed to read drafts_list template {}: {}",
            drafts_template_path.display(),
            e
        )
    })?;
    fs::write(&drafts_list_path, draft_template)
        .map_err(|e| format!("failed to write {}: {}", drafts_list_path.display(), e))?;

    Ok(Some(format!(
        "workspace was empty; initialized parallel environment at {}",
        project_dir.display()
    )))
}

pub(crate) fn action_collect_parallel_feature_tasks() -> Result<Vec<ParallelFeatureTask>, String> {
    let root = Path::new(".project").join("feature");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries =
        fs::read_dir(&root).map_err(|e| format!("failed to read {}: {}", root.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        if !entry
            .file_type()
            .map_err(|e| format!("failed to read file type: {}", e))?
            .is_dir()
        {
            continue;
        }
        let feature_dir = entry.path();
        let draft_candidates = [
            feature_dir.join("draft.yaml"),
            feature_dir.join("tasks.yaml"),
            feature_dir.join("drafts.yaml"),
            feature_dir.join("draft.yaml"),
        ];
        let draft_path = match draft_candidates.into_iter().find(|p| p.exists()) {
            Some(path) => path,
            None => continue,
        };
        let raw = fs::read_to_string(&draft_path)
            .map_err(|e| format!("failed to read {}: {}", draft_path.display(), e))?;
        let doc: DraftDoc =
            serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse draft yaml: {}", e))?;
        let name = feature_dir
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let depends_on = doc.depends_on.clone();
        out.push(ParallelFeatureTask {
            name,
            draft_path,
            depends_on,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub(crate) fn action_collect_parallel_todo_tasks(path: &Path) -> Result<Vec<ParallelFeatureTask>, String> {
    let doc = action_load_todo_doc(path)?;
    let mut out = Vec::new();
    for task in doc.tasks {
        out.push(ParallelFeatureTask {
            name: task.name,
            draft_path: PathBuf::from(task.draft_path),
            depends_on: task.depends_on,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub(crate) fn action_build_task_prompt(
    task_template: &str,
    project_info: &str,
    draft_path: &Path,
) -> Result<String, String> {
    let draft_raw = fs::read_to_string(draft_path)
        .map_err(|e| format!("failed to read {}: {}", draft_path.display(), e))?;
    let rendered = calc_render_template_pairs(
        task_template,
        &[
            ("project_info", project_info),
            ("draft_path", &draft_path.display().to_string()),
            ("draft_content", &draft_raw),
        ],
    );
    let unresolved = calc_collect_unresolved_placeholders(
        &rendered,
        &["project_info", "draft_path", "draft_content"],
    );
    if !unresolved.is_empty() {
        return Err(format!(
            "tasks prompt has unresolved placeholders: {}",
            unresolved.join(", ")
        ));
    }
    let debug_enabled = action_load_app_config()
        .as_ref()
        .is_none_or(config::AppConfig::debug_enabled);
    if !debug_enabled {
        return Ok(rendered);
    }
    Ok(format!(
        "DEBUG MODE(on) 지시:\n- 응답 시작에 `DEBUG_LOG:` 한 줄을 먼저 작성해 현재 작업 단계와 대기 중이면 대기 사유를 기록해.\n- 장시간 작업이면 주요 진행 전환 시점마다 `DEBUG_LOG:`를 한 줄씩 추가해.\n\n{}",
        rendered
    ))
}

pub(crate) fn action_print_parallel_modal(statuses: &[(String, ui::TaskRuntimeState)]) {
    println!("{}", ui::render_parallel_modal(statuses));
}

#[tokio::main]
async fn main() {
    let _ = action_load_app_config();
    let args: Vec<String> = env::args().collect();
    let program = cli::calc_program_name(&args);
    if args.len() < 2 {
        cli::print_usage(program);
        return;
    }
    if cli::calc_is_help_command(&args) {
        cli::print_usage(program);
        return;
    }

    match cli::execute_cli(&args).await {
        Ok(output) => println!("{}", output),
        Err(err) => {
            eprintln!("{}", err);
            cli::print_usage(program);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let base = std::env::temp_dir();
        let uniq = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let dir = base.join(uniq);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn sync_project_md_to_tasks_list_populates_planned_and_items() {
        let root = make_temp_dir("orc_sync");
        let project_dir = root.join("project");
        let meta = project_dir.join(".project");
        fs::create_dir_all(&meta).expect("create project meta");
        let md = r#"# info
- name: sample

## features
- existingFeature

## plan
- promptPreprocessCli
- fileLogHelper
"#;
        fs::write(meta.join("project.md"), md).expect("write project.md");

        let changed =
            action_sync_project_tasks_list_from_project_md(&project_dir).expect("sync tasks_list");
        assert!(changed);

        let doc = action_load_drafts_list(&meta.join("drafts_list.yaml")).expect("load drafts_list");
        let features_key = calc_feature_name_snake_like("existing_feature");
        let planned_a = calc_feature_name_snake_like("prompt_preprocess_cli");
        let planned_b = calc_feature_name_snake_like("file_log_helper");
        assert!(doc.features.iter().any(|v| v == &features_key));
        assert!(doc.planned.iter().any(|v| v == &planned_a));
        assert!(doc.planned.iter().any(|v| v == &planned_b));
        assert!(
            doc.planned_items
                .iter()
                .any(|v| v.name == planned_a)
        );
        assert!(
            doc.planned_items
                .iter()
                .any(|v| v.name == planned_b)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn extract_project_md_list_prefers_action_segment_before_pipe() {
        let md = r#"# info
- name: sample

## plan
- 플레이어 입력 처리 구성 | src/input/playerInput.ts, src/game/updateLoop.ts | 입력 이벤트 전달
- 버튼 클릭 점프: cube를 누르면 점프한다 | src/features/jump/controller.tsx | 점프 실행
"#;
        let list = calc_extract_project_md_list_by_header(md, "## plan");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], "플레이어 입력 처리 구성");
        assert_eq!(list[1], "버튼 클릭 점프");
    }

    #[test]
    fn feature_key_like_rejects_fileish_path_style_names() {
        assert!(calc_is_feature_key_like("render_cube"));
        assert!(!calc_is_feature_key_like("src_features_game_start_handlers_ts"));
        assert!(!calc_is_feature_key_like("easing_src_features_game_start_transition_ts"));
    }

    #[test]
    fn extract_project_md_domain_names_reads_domain_blocks() {
        let md = r#"# Domains
### domain
- **name**: `player`
- **description**: d

### domain
- **name**: `system`
- **description**: d
"#;
        let domains = calc_extract_project_md_domain_names(md);
        assert_eq!(domains, vec!["player".to_string(), "system".to_string()]);
    }

    #[test]
    fn sync_project_md_overrides_placeholder_initialized_tasks_list() {
        let root = make_temp_dir("orc_sync_placeholder");
        let project_dir = root.join("project");
        let meta = project_dir.join(".project");
        fs::create_dir_all(&meta).expect("create project meta");
        let md = r#"# info
- name: sample

## features
1. task 생성 | src-tauri/src/commands/create_task.rs | 신규 task row 저장
2. task 삭제 | src-tauri/src/commands/delete_task.rs | task row 제거
"#;
        fs::write(meta.join("project.md"), md).expect("write project.md");
        let placeholder = DraftsListDoc {
            planned: vec![
                "project_project_md".to_string(),
                "features_project_md_features".to_string(),
                "features_project_features_work_draft_yaml".to_string(),
                "draft_yaml".to_string(),
            ],
            planned_items: vec![
                PlannedItem {
                    name: "project_project_md".to_string(),
                    value: "프로젝트 정보 입력 | .project/project.md 생성 | 설계 기준 문서 확보".to_string(),
                },
                PlannedItem {
                    name: "features_project_md_features".to_string(),
                    value: "features 리스트 입력 | project.md features 항목 업데이트 | 구현 범위 확정"
                        .to_string(),
                },
                PlannedItem {
                    name: "features_project_features_work_draft_yaml".to_string(),
                    value: "features 항목 분석 | .project/features/work/기능이름/draft.yaml 생성 | 기능별 구현 명세 확보"
                        .to_string(),
                },
                PlannedItem {
                    name: "draft_yaml".to_string(),
                    value: "draft.yaml 읽기 | 각 기능 폴더 내 코드 파일 생성/수정 | 기능 구현 완료"
                        .to_string(),
                },
            ],
            sync_initialized: true,
            ..Default::default()
        };
        action_save_drafts_list(&meta.join("drafts_list.yaml"), &placeholder)
            .expect("write placeholder drafts_list");

        let changed =
            action_sync_project_tasks_list_from_project_md(&project_dir).expect("sync tasks_list");
        assert!(changed);

        let doc = action_load_drafts_list(&meta.join("drafts_list.yaml")).expect("load drafts_list");
        assert!(!doc.planned.iter().any(|v| v == "project_project_md"));
        assert_eq!(doc.planned.len(), 2);
        assert_eq!(doc.planned_items.len(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sync_project_md_overrides_stale_template_like_tasks_list() {
        let root = make_temp_dir("orc_sync_stale_template");
        let project_dir = root.join("project");
        let meta = project_dir.join(".project");
        fs::create_dir_all(&meta).expect("create project meta");
        let md = r#"# info
- name: sample

## features
1. click-cube-jump | src/domains/player/jump.ts | jump
2. count-100-win | src/domains/system/win-condition.ts | win

## plan
1. cube 클릭 점프 액션을 구현한다.
2. 100회 점프 승리 조건을 구현한다.
"#;
        fs::write(meta.join("project.md"), md).expect("write project.md");
        let stale = DraftsListDoc {
            planned: vec![
                "func_e6740374".to_string(),
                "features".to_string(),
                "draft_yaml".to_string(),
            ],
            planned_items: vec![
                PlannedItem {
                    name: "func_e6740374".to_string(),
                    value: "프로젝트 정보 입력".to_string(),
                },
                PlannedItem {
                    name: "features".to_string(),
                    value: "features 리스트 입력".to_string(),
                },
                PlannedItem {
                    name: "draft_yaml".to_string(),
                    value: "draft.yaml 읽기".to_string(),
                },
            ],
            sync_initialized: true,
            ..Default::default()
        };
        action_save_drafts_list(&meta.join("drafts_list.yaml"), &stale)
            .expect("write stale drafts_list");

        let changed =
            action_sync_project_tasks_list_from_project_md(&project_dir).expect("sync tasks_list");
        assert!(changed);
        let doc = action_load_drafts_list(&meta.join("drafts_list.yaml")).expect("load drafts_list");
        assert!(!doc.planned.iter().any(|v| v == "func_e6740374"));
        assert!(!doc.planned.iter().any(|v| v == "features"));
        assert!(!doc.planned.iter().any(|v| v == "draft_yaml"));
        assert!(!doc.features.is_empty());
        assert!(!doc.planned.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sync_draft_state_doc_tracks_generated_and_pending() {
        let root = make_temp_dir("orc_draft_state");
        let project_dir = root.join("project");
        let meta = project_dir.join(".project");
        fs::create_dir_all(meta.join("feature").join("alpha_feature"))
            .expect("create generated feature dir");
        fs::write(
            meta.join("feature").join("alpha_feature").join("draft.yaml"),
            "task:\n- name: alpha\n",
        )
        .expect("write generated task");

        let mut doc = DraftsListDoc {
            planned: vec!["alpha_feature".to_string(), "beta_feature".to_string()],
            ..Default::default()
        };
        action_sync_draft_state_doc(&project_dir, &mut doc);
        assert_eq!(doc.draft_state.generated, vec!["alpha_feature".to_string()]);
        assert_eq!(doc.draft_state.pending, vec!["beta_feature".to_string()]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn add_feature_to_planned_doc_keeps_items_in_sync() {
        let mut doc = DraftsListDoc::default();
        let changed = action_add_feature_to_planned_doc(&mut doc, "new_feature");
        assert!(changed);
        assert_eq!(doc.planned, vec!["new_feature".to_string()]);
        assert_eq!(doc.planned_items.len(), 1);
        assert_eq!(doc.planned_items[0].name, "new_feature");
        assert_eq!(doc.planned_items[0].value, "new_feature");
    }

    #[test]
    fn promote_planned_to_features_doc_removes_planned_items() {
        let mut doc = DraftsListDoc {
            features: vec![],
            planned: vec!["alpha_feature".to_string(), "beta_feature".to_string()],
            planned_items: vec![
                PlannedItem {
                    name: "alpha_feature".to_string(),
                    value: "alpha feature value".to_string(),
                },
                PlannedItem {
                    name: "beta_feature".to_string(),
                    value: "beta feature value".to_string(),
                },
            ],
            ..Default::default()
        };

        let changed = action_promote_planned_to_features_doc(&mut doc, &["alpha_feature".to_string()]);
        assert!(changed);
        assert!(doc.features.iter().any(|v| v == "alpha_feature"));
        assert!(!doc.planned.iter().any(|v| v == "alpha_feature"));
        assert!(!doc.planned_items.iter().any(|v| v.name == "alpha_feature"));
        assert!(doc.planned_items.iter().any(|v| v.name == "beta_feature"));
    }

    #[test]
    fn promote_project_md_plan_to_features_moves_completed_items() {
        let root = make_temp_dir("orc_promote_project_md");
        let project_dir = root.join("project");
        let meta = project_dir.join(".project");
        fs::create_dir_all(&meta).expect("create project meta");
        let md = r#"# info
- name: sample

## plan
- alpha_feature
- beta_feature

## features
- existing_feature
"#;
        fs::write(meta.join("project.md"), md).expect("write project.md");

        let changed = action_promote_project_md_plan_to_features(
            &project_dir,
            &["alpha_feature".to_string()],
        )
        .expect("promote project.md");
        assert!(changed);

        let updated = fs::read_to_string(meta.join("project.md")).expect("read project.md");
        let plan = calc_extract_project_md_list_by_header(&updated, "## plan");
        let features = calc_extract_project_md_list_by_header(&updated, "## features");
        assert!(!plan.iter().any(|v| v == "alpha_feature"));
        assert!(plan.iter().any(|v| v == "beta_feature"));
        assert!(features.iter().any(|v| v == "existing_feature"));
        assert!(features.iter().any(|v| v == "alpha_feature"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_draft_create_rejects_invalid_planned_name() {
        let root = make_temp_dir("orc_preflight_draft");
        let tasks_path = root.join(".project").join("drafts_list.yaml");
        fs::create_dir_all(tasks_path.parent().expect("parent")).expect("create parent");
        let doc = DraftsListDoc {
            planned: vec!["문장형 기능".to_string()],
            ..Default::default()
        };
        action_save_drafts_list(&tasks_path, &doc).expect("save drafts_list");
        let err = action_preflight_draft_create(&tasks_path).expect_err("should fail");
        assert!(err.contains("invalid_name"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_parallel_build_rejects_missing_task_files() {
        let root = make_temp_dir("orc_preflight_parallel");
        let old_cwd = env::current_dir().expect("cwd");
        fs::create_dir_all(root.join(".project")).expect("create .project");
        let tasks_path = root.join(".project").join("drafts_list.yaml");
        let doc = DraftsListDoc {
            planned: vec!["sample_feature".to_string()],
            ..Default::default()
        };
        action_save_drafts_list(&tasks_path, &doc).expect("save drafts_list");
        env::set_current_dir(&root).expect("enter temp root");
        let err = action_preflight_parallel_build(Path::new(".project").join("drafts_list.yaml").as_path())
            .expect_err("should fail");
        assert!(err.contains("missing draft/task file"));
        env::set_current_dir(old_cwd).expect("restore cwd");
        let _ = fs::remove_dir_all(root);
    }
}
