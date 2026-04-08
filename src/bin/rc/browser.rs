use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadMode {
    On,
    Off,
}

impl HeadMode {
    pub fn as_agent_browser_flag(self) -> &'static str {
        match self {
            Self::On => " --headed",
            Self::Off => "",
        }
    }
}

pub fn install_command(agent_browser_command: &str) -> String {
    format!(
        "if command -v {agent_browser_command} >/dev/null 2>&1; then {agent_browser_command} install; \
         else npm install -g agent-browser && {agent_browser_command} install; fi"
    )
}

pub fn open_command(agent_browser_command: &str, url: &str, head_mode: HeadMode) -> String {
    format!(
        "{agent_browser_command} open {url}{}",
        head_mode.as_agent_browser_flag()
    )
}

pub fn wait_for_url_command(url: &str) -> String {
    let url_escaped = url.replace('\\', "\\\\").replace('"', "\\\"");
    let script = [
        "import sys",
        "import time",
        "import urllib.request",
        "url = sys.argv[1]",
        "last = None",
        "for _ in range(30):",
        "    try:",
        "        with urllib.request.urlopen(url, timeout=2) as response:",
        "            if response.status < 500:",
        "                raise SystemExit(0)",
        "    except Exception as exc:",
        "        last = exc",
        "        time.sleep(1)",
        "print(\"server not ready: \" + url + \": \" + str(last), file=sys.stderr)",
        "raise SystemExit(1)",
    ]
    .join("\\n")
    .replace('\\', "\\\\")
    .replace('"', "\\\"");
    format!(
        "python3 -c \"{script}\" \"{url}\"",
        script = script,
        url = url_escaped
    )
}

pub fn click_command(agent_browser_command: &str, label: &str) -> String {
    format!("{agent_browser_command} find role button click --name \"{label}\"")
}

pub fn click_selector_command(agent_browser_command: &str, selector: &str) -> String {
    format!("{agent_browser_command} click \"{selector}\"")
}

pub fn fill_command(agent_browser_command: &str, selector: &str, value: &str) -> String {
    format!("{agent_browser_command} fill \"{selector}\" \"{value}\"")
}

pub fn keyboard_type_command(agent_browser_command: &str, value: &str) -> String {
    format!("{agent_browser_command} keyboard type \"{value}\"")
}

pub fn wait_for_selector_command(agent_browser_command: &str, selector: &str) -> String {
    format!("{agent_browser_command} wait \"{selector}\"")
}

pub fn sleep_command(seconds: u32) -> String {
    format!("sleep {seconds}")
}

pub fn snapshot_command(agent_browser_command: &str) -> String {
    format!("{agent_browser_command} snapshot -i")
}

pub fn screenshot_and_close_command(agent_browser_command: &str, output_path: &Path) -> String {
    let output = output_path.display().to_string().replace('"', "\\\"");
    format!(
        "{agent_browser_command} screenshot \"{output}\" && test -f \"{output}\" && printf 'Screenshot saved: {output}\\n' && {agent_browser_command} close"
    )
}

pub struct QaStage {
    pub command: Vec<OsString>,
    pub staging_dir: Option<PathBuf>,
}

pub struct QaEnvPaths {
    pub node_modules: PathBuf,
    pub bin_dir: PathBuf,
    pub helper_path: PathBuf,
}

pub struct FrontUiRuleCheck {
    pub program: String,
    pub args: Vec<String>,
}

pub fn prepend_env_list(existing: Option<&std::ffi::OsStr>, addition: &Path) -> OsString {
    let mut parts = vec![addition.as_os_str().to_os_string()];
    if let Some(existing) = existing {
        parts.extend(std::env::split_paths(existing).map(|path| path.into_os_string()));
    }
    std::env::join_paths(parts).unwrap_or_else(|_| OsString::from(addition.as_os_str()))
}

pub fn resolve_node_entry_script(command: &[OsString]) -> (Option<usize>, Option<PathBuf>) {
    let Some(program) = command.first() else {
        return (None, None);
    };
    let program_name = Path::new(program)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !matches!(program_name, "node" | "nodejs") {
        return (None, None);
    }
    for (index, arg) in command.iter().enumerate().skip(1) {
        if arg == "--" {
            break;
        }
        let Some(value) = arg.to_str() else {
            return (None, None);
        };
        if value.starts_with('-') {
            continue;
        }
        let candidate = PathBuf::from(value);
        if !candidate.exists() {
            return (None, None);
        }
        return (
            Some(index),
            Some(candidate.canonicalize().unwrap_or(candidate)),
        );
    }
    (None, None)
}

pub fn stage_node_entry_script(command: &[OsString], web_root: &Path) -> Result<QaStage> {
    let (entry_index, entry_path) = resolve_node_entry_script(command);
    let (Some(entry_index), Some(entry_path)) = (entry_index, entry_path) else {
        return Ok(QaStage {
            command: command.to_vec(),
            staging_dir: None,
        });
    };
    if entry_path.starts_with(web_root) {
        return Ok(QaStage {
            command: command.to_vec(),
            staging_dir: None,
        });
    }

    let staging_dir = web_root.join(format!(
        ".orc-qa-node-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&staging_dir)
        .with_context(|| format!("failed to create {}", staging_dir.display()))?;
    let staged_script = staging_dir.join(
        entry_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("missing node entry file name"))?,
    );
    fs::copy(&entry_path, &staged_script).with_context(|| {
        format!(
            "failed to stage {} into {}",
            entry_path.display(),
            staged_script.display()
        )
    })?;

    let mut staged_command = command.to_vec();
    staged_command[entry_index] = staged_script.into_os_string();
    Ok(QaStage {
        command: staged_command,
        staging_dir: Some(staging_dir),
    })
}

pub fn prepare_qa_env_paths(web_root: &Path, helper_path: &Path) -> Result<QaEnvPaths> {
    let node_modules = web_root.join("node_modules");
    let package_json = web_root.join("package.json");
    if !package_json.exists() {
        bail!(
            "web workspace package.json not found: {}",
            package_json.display()
        );
    }
    if !node_modules.exists() {
        bail!(
            "web workspace node_modules not found: {}",
            node_modules.display()
        );
    }
    Ok(QaEnvPaths {
        node_modules: node_modules.clone(),
        bin_dir: node_modules.join(".bin"),
        helper_path: helper_path.to_path_buf(),
    })
}

pub fn build_front_ui_rule_check() -> FrontUiRuleCheck {
    FrontUiRuleCheck {
        program: "npm".to_string(),
        args: vec![
            "--prefix".to_string(),
            "assets/web".to_string(),
            "run".to_string(),
            "test:e2e:design-rules".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use tempfile::tempdir;

    #[test]
    fn headed_flag_is_added() {
        assert!(
            open_command("agent-browser", "http://127.0.0.1:3000", HeadMode::On)
                .contains("--headed")
        );
        assert!(
            !open_command("agent-browser", "http://127.0.0.1:3000", HeadMode::Off)
                .contains("--headed")
        );
        assert!(
            !open_command("agent-browser", "http://127.0.0.1:3000", HeadMode::On)
                .contains("snapshot -i")
        );
    }

    #[test]
    fn install_command_skips_system_dependency_flag() {
        let command = install_command("agent-browser");
        assert!(!command.contains("--with-deps"));
        assert!(command.contains("agent-browser install"));
    }

    #[test]
    fn helper_commands_use_supported_locators() {
        assert_eq!(
            click_selector_command(
                "agent-browser",
                ".auth-card:nth-of-type(2) button[type='submit']"
            ),
            "agent-browser click \".auth-card:nth-of-type(2) button[type='submit']\""
        );
        assert_eq!(
            fill_command(
                "agent-browser",
                ".auth-card:nth-of-type(1) input[type='password']",
                "demo-pass"
            ),
            "agent-browser fill \".auth-card:nth-of-type(1) input[type='password']\" \"demo-pass\""
        );
        assert_eq!(
            wait_for_selector_command(
                "agent-browser",
                ".auth-card:nth-of-type(2) input[placeholder='name']"
            ),
            "agent-browser wait \".auth-card:nth-of-type(2) input[placeholder='name']\""
        );
        assert_eq!(sleep_command(1), "sleep 1");
        assert_eq!(
            snapshot_command("agent-browser"),
            "agent-browser snapshot -i"
        );
        assert!(
            screenshot_and_close_command("agent-browser", Path::new("/tmp/rc-web.png"))
                .contains("/tmp/rc-web.png")
        );
        assert!(wait_for_url_command("http://127.0.0.1:3000").contains("python3 -c "));
    }

    #[test]
    fn stages_external_node_entry_into_web_root() {
        let web_root = tempdir().expect("web root");
        let src_dir = tempdir().expect("src dir");
        let source = src_dir.path().join("qa-check.mjs");
        fs::write(&source, "import 'playwright';\n").expect("write");

        let staged = stage_node_entry_script(
            &[OsString::from("node"), source.clone().into_os_string()],
            web_root.path(),
        )
        .expect("stage");

        let staged_path = PathBuf::from(&staged.command[1]);
        assert!(staged.staging_dir.is_some());
        assert!(staged_path.starts_with(web_root.path()));
        assert_eq!(
            fs::read_to_string(&staged_path).expect("staged"),
            fs::read_to_string(&source).expect("source")
        );
    }

    #[test]
    fn keeps_node_entry_inside_web_root_unchanged() {
        let web_root = tempdir().expect("web root");
        let source = web_root.path().join("qa-check.mjs");
        fs::write(&source, "console.log('ok');\n").expect("write");

        let staged = stage_node_entry_script(
            &[OsString::from("node"), source.clone().into_os_string()],
            web_root.path(),
        )
        .expect("stage");

        assert!(staged.staging_dir.is_none());
        assert_eq!(staged.command, vec![OsString::from("node"), source.into()]);
    }

    #[test]
    fn builds_front_ui_rule_check_command() {
        let command = build_front_ui_rule_check();
        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["--prefix", "assets/web", "run", "test:e2e:design-rules"]
        );
    }
}
