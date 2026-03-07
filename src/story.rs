use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MODE_LIST: [&str; 4] = ["project", "plan", "draft", "report"];

#[derive(Debug, Clone, Default)]
struct StoryCommonOpts {
    name: Option<String>,
    description: Option<String>,
    spec: Option<String>,
    path: Option<String>,
    message: Option<String>,
    auto: bool,
}

pub(crate) fn init_story_project(args: &[String]) -> Result<String, String> {
    let opts = parse_common_opts(args);
    if opts.auto && opts.message.is_none() {
        return Err("init_code_project -a requires message (`-a <msg>`)".to_string());
    }

    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let default_name = cwd
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("story-project")
        .to_string();
    let default_path = cwd
        .canonicalize()
        .unwrap_or(cwd.clone())
        .display()
        .to_string();

    let mut name = opts.name.unwrap_or(default_name);
    let mut description = opts
        .description
        .unwrap_or_else(|| "story 프로젝트 초기화".to_string());
    let mut spec = normalize_story_spec(opts.spec.as_deref().unwrap_or("단편"));
    let path = opts.path.unwrap_or(default_path);

    if let Some(msg) = opts.message {
        let (n, d, s) = infer_from_message(&msg);
        if !n.is_empty() {
            name = n;
        }
        if !d.is_empty() {
            description = d;
        }
        if !s.is_empty() {
            spec = normalize_story_spec(&s);
        }
    }

    enforce_project_dir()?;
    write_story_project_md(&name, &description, &path, &spec)?;
    ensure_story_memo_md()?;
    ensure_story_plan_yaml()?;
    ensure_story_drafts_yaml()?;
    ensure_story_draft_yaml()?;

    Ok(format!(
        "mode={:?} | init_story_project completed: .project/project.md/.project/plan.yaml/.project/drafts.yaml/.project/draft.yaml",
        MODE_LIST
    ))
}

pub(crate) fn init_story_plan(args: &[String]) -> Result<String, String> {
    let _auto = args.iter().any(|v| v == "-a");
    ensure_story_plan_yaml()?;
    Ok("init_story_plan completed".to_string())
}

pub(crate) fn add_story_plan(args: &[String]) -> Result<String, String> {
    crate::code::add_code_plan(args)
}

pub(crate) fn create_story_draft() -> Result<String, String> {
    ensure_story_drafts_yaml()?;
    ensure_story_draft_yaml()?;
    Ok("create_story_draft completed".to_string())
}

pub(crate) fn add_story_draft(args: &[String]) -> Result<String, String> {
    crate::code::add_code_draft(args)
}

pub(crate) async fn impl_story_draft() -> Result<String, String> {
    crate::code::impl_code_draft().await
}

pub(crate) fn check_story_draft(auto_yes: bool) -> Result<String, String> {
    crate::code::check_code_draft(auto_yes)
}

pub(crate) fn check_story_task() -> Result<String, String> {
    crate::code::check_task()
}

pub(crate) fn check_story_only() -> Result<String, String> {
    crate::code::check_draft()
}

pub(crate) fn create_story_input_md() -> Result<String, String> {
    crate::code::create_input_md()
}

pub(crate) fn auto_story_message(message: &str) -> Result<String, String> {
    init_story_project(&["-a".to_string(), message.to_string()])
}

pub(crate) fn auto_story_from_input_file() -> Result<String, String> {
    init_story_project(&[])
}

fn parse_common_opts(args: &[String]) -> StoryCommonOpts {
    let mut opts = StoryCommonOpts::default();
    let mut i = 0usize;
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
            "-a" => {
                opts.auto = true;
                i += 1;
                opts.message = args.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }
    opts
}

fn normalize_story_spec(raw: &str) -> String {
    let value = raw.trim();
    if matches!(value, "장편" | "단편" | "중") {
        value.to_string()
    } else {
        "단편".to_string()
    }
}

fn infer_from_message(message: &str) -> (String, String, String) {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new(), String::new());
    }

    let mut spec = "".to_string();
    if trimmed.contains("장편") {
        spec = "장편".to_string();
    } else if trimmed.contains("단편") {
        spec = "단편".to_string();
    } else if trimmed.contains("중") {
        spec = "중".to_string();
    }

    let name = extract_title_like(trimmed).unwrap_or_else(|| "story-project".to_string());
    (name, trimmed.to_string(), spec)
}

fn extract_title_like(message: &str) -> Option<String> {
    let mut out = String::new();
    for ch in message.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    let normalized = out
        .split('-')
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn read_story_template(file_name: &str) -> Result<String, String> {
    let path = crate::source_root()
        .join("assets")
        .join("story")
        .join("templates")
        .join(file_name);
    fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))
}

fn enforce_project_dir() -> Result<PathBuf, String> {
    let dir = Path::new(".project");
    fs::create_dir_all(dir).map_err(|e| format!("failed to create {}: {}", dir.display(), e))?;
    Ok(dir.to_path_buf())
}

fn write_story_project_md(name: &str, description: &str, path: &str, spec: &str) -> Result<(), String> {
    let mut body = read_story_template("project.md")?;
    body = replace_info_field_value(&body, "name", name);
    body = replace_info_field_value(&body, "description", description);
    body = replace_info_field_value(&body, "path", path);
    body = replace_info_field_value(&body, "spec", &normalize_story_spec(spec));
    write_file(Path::new(crate::PROJECT_MD_PATH), &format!("{}\n", body.trim_end()))
}

fn ensure_story_plan_yaml() -> Result<(), String> {
    let path = Path::new(".project").join("plan.yaml");
    if path.exists() {
        return Ok(());
    }
    let body = read_story_template("plan.yaml")?;
    write_file(&path, &body)
}

fn ensure_story_drafts_yaml() -> Result<(), String> {
    let path = Path::new(".project").join("drafts.yaml");
    if path.exists() {
        return Ok(());
    }
    let body = read_story_template("drafts.yaml")?;
    write_file(&path, &body)
}

fn ensure_story_draft_yaml() -> Result<(), String> {
    let path = Path::new(".project").join("draft.yaml");
    if path.exists() {
        return Ok(());
    }
    let body = read_story_template("draft.yaml")?;
    write_file(&path, &body)
}

fn ensure_story_memo_md() -> Result<(), String> {
    let path = Path::new(".project").join("memo.md");
    if path.exists() {
        return Ok(());
    }
    write_file(&path, "")
}

fn write_file(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    fs::write(path, body).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn replace_info_field_value(raw: &str, field: &str, value: &str) -> String {
    let mut out = Vec::new();
    let mut in_info = false;
    let mut replaced = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("# info") {
            in_info = true;
            out.push(line.to_string());
            continue;
        }
        if in_info && trimmed.starts_with('#') && !trimmed.eq_ignore_ascii_case("# info") {
            if !replaced {
                out.push(format!("{} : {}", field, value));
                replaced = true;
            }
            in_info = false;
        }

        if in_info {
            if let Some((lhs, _)) = line.split_once(':') {
                if lhs.trim().eq_ignore_ascii_case(field) {
                    out.push(format!("{} : {}", field, value));
                    replaced = true;
                    continue;
                }
            }
        }

        out.push(line.to_string());
    }

    if !replaced {
        out.push(format!("{} : {}", field, value));
    }

    out.join("\n")
}
