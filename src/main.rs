mod config;
mod cli;
mod tmux;
mod ui;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Semaphore;

const REGISTRY_PATH: &str = "configs/project.yaml";
const LEGACY_REGISTRY_PATH: &str = "configs/Project.yaml";
const EXEC_LOG_PATH: &str = ".project/log.md";
const PROJECT_MD_PATH: &str = ".project/project.md";
const CODEX_DANGEROUS_FLAG: &str = "--dangerously-bypass-approvals-and-sandbox";
const DEFAULT_PROJECT_MD: &str = "# info
- name: bootstrap
- description: initialized by run parallel
- spec: unknown
- goal: initialize workspace

## rule
- keep changes minimal

## features
- bootstrap | .project/* | initialize project metadata
";

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DraftTask {
    name: String,
    #[serde(default)]
    depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DraftDoc {
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    task: Vec<DraftTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DraftsListDoc {
    #[serde(default)]
    domains: Vec<String>,
    #[serde(default)]
    feature: Vec<String>,
    #[serde(default)]
    planned: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParallelFeatureTask {
    name: String,
    draft_path: PathBuf,
    depends_on: Vec<String>,
}

fn calc_now_unix() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
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

fn calc_shell_single_quote_escape(input: &str) -> String {
    input.replace('\'', "'\"'\"'")
}

fn calc_model_supports_dangerous_flag(model_bin: &str) -> bool {
    model_bin.eq_ignore_ascii_case("codex")
}

fn action_default_model_bin() -> String {
    action_load_app_config()
        .and_then(|c| c.ai.as_ref().and_then(|a| a.model.as_ref()).cloned())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "codex".to_string())
}

fn action_send_codex_to_new_tmux_pane(prompt: &str) -> Result<String, String> {
    let pane_id = tmux::action_split_window_pane()?;
    let model_bin = action_default_model_bin();
    let command_line = if calc_model_supports_dangerous_flag(&model_bin) {
        format!(
            "{} exec {} '{}'",
            model_bin,
            CODEX_DANGEROUS_FLAG,
            calc_shell_single_quote_escape(prompt)
        )
    } else {
        format!("{} exec '{}'", model_bin, calc_shell_single_quote_escape(prompt))
    };
    tmux::action_send_keys(&pane_id, &command_line, tmux::SendOption::Enter)?;
    Ok(pane_id)
}

fn flow_tsend(pane_id: &str, msg: &str, option: &str) -> Result<String, String> {
    let send_option = match option {
        "raw" => tmux::SendOption::Raw,
        "enter" => tmux::SendOption::Enter,
        _ => return Err("tsend option must be `enter` or `raw`".to_string()),
    };
    tmux::action_send_keys(pane_id, msg, send_option)?;
    Ok(format!(
        "tsend done: pane={} option={} msg={}",
        pane_id, option, msg
    ))
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

fn action_run_codex_exec_capture(prompt: &str) -> Result<String, String> {
    let model_bin = action_default_model_bin();
    let mut command = Command::new(&model_bin);
    command.arg("exec");
    if calc_model_supports_dangerous_flag(&model_bin) {
        command.arg(CODEX_DANGEROUS_FLAG);
    }
    let output = command
        .arg(prompt)
        .output()
        .map_err(|e| format!("failed to execute {}: {}", model_bin, e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn action_run_codex_exec_capture_in_dir(dir: &Path, prompt: &str) -> Result<String, String> {
    let model_bin = action_default_model_bin();
    let mut command = Command::new(&model_bin);
    command.current_dir(dir).arg("exec");
    if calc_model_supports_dangerous_flag(&model_bin) {
        command.arg(CODEX_DANGEROUS_FLAG);
    }
    let output = command
        .arg(prompt)
        .output()
        .map_err(|e| format!("failed to execute {} in {}: {}", model_bin, dir.display(), e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn action_run_llm_exec_capture(llm: &str, prompt: &str) -> Result<String, String> {
    let output = Command::new(llm)
        .arg("exec")
        .arg("-y")
        .arg(prompt)
        .output()
        .map_err(|e| format!("failed to execute {}: {}", llm, e))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.contains("unexpected argument '-y'") {
        let retry = Command::new(llm)
            .arg("exec")
            .arg(prompt)
            .output()
            .map_err(|e| format!("failed to execute {} retry: {}", llm, e))?;
        if retry.status.success() {
            return Ok(String::from_utf8_lossy(&retry.stdout).to_string());
        }
        return Err(String::from_utf8_lossy(&retry.stderr).trim().to_string());
    }

    Err(stderr)
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

fn action_append_failure_log(task_name: &str, reason: &str) -> Result<(), String> {
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

fn calc_default_project_path(name: &str) -> PathBuf {
    Path::new(".").join(name)
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

fn flow_list_projects() -> Result<String, String> {
    let registry = action_load_registry(&action_registry_path())?;
    Ok(ui::render_project_list(&registry.projects))
}

fn flow_ui() -> Result<String, String> {
    let registry_path = action_registry_path();
    let mut registry = action_load_registry(&registry_path)?;
    let normalized = action_normalize_registry(&mut registry);
    let result = ui::flow_run_ui(&mut registry.projects, &mut registry.recent_active_pane)?;
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
        return flow_auto_mode(Some(&project_name));
    }
    Ok(result.message)
}

fn action_collect_project_features(project_path: &Path) -> Result<Vec<String>, String> {
    let drafts_list_path = project_path.join(".project").join("drafts_list.yaml");
    let doc = action_load_drafts_list(&drafts_list_path)?;
    let mut out = doc.feature;
    for planned in doc.planned {
        if !out.iter().any(|v| v == &planned) {
            out.push(planned);
        }
    }
    Ok(out)
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

fn flow_auto_mode(project_name: Option<&str>) -> Result<String, String> {
    let registry = action_load_registry(&action_registry_path())?;
    let target = if let Some(name) = project_name {
        registry.projects.iter().find(|p| p.name == name)
    } else {
        registry.projects.iter().find(|p| p.selected)
    }
    .ok_or_else(|| "auto mode requires a selected project".to_string())?;

    let pane_id = tmux::action_current_pane_id().map_err(|_| {
        "auto mode warning: tmux pane is not active. open tmux and retry.".to_string()
    })?;
    tmux::action_rename_pane(&pane_id, "plan")?;

    let project_root = PathBuf::from(&target.path);
    let project_md_path = project_root.join(".project").join("project.md");
    let project_info = fs::read_to_string(&project_md_path).unwrap_or_else(|_| {
        format!(
            "# info\n- name: {}\n- description: {}\n- path: {}",
            target.name, target.description, target.path
        )
    });
    let features = action_collect_project_features(&project_root)?;
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
Output a short action log at the end.\n\n\
Current project info:\n{}\n\nCurrent feature list:\n{}",
        target.name, project_info, features_text
    );
    let _ = action_run_codex_exec_capture_in_dir(&project_root, &auto_prompt)?;

    let _ = action_run_command_in_dir(&project_root, "cargo", &["test"], "cargo test")?;
    let _ = action_run_command_in_dir(
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

fn flow_create_project(name: &str, path: Option<&str>, description: &str) -> Result<String, String> {
    let target = path
        .map(PathBuf::from)
        .unwrap_or_else(|| calc_default_project_path(name));

    action_ensure_project_dir(&target)?;

    let existing = calc_is_existing_project(&target);
    if !existing {
        fs::create_dir_all(target.join(".project"))
            .map_err(|e| format!("failed to create .project: {}", e))?;
    }

    let registry_path = action_registry_path();
    let registry = action_load_registry(&registry_path)?;
    let upserted = action_upsert_project(&registry, name, &target, description);
    let selected = calc_select_only(&upserted, name);
    action_save_registry(&registry_path, &selected)?;

    if existing {
        Ok(format!("loaded existing project: {} ({})", name, target.display()))
    } else {
        Ok(format!("created project: {} ({})", name, target.display()))
    }
}

fn flow_add_project(name: &str, path: &str, description: &str) -> Result<String, String> {
    let registry_path = action_registry_path();
    let registry = action_load_registry(&registry_path)?;
    let updated = action_upsert_project(&registry, name, Path::new(path), description);
    action_save_registry(&registry_path, &updated)?;
    Ok(format!("added project: {}", name))
}

fn flow_select_project(name: &str) -> Result<String, String> {
    let registry_path = action_registry_path();
    let registry = action_load_registry(&registry_path)?;
    let exists = registry.projects.iter().any(|p| p.name == name);
    if !exists {
        return Err(format!("project not found: {}", name));
    }
    let updated = calc_select_only(&registry, name);
    action_save_registry(&registry_path, &updated)?;
    Ok(format!("selected project: {}", name))
}

fn flow_delete_project(name: &str) -> Result<String, String> {
    let registry_path = action_registry_path();
    let registry = action_load_registry(&registry_path)?;
    let updated = action_delete_project(&registry, name);
    action_save_registry(&registry_path, &updated)?;
    Ok(format!("deleted project: {}", name))
}

fn flow_draft_create() -> Result<String, String> {
    let prompt = "plan-drafts-code 스킬을 사용해서 `.project/project.md`의 `## features` 전체를 기준으로 `.project/drafts_list.yaml`과 `.project/feature/*/draft.yaml`을 생성/갱신해줘.".to_string();
    let pane_id = action_send_codex_to_new_tmux_pane(&prompt)?;
    Ok(format!(
        "draft-create sent to tmux pane {} | source=.project/project.md#features",
        pane_id
    ))
}

fn flow_draft_add(feature_name: &str, request: Option<String>) -> Result<String, String> {
    let request_text = match request {
        Some(v) if !v.trim().is_empty() => v,
        _ => action_read_one_line("draft 추가 요구사항을 입력하세요: ")?,
    };
    if request_text.trim().is_empty() {
        return Err("draft-add requires non-empty request".to_string());
    }
    let prompt = format!(
        "plan-drafts-code 결과를 바탕으로 `{}` feature draft에 신규 task를 질의응답 방식으로 1개 추가해줘. 요구사항: {}",
        feature_name, request_text
    );
    let pane_id = action_send_codex_to_new_tmux_pane(&prompt)?;
    let patch = format!("# qa-request: {}\n", request_text);
    let path = ui::action_apply_draft_create_update_delete(
        ui::DraftCommand::Add,
        feature_name,
        Some(&patch),
    )?;
    flow_add_feature_to_planned(feature_name)?;
    Ok(format!(
        "draft-add sent to tmux pane {} | updated: {}",
        pane_id,
        path.display()
    ))
}

fn flow_draft_delete(feature_name: &str) -> Result<String, String> {
    let answer = action_read_one_line(&format!(
        "delete `.project/feature/{}/draft.yaml` ? [y/N]: ",
        feature_name
    ))?;
    let accepted = matches!(answer.to_ascii_lowercase().as_str(), "y" | "yes");
    if !accepted {
        return Ok("draft-delete canceled".to_string());
    }
    let path =
        ui::action_apply_draft_create_update_delete(ui::DraftCommand::Delete, feature_name, None)?;
    Ok(format!("draft deleted: {}", path.display()))
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
        .position(|line| line.trim().eq_ignore_ascii_case("## features"));
    let idx = if let Some(i) = header_idx {
        i
    } else {
        lines.push(String::new());
        lines.push("## features".to_string());
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

fn flow_add_func(request_input: Option<String>) -> Result<String, String> {
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
    for obj in &objects {
        let draft_prompt = format!(
            "너는 rust-orc 프로젝트의 draft 작성기다.\nproject info:\n{}\n\nproject rules:\n- {}\n\n입력 객체:\n- name: {}\n- step:\n{}\n- rule:\n{}\n\n다음 형식으로만 출력해:\nFEATURE_NAME: <camelCase>\n```yaml\nrule:\nfeatures:\n  domain: []\ntask:\n  - name: <snake_case>\n    type: action\n    domain: [util]\n    depends_on: []\n    scope:\n      - src/main.rs\n    rule:\n      - \"\"\n    step:\n      - \"\"\n```\n설명 문장 추가 금지.",
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
            },
        );
        let draft_raw = action_run_codex_exec_capture(&draft_prompt)?;
        let feature_name = calc_extract_feature_name(&draft_raw, &obj.name);
        let draft_yaml = calc_extract_yaml_block(&draft_raw);
        let _: serde_yaml::Value = serde_yaml::from_str(&draft_yaml)
            .map_err(|e| format!("generated draft yaml invalid: {}", e))?;

        let draft_path = ui::action_resolve_feature_draft_path(&feature_name);
        if let Some(parent) = draft_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
        }
        fs::write(&draft_path, draft_yaml)
            .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
        action_append_feature_to_project_md(&feature_name, &obj.name)?;
        flow_add_feature_to_planned(&feature_name)?;
        created.push(format!("{} -> {}", feature_name, draft_path.display()));
    }

    Ok(format!(
        "add-func completed: {} item(s)\n{}",
        created.len(),
        created.join("\n")
    ))
}

fn calc_default_project_name_from_cwd() -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let name = cwd
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "failed to infer project name from current directory".to_string())?;
    Ok(name)
}

fn flow_plan_project(llm: Option<&str>) -> Result<String, String> {
    flow_plan_init(None, None, None, llm)
}

fn flow_plan_init(
    name: Option<&str>,
    description: Option<&str>,
    spec: Option<&str>,
    llm: Option<&str>,
) -> Result<String, String> {
    let llm_bin_owned = llm
        .map(|v| v.to_string())
        .unwrap_or_else(action_default_model_bin);
    let llm_bin = llm_bin_owned.as_str();
    let default_name = calc_default_project_name_from_cwd()?;
    let project_name = match name {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            let name_input = action_read_one_line(&format!(
                "project name (기본값: {}): ",
                default_name
            ))?;
            if name_input.trim().is_empty() {
                default_name
            } else {
                name_input.trim().to_string()
            }
        }
    };
    let description = match description {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => action_read_multiline_until_blank("description:")?,
    };
    let spec = match spec {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => action_read_multiline_until_blank("spec(기술스택/환경):")?,
    };
    let goal = action_read_multiline_until_blank("goal:")?;
    let rule_raw = action_read_multiline_until_blank("rule(여러 개는 ; 로 구분):")?;
    let user_rules: Vec<String> = rule_raw
        .split([';', '\n'])
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect();
    let rules_text = if user_rules.is_empty() {
        "- (작성 필요)".to_string()
    } else {
        user_rules
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = format!(
        "너는 project.md 생성기다.\n다음 사용자 입력을 기반으로 `.project/project.md` 전체 초안을 작성해.\n\n입력:\n- name: {}\n- description: {}\n- spec: {}\n- goal: {}\n- rules:\n{}\n\n제약:\n1) 출력은 markdown 본문만, 코드펜스/설명문 금지\n2) 반드시 아래 섹션 순서 유지:\n# info\n## rule\n## features\n## structure\n# Domains\n# Flow\n# Step\n# Constraints\n# Verification\n# Gate Checklist\n3) `# info`에는 name/description/spec/goal을 bullet로 채운다.\n4) `## rule`은 입력 rule을 반영한다.\n5) `## features`는 우선 3개 내외의 초기 항목으로 채운다.\n6) 나머지 섹션은 빈값 대신 최소 1개 이상의 초안 bullet/문장으로 채운다.",
        project_name, description, spec, goal, rules_text
    );
    let generated = action_run_llm_exec_capture(llm_bin, &prompt)?;
    let project_md = calc_extract_markdown_block(&generated);

    if let Some(parent) = Path::new(PROJECT_MD_PATH).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    fs::write(PROJECT_MD_PATH, &project_md)
        .map_err(|e| format!("failed to write {}: {}", PROJECT_MD_PATH, e))?;
    Ok(format!(
        "plan-init completed with {} -> {}",
        llm_bin, PROJECT_MD_PATH
    ))
}

fn flow_detail_project(llm: Option<&str>) -> Result<String, String> {
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
    fs::write(PROJECT_MD_PATH, &project_md)
        .map_err(|e| format!("failed to write {}: {}", PROJECT_MD_PATH, e))?;
    Ok(format!(
        "detail-project completed with {} -> {}",
        llm_bin, PROJECT_MD_PATH
    ))
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

fn flow_validate_tasks(feature_name: &str) -> Result<String, String> {
    let tasks = action_parse_draft_tasks(feature_name)?;
    let (runnable, blocked) = calc_validate_task_dependencies(&tasks);
    Ok(ui::render_task_validation(&runnable, &blocked))
}

fn action_load_app_config() -> Option<config::AppConfig> {
    let root = action_source_root();
    let candidates = [
        root.join("configs.yaml"),
        root.join("config.yaml"),
        root.join("assets").join("config").join("config.yaml"),
        root.join("src").join("assets").join("config").join("config.yaml"),
        PathBuf::from("configs.yaml"),
        PathBuf::from("config.yaml"),
        PathBuf::from("assets").join("config").join("config.yaml"),
        PathBuf::from("src").join("assets").join("config").join("config.yaml"),
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

fn calc_feature_name_camel_like(input: &str) -> String {
    let mut out = String::new();
    let mut cap_next = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if out.is_empty() {
                out.push(ch.to_ascii_lowercase());
            } else if cap_next {
                out.push(ch.to_ascii_uppercase());
                cap_next = false;
            } else {
                out.push(ch.to_ascii_lowercase());
            }
        } else {
            cap_next = true;
        }
    }
    if out.is_empty() {
        "newFeature".to_string()
    } else {
        out
    }
}

fn calc_extract_feature_name(raw: &str, fallback: &str) -> String {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("FEATURE_NAME:") {
            let candidate = calc_feature_name_camel_like(rest.trim());
            if !candidate.is_empty() {
                return candidate;
            }
        }
    }
    calc_feature_name_camel_like(fallback)
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

fn flow_add_feature_to_planned(feature_name: &str) -> Result<(), String> {
    let path = Path::new(".project").join("drafts_list.yaml");
    let mut doc = action_load_drafts_list(&path)?;
    if !doc.feature.iter().any(|v| v == feature_name) && !doc.planned.iter().any(|v| v == feature_name) {
        doc.planned.push(feature_name.to_string());
    }
    action_save_drafts_list(&path, &doc)
}

fn action_read_project_info() -> Result<String, String> {
    let project_md = fs::read_to_string(PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", PROJECT_MD_PATH, e))?;
    Ok(calc_extract_project_info(&project_md))
}

fn action_source_root() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.to_path_buf();
        }
    }
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
        root.join("assets").join("prompt").join("detail-project.txt"),
        root.join("assets").join("prompts").join("detail-project.txt"),
        PathBuf::from("assets").join("prompt").join("detail-project.txt"),
        PathBuf::from("assets").join("prompts").join("detail-project.txt"),
        root.join("src").join("assets").join("prompt").join("detail-project.txt"),
        root.join("src").join("assets").join("prompts").join("detail-project.txt"),
        PathBuf::from("src").join("assets").join("prompt").join("detail-project.txt"),
        PathBuf::from("src").join("assets").join("prompts").join("detail-project.txt"),
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

fn action_resolve_task_template_path() -> Result<PathBuf, String> {
    let root = action_source_root();
    let candidates = [
        root.join("assets").join("templates").join("prompts").join("tasks.txt"),
        PathBuf::from("assets").join("templates").join("prompts").join("tasks.txt"),
        root.join("src").join("assets").join("templates").join("prompts").join("tasks.txt"),
        PathBuf::from("src").join("assets").join("templates").join("prompts").join("tasks.txt"),
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

fn action_is_directory_empty(path: &Path) -> Result<bool, String> {
    let mut entries =
        fs::read_dir(path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    Ok(entries.next().is_none())
}

fn action_initialize_parallel_workspace_if_empty(path: &Path) -> Result<Option<String>, String> {
    if !action_is_directory_empty(path)? {
        return Ok(None);
    }

    let project_dir = path.join(".project");
    fs::create_dir_all(project_dir.join("feature"))
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;
    fs::create_dir_all(project_dir.join("clear"))
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;

    let template = action_resolve_project_template_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .unwrap_or_else(|| DEFAULT_PROJECT_MD.to_string());
    fs::write(project_dir.join("project.md"), template).map_err(|e| {
        format!(
            "failed to write {}: {}",
            project_dir.join("project.md").display(),
            e
        )
    })?;

    let drafts_list_path = project_dir.join("drafts_list.yaml");
    let draft_template = action_resolve_drafts_list_template_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .unwrap_or_else(|| serde_yaml::to_string(&DraftsListDoc::default()).unwrap_or_default());
    fs::write(&drafts_list_path, draft_template)
        .map_err(|e| format!("failed to write {}: {}", drafts_list_path.display(), e))?;

    Ok(Some(format!(
        "workspace was empty; initialized parallel environment at {}",
        project_dir.display()
    )))
}

fn action_collect_parallel_feature_tasks() -> Result<Vec<ParallelFeatureTask>, String> {
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
        let draft_candidates = [feature_dir.join("drafts.yaml"), feature_dir.join("draft.yaml")];
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
        out.push(ParallelFeatureTask {
            name,
            draft_path,
            depends_on: doc.depends_on,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn action_build_task_prompt(
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
    Ok(rendered)
}

fn calc_update_task_status(
    statuses: &[(String, ui::TaskRuntimeState)],
    target: &str,
    state: ui::TaskRuntimeState,
) -> Vec<(String, ui::TaskRuntimeState)> {
    statuses
        .iter()
        .map(|(name, old)| {
            if name == target {
                (name.clone(), state)
            } else {
                (name.clone(), *old)
            }
        })
        .collect()
}

fn action_print_parallel_modal(statuses: &[(String, ui::TaskRuntimeState)]) {
    println!("{}", ui::render_parallel_modal(statuses));
}

async fn action_run_one_parallel_task(
    semaphore: Arc<Semaphore>,
    model_bin: String,
    task_name: String,
    prompt: String,
    timeout_sec: u64,
    _auto_yes: bool,
    dangerous_bypass: bool,
) -> Result<String, String> {
    let _permit = semaphore
        .acquire_owned()
        .await
        .map_err(|e| format!("failed to acquire semaphore: {}", e))?;
    let mut cmd = tokio::process::Command::new(&model_bin);
    cmd.arg("exec");
    if dangerous_bypass && calc_model_supports_dangerous_flag(&model_bin) {
        cmd.arg("--dangerously-bypass-approvals-and-sandbox");
    }
    cmd.arg(prompt);
    let run_fut = cmd.status();
    let status = tokio::time::timeout(Duration::from_secs(timeout_sec), run_fut)
        .await
        .map_err(|_| format!("timeout ({timeout_sec}s) for {task_name}"))?
        .map_err(|e| format!("failed to run command for {task_name}: {}", e))?;
    if status.success() {
        Ok(task_name)
    } else {
        Err(format!(
            "{} failed with exit code {:?}",
            task_name,
            status.code()
        ))
    }
}

async fn flow_run_parallel_build_code() -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    if let Some(init_msg) = action_initialize_parallel_workspace_if_empty(&cwd)? {
        println!("{}", init_msg);
    }

    let app_conf = action_load_app_config();
    let max_parallel = app_conf.as_ref().map_or(10, config::AppConfig::default_max_parallel);
    let timeout_sec = app_conf.as_ref().map_or(1800, config::AppConfig::default_timeout_sec);
    let auto_yes = app_conf.as_ref().is_none_or(config::AppConfig::auto_yes_enabled);
    let dangerous_bypass = app_conf
        .as_ref()
        .is_none_or(config::AppConfig::dangerous_bypass_enabled);
    let model_bin = action_default_model_bin();

    let project_info = action_read_project_info()?;
    let task_template_path = action_resolve_task_template_path()?;
    let task_template = fs::read_to_string(&task_template_path)
        .map_err(|e| format!("failed to read {}: {}", task_template_path.display(), e))?;
    let mut pending = action_collect_parallel_feature_tasks()?;
    if pending.is_empty() {
        return Ok("no feature draft to run".to_string());
    }

    let mut statuses: Vec<(String, ui::TaskRuntimeState)> = pending
        .iter()
        .map(|t| (t.name.clone(), ui::TaskRuntimeState::Inactive))
        .collect();
    action_print_parallel_modal(&statuses);

    let semaphore = Arc::new(Semaphore::new(max_parallel));
    let mut finished: HashSet<String> = HashSet::new();
    let mut success = 0usize;
    let mut failed = 0usize;

    loop {
        if pending.is_empty() {
            break;
        }
        let runnable_names: HashSet<String> = pending
            .iter()
            .filter(|task| task.depends_on.iter().all(|dep| finished.contains(dep)))
            .map(|task| task.name.clone())
            .collect();

        if runnable_names.is_empty() {
            for task in pending {
                failed += 1;
                let reason = format!("blocked by unresolved depends_on: {:?}", task.depends_on);
                let _ = action_append_failure_log(&task.name, &reason);
            }
            break;
        }

        let mut round = Vec::new();
        let mut remain = Vec::new();
        for task in pending {
            if runnable_names.contains(&task.name) {
                round.push(task);
            } else {
                remain.push(task);
            }
        }
        pending = remain;

        let mut handles = Vec::new();
        for task in round {
            statuses = calc_update_task_status(&statuses, &task.name, ui::TaskRuntimeState::Active);
            action_print_parallel_modal(&statuses);
            let prompt = action_build_task_prompt(&task_template, &project_info, &task.draft_path)?;
            handles.push(tokio::spawn(action_run_one_parallel_task(
                semaphore.clone(),
                model_bin.clone(),
                task.name.clone(),
                prompt,
                timeout_sec,
                auto_yes,
                dangerous_bypass,
            )));
        }

        for handle in handles {
            match handle.await {
                Ok(Ok(name)) => {
                    success += 1;
                    finished.insert(name.clone());
                    statuses = calc_update_task_status(&statuses, &name, ui::TaskRuntimeState::Clear);
                    action_print_parallel_modal(&statuses);
                }
                Ok(Err(reason)) => {
                    failed += 1;
                    let task_name = reason.split_whitespace().next().unwrap_or("parallel_task");
                    let _ = action_append_failure_log(task_name, &reason);
                }
                Err(join_err) => {
                    failed += 1;
                    let _ = action_append_failure_log("parallel_task", &join_err.to_string());
                }
            }
        }
    }
    Ok(format!(
        "run_parallel_build_code finished: success={}, failed={}",
        success, failed
    ))
}

async fn flow_press_key(key: &str) -> Result<String, String> {
    let config = action_load_app_config();
    let run_parallel_key = config
        .as_ref()
        .map_or("p", config::AppConfig::run_parallel_key);
    if key == run_parallel_key {
        flow_run_parallel_build_code().await
    } else {
        Err(format!("unmapped key: {} (run_parallel key: {})", key, run_parallel_key))
    }
}

#[tokio::main]
async fn main() {
    let _ = action_load_app_config();
    let args: Vec<String> = env::args().collect();
    let program = cli::calc_program_name(&args);
    if cli::calc_is_help_command(&args) {
        cli::flow_print_usage(program);
        return;
    }

    match cli::flow_execute_cli(&args).await {
        Ok(output) => println!("{}", output),
        Err(err) => {
            eprintln!("{}", err);
            cli::flow_print_usage(program);
            std::process::exit(1);
        }
    }
}
